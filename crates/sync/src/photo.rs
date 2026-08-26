//! The photo protocol: a second socket, and a conversation the device drives.
//!
//! Photos travel on `/photos`, never on `/sync`, and carry a protocol byte of
//! their own — so [`crate::protocol::PROTOCOL`] never moves for them and a
//! phone left in a pocket keeps converging, without photos (DECISIONS 0062,
//! 0080). The two protocols share the group key and the port, and nothing
//! else: no cursor, no epoch, no sequence number.
//!
//! Four facts shape it, and not one of them is the log's:
//!
//! - **A photo is a blob under a name, not an event in a sequence.** There is
//!   no order to preserve — two devices fetching the same photos in different
//!   orders are both right — so what replaces the cursor is **one round
//!   trip**: the hello says what this device holds and what it wants, and the
//!   welcome answers with what to send and what can be had.
//! - **Nothing moves unbidden.** After the welcome every message carries one
//!   photo, and it carries it because the device asked for it or offered it.
//!   A relay that answered the welcome by streaming would hand a phone that
//!   has just joined a group — the case that wants the entire library — tens
//!   of megabytes to buffer at once, and no way to slow it down (Rule 6).
//! - **The device decides when the conversation is over**, because it is the
//!   only party that knows what it still wants. There is no `CaughtUp` here:
//!   the welcome states the whole of the work, and the socket closes when the
//!   queue drains.
//! - **A name is plaintext, and it is the whole of what the relay learns.**
//!   It has to be: nothing can store and serve a blob it cannot name. A photo
//!   id is minted random and says nothing about the bytes — which is why 0062
//!   refused to hash the content — so a name in the clear is the same class
//!   of metadata as a [`crate::protocol::FrameKind`] on the log (0042). The
//!   bytes themselves are sealed by the device that took the photo, stored
//!   verbatim, and opened by the device that asked for them (Rule 7).
//!
//! One rule lives here rather than in the relay: **a [`PhotoName`] is checked
//! as it is decoded**. The relay names a file after one, the group id is the
//! only access control it has, and its port faces the internet (0012) — so a
//! `../` in a name is a write outside `/data`. Checking at the boundary
//! instead of at the use site is what makes forgetting impossible.
//!
//! A **cap is policy, not protocol**: how many bytes a group may store is the
//! relay's business (0062), and it surfaces here as
//! [`PhotoServerMessage::Rejected`] — against one photo, never against the
//! connection. A refusal that closed the socket would stop a queue that has
//! nothing wrong with it, and the reconnect would offer the same photo again.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{Result, SyncError};
use crate::key::GroupId;

/// Bumped on any incompatible change to the messages below. The relay
/// refuses a `Hello` carrying anything else.
///
/// Independent of [`crate::protocol::PROTOCOL`] on purpose: the two are
/// separate endpoints, and a change to one must not make a phone that only
/// speaks the other unable to sync (DECISIONS 0062).
pub const PHOTO_PROTOCOL: u8 = 1;

/// The most a name may weigh. Comfortably above what any device mints —
/// `app::id` produces a four-character prefix and sixteen hex digits — and
/// low enough that a hostile hello cannot spend the relay's memory on names.
const MAX_NAME_LEN: usize = 64;

/// What the relay calls a photo: the device's own id for it, and nothing
/// more.
///
/// A newtype rather than a `String` for one reason, and it is not tidiness:
/// the relay stores a blob in a file named after this, so a name containing
/// `/` or `..` is a write outside its data directory. The check is in
/// [`PhotoName::new`] **and in `Deserialize`**, which is what makes a hostile
/// name a [`SyncError::Wire`] before any relay code sees it.
///
/// It is deliberately not `cabas_domain::PhotoId`: this crate names no domain
/// type (Rule 7's boundary, and the same reason `GroupId` is its own), and
/// nothing here parses the id for meaning. A name is a token — it is compared
/// and it is written down.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhotoName(String);

impl PhotoName {
    /// Checks an id and takes it as a name, or says why it is not one.
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        check(&name)?;
        Ok(PhotoName(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One plain ASCII token: what every id in this workspace already is, and
/// the only shape safe to hand to a filesystem without escaping it.
fn check(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > MAX_NAME_LEN {
        return Err(SyncError::Wire(format!(
            "photo name: {} bytes, expected 1..={MAX_NAME_LEN}",
            name.len()
        )));
    }
    if !name
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(SyncError::Wire(
            "photo name: only letters, digits, '_' and '-'".to_string(),
        ));
    }
    Ok(())
}

impl Serialize for PhotoName {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for PhotoName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        check(&name).map_err(serde::de::Error::custom)?;
        Ok(PhotoName(name))
    }
}

impl std::fmt::Display for PhotoName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Device → relay.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum PhotoClientMessage {
    /// The opening message of every connection, and the whole of the
    /// reconciliation this device's side.
    ///
    /// `have` is every photo this device holds bytes for — offered rather
    /// than announced, since the relay keeps everything and only answers with
    /// what it lacks. `want` is what its replica references and it does not
    /// have: a device asks for what it can name, not for whatever the relay
    /// happens to hold, so a device that is behind on `/sync` does not fetch
    /// photos for entities it has never heard of (DECISIONS 0062).
    Hello {
        protocol: u8,
        group: GroupId,
        have: Vec<PhotoName>,
        want: Vec<PhotoName>,
    },

    /// One sealed photo, from the `upload` list the welcome answered with.
    /// The relay stores the payload verbatim and answers
    /// [`PhotoServerMessage::Stored`] or [`PhotoServerMessage::Rejected`].
    ///
    /// One photo per message, one message in flight: the pace is the device's
    /// (see the module note), and a queue is a queue rather than a burst.
    Push { id: PhotoName, payload: Vec<u8> },

    /// Asks for one photo the welcome listed as available.
    Fetch { id: PhotoName },
}

/// Relay → device.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum PhotoServerMessage {
    /// The answer to the hello, and the only unsolicited message there is:
    /// the reconciliation, in one round trip.
    ///
    /// `upload` is the subset of `have` the relay does not hold — the rest is
    /// already safe and must not be sent again. `available` is the subset of
    /// `want` it does hold, which is what tells the device up front how much
    /// work there is, and which of the photos it is missing nobody has yet
    /// (they are on a phone that has not connected since).
    Welcome {
        upload: Vec<PhotoName>,
        available: Vec<PhotoName>,
    },

    /// The sealed bytes of a fetched photo, exactly as the device that took
    /// it pushed them. Only a device holding the group key can open it.
    Photo { id: PhotoName, payload: Vec<u8> },

    /// The relay does not hold this photo after all — it was forgotten
    /// between the welcome and the fetch (DECISIONS 0050 is by hand, and a
    /// hand can run during a connection). An answer rather than silence,
    /// because a device waiting on bytes that will never arrive would stall
    /// its queue for good.
    Absent { id: PhotoName },

    /// A pushed photo is durable. The device may stop offering it.
    Stored { id: PhotoName },

    /// This photo was not stored, and pushing it again now would not help:
    /// the group is at its byte cap, or the blob is beyond what one photo may
    /// weigh. Against the photo and not the connection — the rest of the
    /// queue is still worth transferring (see the module note).
    Rejected { id: PhotoName, reason: String },

    /// The connection is over and this says why: a protocol mismatch, an
    /// unparsable message. Sent once, then the socket closes — the same
    /// courtesy `/sync` extends, for the same reason (0042).
    Refused { reason: String },
}

/// The encode half never fails in practice — the messages are plain data —
/// but `postcard` says `Result`, and inventing an `unwrap` here would be
/// trading an impossible error for an impossible panic.
pub fn encode_client(message: &PhotoClientMessage) -> Result<Vec<u8>> {
    postcard::to_allocvec(message).map_err(|e| SyncError::Wire(e.to_string()))
}

pub fn decode_client(bytes: &[u8]) -> Result<PhotoClientMessage> {
    postcard::from_bytes(bytes).map_err(|e| SyncError::Wire(e.to_string()))
}

pub fn encode_server(message: &PhotoServerMessage) -> Result<Vec<u8>> {
    postcard::to_allocvec(message).map_err(|e| SyncError::Wire(e.to_string()))
}

pub fn decode_server(bytes: &[u8]) -> Result<PhotoServerMessage> {
    postcard::from_bytes(bytes).map_err(|e| SyncError::Wire(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::GroupKey;

    fn group() -> GroupId {
        GroupKey::generate().unwrap().id()
    }

    fn name(raw: &str) -> PhotoName {
        PhotoName::new(raw).unwrap()
    }

    /// [`PhotoClientMessage`] with the check taken out — which is exactly
    /// what an attacker's encoder is. The variants are in the same order, so
    /// postcard gives them the same discriminants and the relay's decoder
    /// cannot tell the two apart until it looks at a name.
    #[derive(Serialize)]
    #[allow(dead_code)]
    enum Unchecked {
        Hello {
            protocol: u8,
            group: GroupId,
            have: Vec<String>,
            want: Vec<String>,
        },
        Push {
            id: String,
            payload: Vec<u8>,
        },
        Fetch {
            id: String,
        },
    }

    #[test]
    fn client_messages_round_trip() {
        let messages = [
            PhotoClientMessage::Hello {
                protocol: PHOTO_PROTOCOL,
                group: group(),
                have: vec![name("pho_0000000000000001")],
                want: vec![name("pho_0000000000000002"), name("pho_0000000000000003")],
            },
            PhotoClientMessage::Hello {
                protocol: PHOTO_PROTOCOL,
                group: group(),
                have: vec![],
                want: vec![],
            },
            PhotoClientMessage::Push {
                id: name("pho_0000000000000004"),
                payload: vec![0xff; 128],
            },
            PhotoClientMessage::Fetch {
                id: name("pho_0000000000000005"),
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
            PhotoServerMessage::Welcome {
                upload: vec![name("pho_0000000000000001")],
                available: vec![name("pho_0000000000000002")],
            },
            PhotoServerMessage::Photo {
                id: name("pho_0000000000000002"),
                payload: vec![1, 2, 3],
            },
            PhotoServerMessage::Absent {
                id: name("pho_0000000000000006"),
            },
            PhotoServerMessage::Stored {
                id: name("pho_0000000000000001"),
            },
            PhotoServerMessage::Rejected {
                id: name("pho_0000000000000007"),
                reason: "this group is at its byte cap".to_string(),
            },
            PhotoServerMessage::Refused {
                reason: format!("speak photo protocol {PHOTO_PROTOCOL}"),
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

    #[test]
    fn what_a_device_mints_is_a_name() {
        // The shape `app::id` produces: a prefix and sixteen hex digits.
        // Nothing here parses it — this is the promise the other end keeps.
        assert!(PhotoName::new("pho_0123456789abcdef").is_ok());
        assert_eq!(
            name("pho_0123456789abcdef").to_string(),
            "pho_0123456789abcdef"
        );
    }

    #[test]
    fn a_name_is_one_plain_ascii_token() {
        for hostile in [
            "",
            "..",
            "../../etc/passwd",
            "pho_0123/../..",
            "pho_0123456789abcdef/",
            "pho 0123456789abcdef",
            "pho_0123456789abcdéf",
            "pho_0123456789abcdef\0",
            ".",
        ] {
            assert!(
                PhotoName::new(hostile).is_err(),
                "{hostile:?} was accepted as a photo name"
            );
        }
        assert!(PhotoName::new("a".repeat(MAX_NAME_LEN)).is_ok());
        assert!(PhotoName::new("a".repeat(MAX_NAME_LEN + 1)).is_err());
    }

    /// The check that has to hold on the *wire*, not just at the constructor:
    /// the relay decodes bytes a stranger who found the group id can send,
    /// and it names a file after what comes out (see the module note).
    #[test]
    fn a_hostile_name_does_not_survive_decoding() {
        let wire = postcard::to_allocvec(&Unchecked::Fetch {
            id: "../../../data/00112233445566778899aabbccddeeff/log".to_string(),
        })
        .unwrap();

        assert!(
            matches!(decode_client(&wire), Err(SyncError::Wire(_))),
            "a traversing name reached the relay's file naming"
        );
    }

    /// A name inside a list is checked too — the hello carries two of them,
    /// and a check that only ran on the single-name variants would leave the
    /// one message every connection starts with unguarded.
    #[test]
    fn a_hostile_name_in_a_list_does_not_survive_either() {
        let wire = postcard::to_allocvec(&Unchecked::Hello {
            protocol: PHOTO_PROTOCOL,
            group: group(),
            have: vec!["pho_0123456789abcdef".to_string(), "../meta".to_string()],
            want: vec![],
        })
        .unwrap();

        assert!(matches!(decode_client(&wire), Err(SyncError::Wire(_))));
    }
}
