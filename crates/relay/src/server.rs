//! The WebSocket side: replay, then forward, and never understand.
//!
//! One connection speaks for one device. Its whole life is: a `Hello`
//! naming a family and a cursor, a `Welcome` carrying the log's epoch, the
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

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::{any, get};
use tokio::sync::{Mutex, RwLock, broadcast};

use cabas_sync::FamilyId;
use cabas_sync::protocol::{self, ClientMessage, PROTOCOL, ServerMessage};

use crate::log::FamilyLog;

/// Everything the process holds: where the logs live, and which are open.
/// Families load lazily on their first `Hello` and stay open — a family is
/// two people, and the map is as big as the number of households served.
pub struct Relay {
    root: PathBuf,
    families: RwLock<HashMap<FamilyId, Arc<Family>>>,
}

struct Family {
    log: Mutex<FamilyLog>,
    /// Pre-encoded `ServerMessage::Frame`s, fanned out to every connection.
    /// `Bytes` so a frame is encoded once and cloned by reference count.
    forward: broadcast::Sender<Bytes>,
}

/// Frames a slow connection may fall behind before it is disconnected and
/// made to reconnect — at which point the replay, not the channel, fills
/// the gap. Losing a channel message is therefore never losing data.
const FORWARD_BUFFER: usize = 256;

impl Relay {
    pub fn open(root: PathBuf) -> std::io::Result<Arc<Self>> {
        std::fs::create_dir_all(&root)?;
        Ok(Arc::new(Relay {
            root,
            families: RwLock::new(HashMap::new()),
        }))
    }

    async fn family(&self, id: FamilyId) -> std::io::Result<Arc<Family>> {
        if let Some(family) = self.families.read().await.get(&id) {
            return Ok(family.clone());
        }
        let mut families = self.families.write().await;
        // Two devices of one family saying hello at once race to this
        // write lock; the loser must find the winner's log, not a second
        // one over the same directory.
        if let Some(family) = families.get(&id) {
            return Ok(family.clone());
        }
        let log = FamilyLog::open(self.root.join(id.to_hex()))?;
        let (forward, _) = broadcast::channel(FORWARD_BUFFER);
        let family = Arc::new(Family {
            log: Mutex::new(log),
            forward,
        });
        families.insert(id, family.clone());
        Ok(family)
    }
}

pub fn router(relay: Arc<Relay>) -> Router {
    Router::new()
        .route("/sync", any(ws_handler))
        // For the M6 add-on's watchdog; says the process is up, nothing else.
        .route("/healthz", get(|| async { "ok" }))
        .with_state(relay)
}

async fn ws_handler(ws: WebSocketUpgrade, State(relay): State<Arc<Relay>>) -> Response {
    ws.on_upgrade(move |socket| connection(socket, relay))
}

/// Runs one connection to completion. Exits on close, on error, and on the
/// one impoliteness the relay answers with words: a `Refused` names its
/// reason before the socket drops, because a silent close reads as a
/// network blip and invites a pointless retry.
async fn connection(mut socket: WebSocket, relay: Arc<Relay>) {
    let (family, hello_epoch, hello_since) = match expect_hello(&mut socket).await {
        Some(hello) => hello,
        None => return,
    };
    let family = match relay.family(family).await {
        Ok(family) => family,
        Err(e) => {
            tracing::error!(error = %e, "family log failed to open");
            refuse(&mut socket, "storage failed").await;
            return;
        }
    };

    // Subscribe and snapshot the replay under one lock: a push landing now
    // is either in `replay` or already in `rx`, never lost between them.
    let (epoch, replay, mut rx) = {
        let log = family.log.lock().await;
        let since = if hello_epoch == log.epoch() {
            hello_since
        } else {
            // The cursor points into a log that no longer exists — replay
            // everything and let `Welcome` tell the device why.
            0
        };
        (log.epoch(), log.replay(since), family.forward.subscribe())
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
                            let mut log = family.log.lock().await;
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
                                // family with one device online.
                                let _ = family.forward.send(Bytes::from(wire));
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
        }
    }
}

/// The first message must be a well-formed `Hello` speaking this protocol;
/// anything else is refused by name.
async fn expect_hello(socket: &mut WebSocket) -> Option<(FamilyId, u64, u64)> {
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
            family,
            epoch,
            since,
        }) => {
            if version != PROTOCOL {
                refuse(socket, &format!("speak protocol {PROTOCOL}")).await;
                return None;
            }
            Some((family, epoch, since))
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
