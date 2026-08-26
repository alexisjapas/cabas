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
//!
//! # And `/photos`, which shares the port and nothing else
//!
//! The second endpoint is at the bottom of this file and has none of the
//! shape above: no epoch, no cursor, no replay, no subscription and no
//! broadcast (DECISIONS 0080). A photo library has no order, so what
//! replaces all of it is one round trip — a hello saying what this device
//! holds and what it lacks, a welcome answering with the difference — and
//! then one photo per message, in whichever direction asked for it. The two
//! endpoints share the group id, the keepalive and the process. They do not
//! share a lock, a directory or a protocol byte, which is what lets a phone
//! that speaks only [`PROTOCOL`] keep converging (0062).

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

use cabas_sync::GroupId;
use cabas_sync::photo::{self, PHOTO_PROTOCOL, PhotoClientMessage, PhotoName, PhotoServerMessage};
use cabas_sync::protocol::{self, ClientMessage, PROTOCOL, ServerMessage};

use crate::log::GroupLog;
use crate::photos::{DEFAULT_CAP, GroupPhotos, Kept};

/// Everything the process holds: where the logs live, and which are open.
/// Groups load lazily on their first `Hello` and stay open — a group is
/// two people, and the map is as big as the number of households served.
pub struct Relay {
    root: PathBuf,
    groups: RwLock<HashMap<GroupId, Arc<Group>>>,
    /// The same laziness for the other endpoint, and a second map rather
    /// than a second field on [`Group`]: a photo hello must not open a log,
    /// because opening one mints an epoch for a group that has none and
    /// rewrites its `meta` (see [`crate::admin`]). The two endpoints are
    /// independent all the way down to the lock.
    photos: RwLock<HashMap<GroupId, Arc<Mutex<GroupPhotos>>>>,
    /// How often a connection is pinged. A field rather than a bare constant
    /// so the test below can watch one arrive without sitting out
    /// `PING_EVERY`; nothing outside this module can set it.
    ping_every: Duration,
    /// What one group's photos may weigh. A field for the same reason: a
    /// test that had to write half a gigabyte to see a refusal would not be
    /// written.
    photo_cap: u64,
}

struct Group {
    log: Mutex<GroupLog>,
    /// Pre-encoded `ServerMessage::Frame`s, fanned out to every connection.
    /// `Bytes` so a frame is encoded once and cloned by reference count.
    forward: broadcast::Sender<Bytes>,
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
        Self::open_with(root, PING_EVERY, DEFAULT_CAP)
    }

    fn open_with(
        root: PathBuf,
        ping_every: Duration,
        photo_cap: u64,
    ) -> std::io::Result<Arc<Self>> {
        std::fs::create_dir_all(&root)?;
        Ok(Arc::new(Relay {
            root,
            groups: RwLock::new(HashMap::new()),
            photos: RwLock::new(HashMap::new()),
            ping_every,
            photo_cap,
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
        let log = GroupLog::open(self.root.join(id.to_hex()))?;
        let (forward, _) = broadcast::channel(FORWARD_BUFFER);
        let group = Arc::new(Group {
            log: Mutex::new(log),
            forward,
        });
        groups.insert(id, group.clone());
        Ok(group)
    }

    /// One group's photos, opened on first use and kept — the same
    /// double-checked shape as [`Relay::group`], and for the same reason:
    /// two devices saying hello at once must share one index over one
    /// directory, or the second would write against a stale weight.
    async fn photos(&self, id: GroupId) -> std::io::Result<Arc<Mutex<GroupPhotos>>> {
        if let Some(photos) = self.photos.read().await.get(&id) {
            return Ok(photos.clone());
        }
        let mut open = self.photos.write().await;
        if let Some(photos) = open.get(&id) {
            return Ok(photos.clone());
        }
        let photos = Arc::new(Mutex::new(GroupPhotos::open(
            crate::photos::dir_of(&self.root.join(id.to_hex())),
            self.photo_cap,
        )?));
        open.insert(id, photos.clone());
        Ok(photos)
    }
}

/// The three named routes, and the PWA under everything else.
///
/// Order matters only in that `/sync`, `/photos` and `/healthz` are named:
/// the fallback answers every other path out of the embedded bundle, so a
/// file called `sync` in `ui/dist` would be unreachable — which is a rule the
/// bundler cannot break, since it names its own outputs.
///
/// `get` rather than `any` for the assets: it covers HEAD, whose body axum
/// discards for us, and answers 405 to a POST at a static file instead of
/// serving it.
pub fn router(relay: Arc<Relay>) -> Router {
    Router::new()
        .route("/sync", any(ws_handler))
        .route("/photos", any(photo_ws_handler))
        // For the M6 add-on's watchdog; says the process is up, nothing else.
        .route("/healthz", get(|| async { "ok" }))
        .fallback(get(crate::assets::handler))
        .with_state(relay)
}

async fn ws_handler(ws: WebSocketUpgrade, State(relay): State<Arc<Relay>>) -> Response {
    ws.on_upgrade(move |socket| connection(socket, relay))
}

/// The largest message `/photos` will read, and the outer of the two limits
/// this endpoint owes (DECISIONS 0080).
///
/// A hello's two lists and a push's payload are the only unbounded things a
/// stranger who found the group id can send, and axum's default is 64 MB per
/// message — enough to make a memory exhaustion out of one socket. The
/// number sits deliberately **above** [`crate::photos::MAX_BLOB_BYTES`], so
/// the two limits have different jobs: an honestly oversized photo is refused
/// by name, with words a person reads, while this one is a wall that only a
/// message no version of this app sends can hit, and hitting it drops the
/// socket. There is nothing to say to a caller that is not speaking the
/// protocol.
const MAX_MESSAGE: usize = 2 * 1024 * 1024;

/// `max_frame_size` as well as `max_message_size`: a message is assembled
/// from frames, and capping only the total would let a single enormous frame
/// be read before the total was known.
async fn photo_ws_handler(ws: WebSocketUpgrade, State(relay): State<Arc<Relay>>) -> Response {
    ws.max_message_size(MAX_MESSAGE)
        .max_frame_size(MAX_MESSAGE)
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

// --- `/photos` (DECISIONS 0080, 0081) ---------------------------------------

/// Runs one photo connection to completion.
///
/// Shorter than its neighbour above by construction: a hello, a welcome, and
/// then a message per photo until the device has what it came for and closes
/// the socket. **The relay never decides that the conversation is over** —
/// only the device knows what it still wants, so there is no `CaughtUp` here
/// and nothing that ends this loop but the socket itself.
///
/// Nothing is forwarded. A photo arriving from one device is not pushed at
/// the other; the other asks for it on its own next connection, out of the
/// hello it computes from its own disk. That is what keeps a 200 kB transfer
/// off the path a list edit takes (0062) and what makes the whole of this
/// endpoint's state a directory.
async fn photo_connection(mut socket: WebSocket, relay: Arc<Relay>) {
    let (group, have, want) = match expect_photo_hello(&mut socket).await {
        Some(hello) => hello,
        None => return,
    };
    let photos = match relay.photos(group).await {
        Ok(photos) => photos,
        Err(e) => {
            tracing::error!(error = %e, "photo directory failed to open");
            refuse_photo(&mut socket, "storage failed").await;
            return;
        }
    };

    // The whole of the reconciliation, in the one message the device did not
    // have to ask for: what it offered that is not here, and what it asked
    // for that is. Both are subsets of what the hello said — the device
    // narrows them again on arrival, because it is the untrusting party here
    // and this relay is the untrusted one (Rule 7).
    let welcome = {
        let store = photos.lock().await;
        PhotoServerMessage::Welcome {
            upload: have.into_iter().filter(|id| !store.holds(id)).collect(),
            available: want.into_iter().filter(|id| store.holds(id)).collect(),
        }
    };
    if send_photo(&mut socket, &welcome).await.is_err() {
        return;
    }

    // The keepalive of DECISIONS 0051 applies here too and matters less: a
    // photo connection is short by construction. It costs one timer.
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
                        let mut store = photos.lock().await;
                        match store.store(&id, &payload) {
                            Ok(Kept::Stored) => PhotoServerMessage::Stored { id },
                            Ok(Kept::Rejected(reason)) => {
                                PhotoServerMessage::Rejected { id, reason }
                            }
                            Err(e) => {
                                tracing::error!(error = %e, "storing a photo failed");
                                drop(store);
                                refuse_photo(&mut socket, "storage failed").await;
                                return;
                            }
                        }
                    }
                    Ok(PhotoClientMessage::Fetch { id }) => {
                        let store = photos.lock().await;
                        match store.load(&id) {
                            // Forgotten between the welcome and now — a hand
                            // runs `forget` (0050), and silence would stall
                            // this device's queue for good.
                            Ok(None) => PhotoServerMessage::Absent { id },
                            Ok(Some(payload)) => PhotoServerMessage::Photo { id, payload },
                            Err(e) => {
                                tracing::error!(error = %e, "reading a photo failed");
                                drop(store);
                                refuse_photo(&mut socket, "storage failed").await;
                                return;
                            }
                        }
                    }
                    Ok(PhotoClientMessage::Hello { .. }) => {
                        // A second welcome would re-queue work already done,
                        // and the client rejects one anyway — this is the
                        // same rule stated from the other end.
                        refuse_photo(&mut socket, "one hello per connection").await;
                        return;
                    }
                    Err(_) => {
                        refuse_photo(&mut socket, "unparsable message").await;
                        return;
                    }
                };
                if send_photo(&mut socket, &answer).await.is_err() {
                    return;
                }
            }
            _ = ping.tick() => {
                if socket.send(Message::Ping(Bytes::new())).await.is_err() {
                    return;
                }
            }
        }
    }
}

/// The first message must be a well-formed photo `Hello` speaking this
/// endpoint's own protocol byte — which is not [`PROTOCOL`], and moves on its
/// own (DECISIONS 0080's decision 8).
///
/// A name that could name something other than a file never gets this far:
/// [`PhotoName`] is checked as it is decoded, in `cabas-sync`, precisely so
/// that the party about to write a file cannot be the party responsible for
/// remembering.
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
                refuse_photo(socket, &format!("speak photo protocol {PHOTO_PROTOCOL}")).await;
                return None;
            }
            Some((group, have, want))
        }
        Ok(_) => {
            refuse_photo(socket, "hello first").await;
            None
        }
        Err(_) => {
            refuse_photo(socket, "unparsable hello").await;
            None
        }
    }
}

async fn send_photo(socket: &mut WebSocket, message: &PhotoServerMessage) -> Result<(), ()> {
    let wire = photo::encode_server(message).map_err(|_| ())?;
    socket
        .send(Message::Binary(Bytes::from(wire)))
        .await
        .map_err(|_| ())
}

async fn refuse_photo(socket: &mut WebSocket, reason: &str) {
    let _ = send_photo(
        socket,
        &PhotoServerMessage::Refused {
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

    type Ws = tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >;

    /// A relay on a real port, with both periods and the cap in the caller's
    /// hands — a test that had to wait out `PING_EVERY` or write half a
    /// gigabyte would not be written.
    async fn serve(dir: &TempDir, ping_every: Duration, cap: u64) -> std::net::SocketAddr {
        let relay = Relay::open_with(dir.0.clone(), ping_every, cap).expect("data dir");
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        tokio::spawn(async move {
            axum::serve(listener, router(relay)).await.expect("serve");
        });
        addr
    }

    async fn connect(addr: std::net::SocketAddr, path: &str) -> Ws {
        let (ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}{path}"))
            .await
            .expect("connect");
        ws
    }

    fn group() -> GroupId {
        GroupId::from_hex("00112233445566778899aabbccddeeff").expect("a group id")
    }

    fn name(raw: &str) -> PhotoName {
        PhotoName::new(raw).expect("a name")
    }

    async fn say(ws: &mut Ws, message: &PhotoClientMessage) {
        ws.send(tungstenite::Message::Binary(
            photo::encode_client(message).expect("encode"),
        ))
        .await
        .expect("send");
    }

    /// The next server message, or a loud failure — a hang here is a
    /// protocol bug, and CI deserves a message over a timeout.
    async fn hear(ws: &mut Ws) -> PhotoServerMessage {
        let deadline = Duration::from_secs(10);
        loop {
            let message = tokio::time::timeout(deadline, ws.next())
                .await
                .expect("a message within 10s")
                .expect("an open socket")
                .expect("a healthy socket");
            match message {
                tungstenite::Message::Binary(bytes) => {
                    return photo::decode_server(&bytes).expect("a server message");
                }
                tungstenite::Message::Close(_) => panic!("closed while a message was expected"),
                _ => continue,
            }
        }
    }

    /// A hello for a group that has nothing yet: everything offered is owed,
    /// nothing asked for is available.
    async fn photo_hello(ws: &mut Ws, have: &[&str], want: &[&str]) -> PhotoServerMessage {
        say(
            ws,
            &PhotoClientMessage::Hello {
                protocol: PHOTO_PROTOCOL,
                group: group(),
                have: have.iter().map(|id| name(id)).collect(),
                want: want.iter().map(|id| name(id)).collect(),
            },
        )
        .await;
        hear(ws).await
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
        let addr = serve(&dir, Duration::from_millis(50), DEFAULT_CAP).await;
        let mut ws = connect(addr, "/sync").await;

        let hello = encode_client(&ClientMessage::Hello {
            protocol: PROTOCOL,
            group: group(),
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

    // --- `/photos` ---------------------------------------------------------

    /// The whole of a connection: one round trip that says what is owed in
    /// each direction, then a photo per message. The welcome is the only
    /// message the device did not ask for (DECISIONS 0080).
    #[tokio::test(flavor = "multi_thread")]
    async fn a_photo_pushed_by_one_connection_is_fetched_by_the_next() {
        let dir = TempDir::new("photos-round-trip");
        let addr = serve(&dir, PING_EVERY, DEFAULT_CAP).await;

        // A device holding one photo, wanting one it has heard of.
        let mut ws = connect(addr, "/photos").await;
        assert_eq!(
            photo_hello(&mut ws, &["pho_a1"], &["pho_b2"]).await,
            PhotoServerMessage::Welcome {
                upload: vec![name("pho_a1")],
                available: vec![],
            },
            "an empty relay owes the offer and can answer nothing"
        );
        say(
            &mut ws,
            &PhotoClientMessage::Push {
                id: name("pho_a1"),
                payload: b"sealed bytes".to_vec(),
            },
        )
        .await;
        assert_eq!(
            hear(&mut ws).await,
            PhotoServerMessage::Stored { id: name("pho_a1") }
        );
        ws.close(None).await.expect("close");

        // The other device, later, with nothing and wanting that photo.
        let mut ws = connect(addr, "/photos").await;
        assert_eq!(
            photo_hello(&mut ws, &[], &["pho_a1", "pho_b2"]).await,
            PhotoServerMessage::Welcome {
                upload: vec![],
                available: vec![name("pho_a1")],
            },
            "only what is here is offered — pho_b2 is on a phone that has not connected"
        );
        say(&mut ws, &PhotoClientMessage::Fetch { id: name("pho_a1") }).await;
        assert_eq!(
            hear(&mut ws).await,
            PhotoServerMessage::Photo {
                id: name("pho_a1"),
                payload: b"sealed bytes".to_vec(),
            },
            "stored verbatim and handed back verbatim — the relay re-encodes nothing"
        );
    }

    /// A `forget` (DECISIONS 0050) is run by a person and can land between
    /// the welcome and the fetch. Silence would stall that device's queue for
    /// good, so the relay answers.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_photo_that_is_not_here_is_answered_absent() {
        let dir = TempDir::new("photos-absent");
        let addr = serve(&dir, PING_EVERY, DEFAULT_CAP).await;
        let mut ws = connect(addr, "/photos").await;

        photo_hello(&mut ws, &[], &[]).await;
        say(
            &mut ws,
            &PhotoClientMessage::Fetch {
                id: name("pho_never_pushed"),
            },
        )
        .await;
        assert_eq!(
            hear(&mut ws).await,
            PhotoServerMessage::Absent {
                id: name("pho_never_pushed")
            }
        );
    }

    /// The cap is against the photo and never against the connection: the
    /// queue behind a refused push has nothing wrong with it, and a socket
    /// dropped here would offer the same photo first on every reconnect,
    /// forever (DECISIONS 0080's decision 6).
    #[tokio::test(flavor = "multi_thread")]
    async fn a_group_at_its_cap_is_refused_by_photo_and_keeps_its_connection() {
        let dir = TempDir::new("photos-cap");
        let addr = serve(&dir, PING_EVERY, 1024).await;
        let mut ws = connect(addr, "/photos").await;

        photo_hello(&mut ws, &["pho_a1", "pho_b2"], &[]).await;
        say(
            &mut ws,
            &PhotoClientMessage::Push {
                id: name("pho_a1"),
                payload: vec![0u8; 900],
            },
        )
        .await;
        assert_eq!(
            hear(&mut ws).await,
            PhotoServerMessage::Stored { id: name("pho_a1") }
        );

        say(
            &mut ws,
            &PhotoClientMessage::Push {
                id: name("pho_b2"),
                payload: vec![0u8; 900],
            },
        )
        .await;
        let PhotoServerMessage::Rejected { id, reason } = hear(&mut ws).await else {
            panic!("the cap let a push through, or took the connection with it");
        };
        assert_eq!(id, name("pho_b2"));
        assert!(reason.contains("at its"), "{reason}");

        // Still talking: the connection outlives the refusal.
        say(&mut ws, &PhotoClientMessage::Fetch { id: name("pho_a1") }).await;
        assert!(matches!(
            hear(&mut ws).await,
            PhotoServerMessage::Photo { .. }
        ));
    }

    /// `/photos` carries its own protocol byte, so that a change to the
    /// photo conversation never asks a phone in a pocket to stop converging
    /// (DECISIONS 0062, 0080's decision 8).
    #[tokio::test(flavor = "multi_thread")]
    async fn a_photo_hello_speaking_another_protocol_is_refused_by_name() {
        let dir = TempDir::new("photos-protocol");
        let addr = serve(&dir, PING_EVERY, DEFAULT_CAP).await;
        let mut ws = connect(addr, "/photos").await;

        say(
            &mut ws,
            &PhotoClientMessage::Hello {
                protocol: PHOTO_PROTOCOL + 1,
                group: group(),
                have: vec![],
                want: vec![],
            },
        )
        .await;
        let PhotoServerMessage::Refused { reason } = hear(&mut ws).await else {
            panic!("a stranger's protocol was accepted");
        };
        assert!(reason.contains(&PHOTO_PROTOCOL.to_string()), "{reason}");
    }

    /// The three impolitenesses, each answered with words before the socket
    /// drops — the same courtesy `/sync` extends, because a silent close
    /// reads as a network blip and invites a pointless retry.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_photo_endpoint_says_why_it_is_hanging_up() {
        let dir = TempDir::new("photos-rude");
        let addr = serve(&dir, PING_EVERY, DEFAULT_CAP).await;

        // A fetch before any hello.
        let mut ws = connect(addr, "/photos").await;
        say(&mut ws, &PhotoClientMessage::Fetch { id: name("pho_a1") }).await;
        assert_eq!(
            hear(&mut ws).await,
            PhotoServerMessage::Refused {
                reason: "hello first".to_string()
            }
        );

        // Bytes that are not a message at all.
        let mut ws = connect(addr, "/photos").await;
        ws.send(tungstenite::Message::Binary(vec![0xde, 0xad]))
            .await
            .expect("send");
        assert_eq!(
            hear(&mut ws).await,
            PhotoServerMessage::Refused {
                reason: "unparsable hello".to_string()
            }
        );

        // A second hello would re-queue work already done, and would keep
        // this socket open for good: there is exactly one round trip.
        let mut ws = connect(addr, "/photos").await;
        photo_hello(&mut ws, &[], &[]).await;
        assert_eq!(
            photo_hello(&mut ws, &[], &[]).await,
            PhotoServerMessage::Refused {
                reason: "one hello per connection".to_string()
            }
        );
    }

    /// The keepalive applies to this endpoint too (DECISIONS 0051). It
    /// matters less — a photo connection is short by construction — and it
    /// costs one timer, which is cheaper than finding out in a shop that
    /// Cloudflare closed a socket in the middle of a prefetch.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_silent_photo_connection_is_pinged_too() {
        let dir = TempDir::new("photos-ping");
        let addr = serve(&dir, Duration::from_millis(50), DEFAULT_CAP).await;
        let mut ws = connect(addr, "/photos").await;
        photo_hello(&mut ws, &[], &[]).await;

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

    /// A photo hello must not mine a group directory into existence. The log
    /// side creates one on its first hello — the id is unguessable, so that
    /// is safe — but doing it here would put an empty directory under `/data`
    /// for a group that has never synced, and `admin::survey` would report a
    /// group that does not exist.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_photo_connection_alone_leaves_no_directory_behind() {
        let dir = TempDir::new("photos-no-dir");
        let addr = serve(&dir, PING_EVERY, DEFAULT_CAP).await;
        let mut ws = connect(addr, "/photos").await;
        photo_hello(&mut ws, &[], &["pho_a1"]).await;
        ws.close(None).await.expect("close");

        assert!(
            crate::admin::survey(&dir.0).expect("survey").is_empty(),
            "a hello that stored nothing left a group on disk"
        );
    }
}
