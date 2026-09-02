//! The WebSocket side: replay, then forward, and never understand.
//!
//! One connection speaks for one device. Its whole life is: a `Hello`
//! naming a group and a cursor, a `Welcome` carrying the log's epoch, the
//! replay, a `CaughtUp`, and then a steady state of pushes going in and
//! frames coming out (DECISIONS 0042).
//!
//! Two ordering facts carry the correctness, and both are cheap:
//!
//! - **Appends and their broadcast happen under the log lock**, so every
//!   subscriber's channel sees frames in sequence order — which is what
//!   lets a device advance its cursor on frames alone.
//! - **Subscribing and snapshotting the replay happen under that same
//!   lock**, so a concurrent push lands either in the replay or in the
//!   subscription, never in both and never in neither.
//!
//! Acks ride the same socket but carry no ordering promise against frames,
//! and the client does not need one: an ack moves the shadow version, only
//! frames move the cursor. The pusher receives its own frame back and
//! merges it into a no-op — skipping it would be an optimisation on a
//! payload the size of a shopping-list edit.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::{any, get};
use tokio::sync::{Mutex, RwLock, broadcast};

use cabas_sync::photo::{self, PHOTO_PROTOCOL, PhotoClientMessage, PhotoName, PhotoServerMessage};
use cabas_sync::protocol::{self, ClientMessage, PROTOCOL, ServerMessage};
use cabas_sync::{GroupId, SyncError};

use crate::log::GroupLog;
use crate::photos::GroupPhotos;

/// Everything the process holds: where the logs live, and which are open.
/// Groups load lazily on their first `Hello` and stay open — a group is
/// two people, and the map is as big as the number of households served.
pub struct Relay {
    root: PathBuf,
    groups: RwLock<HashMap<GroupId, Arc<Group>>>,
    /// How often a connection is pinged. A field rather than a bare constant
    /// so the test below can watch one arrive without sitting out
    /// `PING_EVERY`; nothing outside this module can set it.
    ping_every: Duration,
}

struct Group {
    log: Mutex<GroupLog>,
    /// Pre-encoded `ServerMessage::Frame`s, fanned out to every connection.
    /// `Bytes` so a frame is encoded once and cloned by reference count.
    forward: broadcast::Sender<Bytes>,
    /// The photos, beside the log and sharing nothing with it but the
    /// directory (DECISIONS 0080). A second lock rather than one over both:
    /// a photo transfer is slow and a tick in a shop is not, and a shared
    /// lock would put the second behind the first.
    photos: Mutex<GroupPhotos>,
}

/// Frames a slow connection may fall behind before it is disconnected and
/// made to reconnect — at which point the replay, not the channel, fills
/// the gap. Losing a channel message is therefore never losing data.
const FORWARD_BUFFER: usize = 256;

/// How often an otherwise silent connection is pinged (DECISIONS 0051).
///
/// A proxy closes a WebSocket that carries nothing for long enough, and
/// Cloudflare — which is what faces the internet here (0012) — says so in as
/// many words without publishing the number. A ping is the smallest frame
/// that resets that clock, and a browser answers it inside its own socket
/// implementation, so a live connection costs no protocol version (0042) and
/// no line of frontend.
///
/// Thirty seconds is well under every timeout anyone publishes, and the cost
/// is two bytes a minute on a socket that only exists while somebody has the
/// app on screen (0011).
const PING_EVERY: Duration = Duration::from_secs(30);

impl Relay {
    pub fn open(root: PathBuf) -> std::io::Result<Arc<Self>> {
        Self::open_every(root, PING_EVERY)
    }

    fn open_every(root: PathBuf, ping_every: Duration) -> std::io::Result<Arc<Self>> {
        std::fs::create_dir_all(&root)?;
        Ok(Arc::new(Relay {
            root,
            groups: RwLock::new(HashMap::new()),
            ping_every,
        }))
    }

    async fn group(&self, id: GroupId) -> std::io::Result<Arc<Group>> {
        if let Some(group) = self.groups.read().await.get(&id) {
            return Ok(group.clone());
        }
        let mut groups = self.groups.write().await;
        // Two devices of one group saying hello at once race to this
        // write lock; the loser must find the winner's log, not a second
        // one over the same directory.
        if let Some(group) = groups.get(&id) {
            return Ok(group.clone());
        }
        let dir = self.root.join(id.to_hex());
        let log = GroupLog::open(dir.clone())?;
        let photos = GroupPhotos::open(GroupPhotos::dir_of(&dir))?;
        let (forward, _) = broadcast::channel(FORWARD_BUFFER);
        let group = Arc::new(Group {
            log: Mutex::new(log),
            forward,
            photos: Mutex::new(photos),
        });
        groups.insert(id, group.clone());
        Ok(group)
    }
}

/// The three named routes, and the PWA under everything else.
///
/// Order matters only in that `/sync`, `/photos` and `/healthz` are named: the
/// fallback answers every other path out of the embedded bundle, so a file
/// called `sync` in `ui/dist` would be unreachable — which is a rule the
/// bundler cannot break, since it names its own outputs.
///
/// `/photos` is a second socket rather than a second message on the first,
/// and the reason is what the two carry (DECISIONS 0080): a photo is hundreds
/// of kilobytes and a list edit is a hundred bytes, so sharing a socket would
/// put a tick in a shop behind a picture of a jar. It has a protocol byte of
/// its own too, so a change to one never stops a phone speaking the other.
///
/// `get` rather than `any` for the assets: it covers HEAD, whose body axum
/// discards for us, and answers 405 to a POST at a static file instead of
/// serving it.
pub fn router(relay: Arc<Relay>) -> Router {
    Router::new()
        .route("/sync", any(ws_handler))
        .route("/photos", any(photo_handler))
        // For the M6 add-on's watchdog; says the process is up, nothing else.
        .route("/healthz", get(|| async { "ok" }))
        .fallback(get(crate::assets::handler))
        .with_state(relay)
}

async fn ws_handler(ws: WebSocketUpgrade, State(relay): State<Arc<Relay>>) -> Response {
    ws.on_upgrade(move |socket| connection(socket, relay))
}

/// How large a photo message may be before the socket refuses to assemble it
/// at all (DECISIONS 0080, 0092).
///
/// The policy the protocol deliberately leaves to the relay. Two things a
/// stranger who guessed a group id can make arbitrarily long — a hello's two
/// lists and a push's payload — and `GroupPhotos::store` rejecting an oversized
/// blob is one step too late: by then the bytes are in this process's memory.
/// Comfortably above [`crate::photos::MAX_PHOTO_BYTES`] plus its postcard
/// framing, and far below axum's own 64 MB default.
const MAX_PHOTO_MESSAGE: usize = 2 * 1024 * 1024;

async fn photo_handler(ws: WebSocketUpgrade, State(relay): State<Arc<Relay>>) -> Response {
    ws.max_message_size(MAX_PHOTO_MESSAGE)
        .on_upgrade(move |socket| photo_connection(socket, relay))
}

/// Runs one connection to completion. Exits on close, on error, and on the
/// one impoliteness the relay answers with words: a `Refused` names its
/// reason before the socket drops, because a silent close reads as a
/// network blip and invites a pointless retry.
async fn connection(mut socket: WebSocket, relay: Arc<Relay>) {
    let (group, hello_epoch, hello_since) = match expect_hello(&mut socket).await {
        Some(hello) => hello,
        None => return,
    };
    let group = match relay.group(group).await {
        Ok(group) => group,
        Err(e) => {
            tracing::error!(error = %e, "group log failed to open");
            refuse(&mut socket, "storage failed").await;
            return;
        }
    };

    // Subscribe and snapshot the replay under one lock: a push landing now
    // is either in `replay` or already in `rx`, never lost between them.
    let (epoch, replay, mut rx) = {
        let log = group.log.lock().await;
        // A cursor is honoured only if it names this log's epoch *and* points
        // inside it. The epoch alone is not enough: a log restored from a
        // backup brings its epoch back with it, while every device holds a
        // cursor from further along than the restored log ever reached
        // (DECISIONS 0053). Replaying from there is replaying nothing, for as
        // many pushes as the restore rolled back — silently, on both sides.
        let since = if hello_epoch == log.epoch() && hello_since < log.next_seq() {
            hello_since
        } else {
            // The cursor points into a log that no longer exists — replay
            // everything and let `Welcome` tell the device why.
            0
        };
        (log.epoch(), log.replay(since), group.forward.subscribe())
    };

    if send(&mut socket, &ServerMessage::Welcome { epoch })
        .await
        .is_err()
    {
        return;
    }
    for frame in replay {
        let message = ServerMessage::Frame {
            seq: frame.seq,
            kind: frame.kind,
            payload: frame.payload,
        };
        if send(&mut socket, &message).await.is_err() {
            return;
        }
    }
    if send(&mut socket, &ServerMessage::CaughtUp).await.is_err() {
        return;
    }

    // Ticks on a schedule rather than idling out from the last message: a
    // ping on a busy socket costs two bytes and a branch, and tracking
    // activity would be a second clock to keep honest for no gain. The first
    // tick is a period away, so a connection that says its piece and leaves
    // is never pinged at all.
    let mut ping = tokio::time::interval_at(
        tokio::time::Instant::now() + relay.ping_every,
        relay.ping_every,
    );
    ping.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            incoming = socket.recv() => {
                let bytes = match incoming {
                    Some(Ok(Message::Binary(bytes))) => bytes,
                    Some(Ok(Message::Close(_))) | None => return,
                    Some(Ok(_)) => continue, // text, ping, pong — not ours
                    Some(Err(_)) => return,
                };
                match protocol::decode_client(&bytes) {
                    Ok(ClientMessage::Push { kind, payload }) => {
                        let ack = {
                            let mut log = group.log.lock().await;
                            let frame = match log.append(kind, payload) {
                                Ok(frame) => frame,
                                Err(e) => {
                                    tracing::error!(error = %e, "append failed");
                                    drop(log);
                                    refuse(&mut socket, "storage failed").await;
                                    return;
                                }
                            };
                            if let Ok(wire) = protocol::encode_server(&ServerMessage::Frame {
                                seq: frame.seq,
                                kind: frame.kind,
                                payload: frame.payload,
                            }) {
                                // Errors only mean "no subscriber" — a
                                // group with one device online.
                                let _ = group.forward.send(Bytes::from(wire));
                            }
                            ServerMessage::Ack { seq: frame.seq }
                        };
                        if send(&mut socket, &ack).await.is_err() {
                            return;
                        }
                    }
                    Ok(ClientMessage::Hello { .. }) => {
                        refuse(&mut socket, "one hello per connection").await;
                        return;
                    }
                    Err(_) => {
                        refuse(&mut socket, "unparsable message").await;
                        return;
                    }
                }
            }
            forwarded = rx.recv() => {
                match forwarded {
                    Ok(wire) => {
                        if socket.send(Message::Binary(wire)).await.is_err() {
                            return;
                        }
                    }
                    // Fell FORWARD_BUFFER frames behind: disconnect, and the
                    // reconnect's replay fills the gap from the log.
                    Err(broadcast::error::RecvError::Lagged(_)) => return,
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
            _ = ping.tick() => {
                // No pong is waited for. This is a keepalive, not a liveness
                // check: a peer that has genuinely gone is discovered by a
                // send failing, which every branch here already returns on.
                // Answering the pong is the browser's job and it does it
                // without the page knowing.
                if socket.send(Message::Ping(Bytes::new())).await.is_err() {
                    return;
                }
            }
        }
    }
}

/// Runs one photo connection to completion (DECISIONS 0080).
///
/// A different shape from [`connection`], and the difference is the whole of
/// the protocol: **there is nothing to replay and nothing to forward.** A
/// photo library has no order, so there is no cursor and no epoch; the hello
/// says what this device holds and what it wants, the welcome answers with the
/// two set differences, and after that every message is one photo, asked for
/// or offered. Nothing is pushed at a device that did not ask.
///
/// The device decides when the conversation is over, because it is the only
/// party that knows what it still wants — so this loop simply serves requests
/// until the socket closes.
async fn photo_connection(mut socket: WebSocket, relay: Arc<Relay>) {
    let Some((id, have, want)) = expect_photo_hello(&mut socket).await else {
        return;
    };
    let group = match relay.group(id).await {
        Ok(group) => group,
        Err(e) => {
            tracing::error!(error = %e, "group photos failed to open");
            photo_refuse(&mut socket, "storage failed").await;
            return;
        }
    };

    // The whole reconciliation, in one round trip. Under the lock so a push
    // landing from the other phone right now is either in `available` or
    // still to come — never counted and then absent.
    let welcome = {
        let photos = group.photos.lock().await;
        PhotoServerMessage::Welcome {
            upload: photos.missing(&have),
            available: photos.present(&want),
        }
    };
    if photo_send(&mut socket, &welcome).await.is_err() {
        return;
    }

    let mut ping = tokio::time::interval_at(
        tokio::time::Instant::now() + relay.ping_every,
        relay.ping_every,
    );
    ping.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            incoming = socket.recv() => {
                let bytes = match incoming {
                    Some(Ok(Message::Binary(bytes))) => bytes,
                    Some(Ok(Message::Close(_))) | None => return,
                    Some(Ok(_)) => continue, // text, ping, pong — not ours
                    Some(Err(_)) => return,
                };
                let answer = match photo::decode_client(&bytes) {
                    Ok(PhotoClientMessage::Push { id, payload }) => {
                        let mut photos = group.photos.lock().await;
                        match photos.store(&id, &payload) {
                            Ok(Ok(())) => PhotoServerMessage::Stored { id },
                            // A cap, not a fault: against this photo and not
                            // against the connection, because the rest of the
                            // queue has nothing wrong with it.
                            Ok(Err(why)) => PhotoServerMessage::Rejected { id, reason: why.0 },
                            Err(e) => {
                                tracing::error!(error = %e, "storing a photo failed");
                                drop(photos);
                                photo_refuse(&mut socket, "storage failed").await;
                                return;
                            }
                        }
                    }
                    Ok(PhotoClientMessage::Fetch { id }) => {
                        let read = { group.photos.lock().await.read(&id) };
                        match read {
                            Ok(Some(payload)) => PhotoServerMessage::Photo { id, payload },
                            // Forgotten between the welcome and the fetch: a
                            // hand runs `forget`, and a hand can run during a
                            // connection (DECISIONS 0050). Answered rather
                            // than ignored — a device waiting on bytes that
                            // will never come stalls its queue for good.
                            Ok(None) => PhotoServerMessage::Absent { id },
                            Err(e) => {
                                tracing::error!(error = %e, "reading a photo failed");
                                photo_refuse(&mut socket, "storage failed").await;
                                return;
                            }
                        }
                    }
                    Ok(PhotoClientMessage::Hello { .. }) => {
                        photo_refuse(&mut socket, "one hello per connection").await;
                        return;
                    }
                    Err(_) => {
                        photo_refuse(&mut socket, "unparsable message").await;
                        return;
                    }
                };
                if photo_send(&mut socket, &answer).await.is_err() {
                    return;
                }
            }
            _ = ping.tick() => {
                // The same keepalive as `/sync`, for the same reason and
                // rather more sharply: a device with nothing to fetch holds
                // this socket open saying nothing at all, which is exactly
                // what a proxy in front of us closes (DECISIONS 0051).
                if socket.send(Message::Ping(Bytes::new())).await.is_err() {
                    return;
                }
            }
        }
    }
}

/// The first message must be a well-formed photo `Hello` speaking this
/// protocol; anything else is refused by name.
///
/// A hostile name never reaches this function: `PhotoName`'s `Deserialize`
/// checks it, so a `../` in `have` is an unparsable hello (DECISIONS 0080).
/// That is what makes naming a file after one safe in [`crate::photos`].
async fn expect_photo_hello(
    socket: &mut WebSocket,
) -> Option<(GroupId, Vec<PhotoName>, Vec<PhotoName>)> {
    let bytes = loop {
        match socket.recv().await? {
            Ok(Message::Binary(bytes)) => break bytes,
            Ok(Message::Close(_)) => return None,
            Ok(_) => continue,
            Err(_) => return None,
        }
    };
    match photo::decode_client(&bytes) {
        Ok(PhotoClientMessage::Hello {
            protocol: version,
            group,
            have,
            want,
        }) => {
            if version != PHOTO_PROTOCOL {
                photo_refuse(socket, &format!("speak photo protocol {PHOTO_PROTOCOL}")).await;
                return None;
            }
            Some((group, have, want))
        }
        Ok(_) => {
            photo_refuse(socket, "hello first").await;
            None
        }
        Err(SyncError::Wire(why)) => {
            // Says which rule the hello broke — a name with a slash in it is
            // the interesting case, and a bare "unparsable" would leave
            // whoever hits it reading postcard's source.
            photo_refuse(socket, &format!("unparsable hello: {why}")).await;
            None
        }
        Err(_) => {
            photo_refuse(socket, "unparsable hello").await;
            None
        }
    }
}

async fn photo_send(socket: &mut WebSocket, message: &PhotoServerMessage) -> Result<(), ()> {
    let wire = photo::encode_server(message).map_err(|_| ())?;
    socket
        .send(Message::Binary(Bytes::from(wire)))
        .await
        .map_err(|_| ())
}

async fn photo_refuse(socket: &mut WebSocket, reason: &str) {
    let _ = photo_send(
        socket,
        &PhotoServerMessage::Refused {
            reason: reason.to_string(),
        },
    )
    .await;
}

/// The first message must be a well-formed `Hello` speaking this protocol;
/// anything else is refused by name.
async fn expect_hello(socket: &mut WebSocket) -> Option<(GroupId, u64, u64)> {
    let bytes = loop {
        match socket.recv().await? {
            Ok(Message::Binary(bytes)) => break bytes,
            Ok(Message::Close(_)) => return None,
            Ok(_) => continue,
            Err(_) => return None,
        }
    };
    match protocol::decode_client(&bytes) {
        Ok(ClientMessage::Hello {
            protocol: version,
            group,
            epoch,
            since,
        }) => {
            if version != PROTOCOL {
                refuse(socket, &format!("speak protocol {PROTOCOL}")).await;
                return None;
            }
            Some((group, epoch, since))
        }
        Ok(_) => {
            refuse(socket, "hello first").await;
            None
        }
        Err(_) => {
            refuse(socket, "unparsable hello").await;
            None
        }
    }
}

async fn send(socket: &mut WebSocket, message: &ServerMessage) -> Result<(), ()> {
    let wire = protocol::encode_server(message).map_err(|_| ())?;
    socket
        .send(Message::Binary(Bytes::from(wire)))
        .await
        .map_err(|_| ())
}

async fn refuse(socket: &mut WebSocket, reason: &str) {
    let _ = send(
        socket,
        &ServerMessage::Refused {
            reason: reason.to_string(),
        },
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    use cabas_sync::protocol::encode_client;
    use futures::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite;

    /// A directory that cleans itself up, so this needs no dev-dep — the
    /// same shape `tests/convergence.rs` uses.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let mut path = std::env::temp_dir();
            path.push(format!(
                "cabas-server-{tag}-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            let _ = std::fs::remove_dir_all(&path);
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// The keepalive of DECISIONS 0051: a connection that has said hello and
    /// then falls silent is pinged anyway.
    ///
    /// The period is milliseconds here for obvious reasons; what the test
    /// pins is that a ping arrives *without traffic*, which is the property
    /// Cloudflare's idle timeout cares about. A relay that only pinged in
    /// response to something would pass every other test in this repo and
    /// drop the socket in a supermarket.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_silent_connection_is_still_pinged() {
        let dir = TempDir::new("ping");
        let relay = Relay::open_every(dir.0.clone(), Duration::from_millis(50)).expect("data dir");
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        tokio::spawn(async move {
            axum::serve(listener, router(relay)).await.expect("serve");
        });

        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/sync"))
            .await
            .expect("connect");

        let hello = encode_client(&ClientMessage::Hello {
            protocol: PROTOCOL,
            group: GroupId::from_hex("00112233445566778899aabbccddeeff").expect("a group id"),
            epoch: 0,
            since: 0,
        })
        .expect("encode");
        ws.send(tungstenite::Message::Binary(hello))
            .await
            .expect("send hello");

        // Welcome and CaughtUp arrive first and are traffic; the ping is what
        // comes after them, on a socket this test deliberately stops using.
        let ping = tokio::time::timeout(Duration::from_secs(5), async {
            while let Some(message) = ws.next().await {
                if let tungstenite::Message::Ping(_) = message.expect("a healthy socket") {
                    return true;
                }
            }
            false
        })
        .await
        .expect("a ping within 5s");

        assert!(ping, "the socket closed before a keepalive arrived");
    }
}
