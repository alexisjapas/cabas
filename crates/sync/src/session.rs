//! The client half of a sync connection, with no socket in sight.
//!
//! Sans-IO on purpose: the PWA speaks WebSocket through the browser's own
//! API, from the frontend (DECISIONS 0043), the native hosts through
//! `tokio-tungstenite`, and everything the two would otherwise duplicate —
//! sealing, opening, cursor discipline, the epoch reset — lives here as
//! plain synchronous calls on bytes. The transport adapters stay so thin
//! they are written next to their event loops rather than behind a trait
//! invented before its second user (the M3 note in ROADMAP makes the same
//! argument for storage).
//!
//! The caller owns the replica. This type never sees `App` or `Document`;
//! it turns wire bytes into "merge this plaintext" and local deltas into
//! wire bytes, and tracks the two numbers worth persisting.

use crate::error::{Result, SyncError};
use crate::key::FamilyKey;
use crate::protocol::{self, ClientMessage, FrameKind, PROTOCOL, ServerMessage};
use crate::seal;

/// What the caller must do with a server message, once the session has
/// opened, verified and accounted for it.
#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    /// The relay answered the hello; the connection is on. The cursor may
    /// have been reset if the relay's log is not the one this device knew.
    Connected,

    /// An opened payload — hand it to `App::merge`. The cursor has already
    /// advanced; persist it (see [`Session::cursor`]) once the merge lands.
    Merge(Vec<u8>),

    /// The replay is exhausted. This is the moment to push what was
    /// produced offline: `App::changes_since(shadow)`, through
    /// [`Session::delta`].
    CaughtUp,

    /// A push is durable on the relay. The caller advances its shadow
    /// version — the cursor itself moves on frames alone, because acks and
    /// frames travel independently and only the frame stream is ordered.
    Acked { seq: u64 },

    /// The relay hung up with a reason: protocol mismatch, malformed
    /// message. Reconnecting without changing something is pointless.
    Refused { reason: String },

    /// A frame arrived that the family key does not open — a stranger who
    /// found the id, or a corrupted blob. Dropped, counted, cursor
    /// advanced: refetching it forever would not make it open (0042).
    Dropped { seq: u64 },
}

/// One connection's worth of client state. Create it from the persisted
/// cursor, drive it with wire bytes, persist [`Session::cursor`] when it
/// moves.
pub struct Session {
    key: FamilyKey,
    epoch: u64,
    since: u64,
    replayed: u64,
    dropped: u64,
}

impl Session {
    /// `epoch` and `since` are what [`Session::cursor`] returned last time —
    /// zeros on a device that has never synced.
    pub fn new(key: FamilyKey, epoch: u64, since: u64) -> Self {
        Session {
            key,
            epoch,
            since,
            replayed: 0,
            dropped: 0,
        }
    }

    /// The opening message. Send it, then feed every incoming binary
    /// message to [`Session::handle`].
    pub fn hello(&self) -> Result<Vec<u8>> {
        protocol::encode_client(&ClientMessage::Hello {
            protocol: PROTOCOL,
            family: self.key.id(),
            epoch: self.epoch,
            since: self.since,
        })
    }

    /// Seals a local delta — `App::changes_since(shadow)` — into a push.
    pub fn delta(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let payload = seal::seal(&self.key, plaintext)?;
        protocol::encode_client(&ClientMessage::Push {
            kind: FrameKind::Delta,
            payload,
        })
    }

    /// Seals a full snapshot into a push that lets the relay drop every
    /// frame this session has applied. Compaction is device-driven because
    /// the relay cannot merge what it cannot read (0042); a caller that saw
    /// a long replay ([`Session::replayed`]) is the natural volunteer.
    pub fn snapshot(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let payload = seal::seal(&self.key, plaintext)?;
        protocol::encode_client(&ClientMessage::Push {
            kind: FrameKind::Snapshot { covers: self.since },
            payload,
        })
    }

    /// One incoming binary message → one instruction to the caller.
    pub fn handle(&mut self, wire: &[u8]) -> Result<Event> {
        match protocol::decode_server(wire)? {
            ServerMessage::Welcome { epoch } => {
                if epoch != self.epoch {
                    // The log this cursor pointed into no longer exists —
                    // the relay was reset or restored. Start over; merges
                    // being idempotent makes the full replay only verbose.
                    self.since = 0;
                    self.epoch = epoch;
                }
                Ok(Event::Connected)
            }
            ServerMessage::Frame { seq, payload, .. } => {
                self.replayed += 1;
                self.since = self.since.max(seq);
                match seal::open(&self.key, &payload) {
                    Ok(plaintext) => Ok(Event::Merge(plaintext)),
                    Err(SyncError::Open) => {
                        self.dropped += 1;
                        Ok(Event::Dropped { seq })
                    }
                    Err(other) => Err(other),
                }
            }
            ServerMessage::CaughtUp => Ok(Event::CaughtUp),
            ServerMessage::Ack { seq } => Ok(Event::Acked { seq }),
            ServerMessage::Refused { reason } => Ok(Event::Refused { reason }),
        }
    }

    /// What to persist, together, whenever it changes: the relay's epoch
    /// and the last sequence applied. Device-local coordination state —
    /// stored alongside the identity, never synced (0042).
    pub fn cursor(&self) -> (u64, u64) {
        (self.epoch, self.since)
    }

    /// Frames seen on this connection — replay and live alike. A caller
    /// watching this grow past a threshold should answer with
    /// [`Session::snapshot`].
    pub fn replayed(&self) -> u64 {
        self.replayed
    }

    /// Frames the key refused to open, kept visible for a future
    /// diagnostics screen: a nonzero count is either corruption or company.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{decode_client, encode_server};

    fn session() -> Session {
        Session::new(FamilyKey::generate().unwrap(), 0, 0)
    }

    fn wire(message: &ServerMessage) -> Vec<u8> {
        encode_server(message).unwrap()
    }

    #[test]
    fn hello_carries_the_persisted_cursor() {
        let key = FamilyKey::generate().unwrap();
        let id = key.id();
        let s = Session::new(key, 5, 17);
        match decode_client(&s.hello().unwrap()).unwrap() {
            ClientMessage::Hello {
                protocol,
                family,
                epoch,
                since,
            } => {
                assert_eq!(protocol, PROTOCOL);
                assert_eq!(family, id);
                assert_eq!(epoch, 5);
                assert_eq!(since, 17);
            }
            other => panic!("expected a hello, got {other:?}"),
        }
    }

    #[test]
    fn a_matching_epoch_keeps_the_cursor() {
        let key = FamilyKey::generate().unwrap();
        let mut s = Session::new(key, 5, 17);
        assert_eq!(
            s.handle(&wire(&ServerMessage::Welcome { epoch: 5 }))
                .unwrap(),
            Event::Connected
        );
        assert_eq!(s.cursor(), (5, 17));
    }

    #[test]
    fn a_new_epoch_resets_the_cursor() {
        let key = FamilyKey::generate().unwrap();
        let mut s = Session::new(key, 5, 17);
        s.handle(&wire(&ServerMessage::Welcome { epoch: 6 }))
            .unwrap();
        assert_eq!(
            s.cursor(),
            (6, 0),
            "old sequence numbers point into a log that is gone"
        );
    }

    #[test]
    fn frames_open_advance_and_surface_the_plaintext() {
        let key = FamilyKey::from_phrase(
            "abandon abandon abandon abandon abandon abandon \
             abandon abandon abandon abandon abandon about",
        )
        .unwrap();
        let sealed = seal::seal(&key, b"ops from the other phone").unwrap();
        let mut s = Session::new(key, 0, 0);
        let event = s
            .handle(&wire(&ServerMessage::Frame {
                seq: 4,
                kind: FrameKind::Delta,
                payload: sealed,
            }))
            .unwrap();
        assert_eq!(event, Event::Merge(b"ops from the other phone".to_vec()));
        assert_eq!(s.cursor(), (0, 4));
        assert_eq!(s.replayed(), 1);
        assert_eq!(s.dropped(), 0);
    }

    #[test]
    fn a_frame_that_does_not_open_is_dropped_and_stepped_over() {
        let mut s = session();
        let event = s
            .handle(&wire(&ServerMessage::Frame {
                seq: 9,
                kind: FrameKind::Delta,
                payload: vec![0; 64],
            }))
            .unwrap();
        assert_eq!(event, Event::Dropped { seq: 9 });
        assert_eq!(
            s.cursor().1,
            9,
            "refetching garbage forever would not make it open"
        );
        assert_eq!(s.dropped(), 1);
    }

    #[test]
    fn pushes_seal_and_declare_their_kind() {
        let key = FamilyKey::generate().unwrap();
        let opener = FamilyKey::from_phrase(key.phrase()).unwrap();
        let mut s = Session::new(key, 0, 0);
        s.handle(&wire(&ServerMessage::Frame {
            seq: 21,
            kind: FrameKind::Delta,
            payload: seal::seal(&opener, b"x").unwrap(),
        }))
        .unwrap();

        match decode_client(&s.delta(b"local edits").unwrap()).unwrap() {
            ClientMessage::Push { kind, payload } => {
                assert_eq!(kind, FrameKind::Delta);
                assert_eq!(seal::open(&opener, &payload).unwrap(), b"local edits");
            }
            other => panic!("expected a push, got {other:?}"),
        }
        match decode_client(&s.snapshot(b"whole doc").unwrap()).unwrap() {
            ClientMessage::Push { kind, payload } => {
                assert_eq!(
                    kind,
                    FrameKind::Snapshot { covers: 21 },
                    "a snapshot covers exactly what this session has applied"
                );
                assert_eq!(seal::open(&opener, &payload).unwrap(), b"whole doc");
            }
            other => panic!("expected a push, got {other:?}"),
        }
    }

    #[test]
    fn acks_and_refusals_pass_through() {
        let mut s = session();
        assert_eq!(
            s.handle(&wire(&ServerMessage::Ack { seq: 3 })).unwrap(),
            Event::Acked { seq: 3 }
        );
        assert_eq!(
            s.handle(&wire(&ServerMessage::CaughtUp)).unwrap(),
            Event::CaughtUp
        );
        assert_eq!(
            s.handle(&wire(&ServerMessage::Refused {
                reason: "speak protocol 1".into()
            }))
            .unwrap(),
            Event::Refused {
                reason: "speak protocol 1".into()
            }
        );
    }

    #[test]
    fn garbage_from_the_wire_is_an_error_not_a_panic() {
        let mut s = session();
        assert!(matches!(s.handle(&[0xba, 0xad]), Err(SyncError::Wire(_))));
    }
}
