//! The wire protocol between a device and the relay (DECISIONS 0042).
//!
//! Four facts shape it. The relay cannot read a version vector, so the only
//! ordering is the **sequence number** the relay assigns as it appends. The
//! payloads are ciphertext, so the codec is `postcard` — binary,
//! serde-native, and not self-describing, because there is nothing to
//! describe. The frame **kind** and what a snapshot **covers** are plaintext
//! metadata, the irreducible minimum the relay needs to truncate its log.
//! And `Hello` carries a protocol version byte, so an incompatible future
//! change is a clean [`ServerMessage::Refused`] rather than a misparse.
//!
//! The **epoch** exists for one scenario: the relay losing its log — a disk
//! swap, a restore from an older Home Assistant backup (ROADMAP M6). Restart
//! sequence numbers without it and a device whose cursor points past the end
//! of the reborn log would silently skip everything in it. The epoch is
//! minted with the log; a mismatch tells both sides to start the replay from
//! the beginning, which idempotent merges make merely verbose, never wrong.

use serde::{Deserialize, Serialize};

use crate::error::{Result, SyncError};
use crate::key::FamilyId;

/// Bumped on any incompatible change to the messages below. The relay
/// refuses a `Hello` carrying anything else.
pub const PROTOCOL: u8 = 1;

/// What a frame is to the *relay*: a delta it appends, or a snapshot that
/// lets it drop every frame at or below `covers`. A client merges both
/// identically — the distinction exists so the log can shrink.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameKind {
    Delta,
    Snapshot { covers: u64 },
}

/// Device → relay.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ClientMessage {
    /// The opening message of every connection: which family, and where the
    /// replay should start. `epoch` and `since` are the device's persisted
    /// cursor; a device that has never synced sends zero for both.
    Hello {
        protocol: u8,
        family: FamilyId,
        epoch: u64,
        since: u64,
    },

    /// A sealed payload to append. The relay assigns the sequence number and
    /// answers with [`ServerMessage::Ack`].
    Push { kind: FrameKind, payload: Vec<u8> },
}

/// Relay → device.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ServerMessage {
    /// First reply to `Hello`, before any frame. A device seeing an epoch
    /// other than its persisted one resets its cursor: the sequence numbers
    /// it knew belong to a log that no longer exists.
    Welcome { epoch: u64 },

    /// One log entry. During replay these arrive in sequence order; after
    /// [`ServerMessage::CaughtUp`], live pushes from other devices — and the
    /// device's own, which the relay does not bother to skip: merging your
    /// own delta back is a no-op by CRDT construction.
    Frame {
        seq: u64,
        kind: FrameKind,
        payload: Vec<u8>,
    },

    /// The replay is exhausted; everything from here on is live. This is
    /// the moment a device is entitled to push what it produced offline.
    CaughtUp,

    /// The push at the head of this connection's pipe is durable under this
    /// sequence number. The device may advance its shadow version — its
    /// cursor moves on frames alone.
    Ack { seq: u64 },

    /// The connection is over and this says why: a protocol mismatch, an
    /// unparsable message. Sent once, then the socket closes.
    Refused { reason: String },
}

/// The encode half never fails in practice — the messages are plain data —
/// but `postcard` says `Result`, and inventing an `unwrap` here would be
/// trading an impossible error for an impossible panic.
pub fn encode_client(message: &ClientMessage) -> Result<Vec<u8>> {
    postcard::to_allocvec(message).map_err(|e| SyncError::Wire(e.to_string()))
}

pub fn decode_client(bytes: &[u8]) -> Result<ClientMessage> {
    postcard::from_bytes(bytes).map_err(|e| SyncError::Wire(e.to_string()))
}

pub fn encode_server(message: &ServerMessage) -> Result<Vec<u8>> {
    postcard::to_allocvec(message).map_err(|e| SyncError::Wire(e.to_string()))
}

pub fn decode_server(bytes: &[u8]) -> Result<ServerMessage> {
    postcard::from_bytes(bytes).map_err(|e| SyncError::Wire(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::FamilyKey;

    fn family() -> FamilyId {
        FamilyKey::generate().unwrap().id()
    }

    #[test]
    fn client_messages_round_trip() {
        let messages = [
            ClientMessage::Hello {
                protocol: PROTOCOL,
                family: family(),
                epoch: 7,
                since: 42,
            },
            ClientMessage::Push {
                kind: FrameKind::Delta,
                payload: vec![1, 2, 3],
            },
            ClientMessage::Push {
                kind: FrameKind::Snapshot { covers: 9000 },
                payload: vec![],
            },
        ];
        for message in messages {
            let wire = encode_client(&message).unwrap();
            assert_eq!(decode_client(&wire).unwrap(), message);
        }
    }

    #[test]
    fn server_messages_round_trip() {
        let messages = [
            ServerMessage::Welcome { epoch: 3 },
            ServerMessage::Frame {
                seq: 12,
                kind: FrameKind::Delta,
                payload: vec![0xff; 64],
            },
            ServerMessage::CaughtUp,
            ServerMessage::Ack { seq: 13 },
            ServerMessage::Refused {
                reason: "speak protocol 1".to_string(),
            },
        ];
        for message in messages {
            let wire = encode_server(&message).unwrap();
            assert_eq!(decode_server(&wire).unwrap(), message);
        }
    }

    #[test]
    fn garbage_is_a_wire_error_not_a_panic() {
        assert!(matches!(
            decode_client(&[0xde, 0xad]),
            Err(SyncError::Wire(_))
        ));
        assert!(matches!(decode_server(&[]), Err(SyncError::Wire(_))));
    }
}
