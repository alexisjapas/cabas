//! The client half of a photo connection, with no socket in sight.
//!
//! Sans-IO for the reason [`crate::Session`] is (DECISIONS 0043): the PWA
//! speaks WebSocket through the browser's own API from the frontend, the
//! native hosts through `tokio-tungstenite`, and everything the two would
//! otherwise write twice — sealing, opening, and the discipline of what may
//! be stored — lives here as plain synchronous calls on bytes.
//!
//! It is [`crate::photo`]'s conversation, held from the device's end. What
//! makes it a different shape from [`crate::Session`] is that **a photo
//! library has no order** (DECISIONS 0080): there is no cursor to advance and
//! nothing to replay, only two queues that the welcome fills once and that
//! drain. The connection is over when they are empty, and that is a fact only
//! this side can compute.
//!
//! # What it will not do
//!
//! **It stores nothing it did not ask for.** An incoming photo is checked
//! against the `want` this session was opened with *before* it is opened, so
//! a relay answering a fetch with something else spends neither this device's
//! storage nor its cipher. That is the one place the untrusted party in Rule 7
//! could otherwise write bytes onto a phone, and the check is two lines.
//!
//! The caller owns the socket, the pace and the store. This type never sees a
//! replica, a `PhotoStore` or a filesystem; it turns wire bytes into "store
//! these bytes under this name" and local bytes into sealed pushes.

use std::collections::VecDeque;

use crate::error::{Result, SyncError};
use crate::key::GroupKey;
use crate::photo::{self, PHOTO_PROTOCOL, PhotoClientMessage, PhotoName, PhotoServerMessage};
use crate::seal;

/// What the caller must do with a server message, once the session has
/// opened, checked and accounted for it.
#[derive(Debug, PartialEq, Eq)]
pub enum PhotoEvent {
    /// The relay answered the hello, and the whole of the work is now known:
    /// `upload` photos to send and `download` to ask for, counted from the
    /// device's end. Everything after this is one of those, one message at a
    /// time.
    Reconciled { upload: usize, download: usize },

    /// Sealed bytes opened — write them under this name. The device asked for
    /// them, so its replica already references them.
    Store { id: PhotoName, bytes: Vec<u8> },

    /// The relay does not hold this photo after all: a `forget` (0050) is run
    /// by a person, and a hand can run between the welcome and the fetch. The
    /// photo is on a device that has not connected since, or on none.
    Absent { id: PhotoName },

    /// A pushed photo is durable on the relay. This device may stop being its
    /// only copy.
    Sent { id: PhotoName },

    /// A pushed photo was not stored and pushing it again now would not help
    /// — the group is at its byte cap, or the blob is beyond what one photo
    /// may weigh. Against the photo and not the connection, so the rest of
    /// the queue is still worth transferring (DECISIONS 0080).
    Rejected { id: PhotoName, reason: String },

    /// A photo arrived and was not stored, for one of two reasons that ask
    /// the same thing of the caller — nothing.
    ///
    /// Either the group key refused to open it (corruption, or a stranger who
    /// found the group id, which the relay stores in the clear), or it was
    /// never asked for. `Session::Dropped` folds "corruption or company" into
    /// one event for the same reason: no caller acts differently, and the
    /// count is the part worth watching.
    Dropped { id: PhotoName },

    /// The relay hung up with a reason: a protocol mismatch, a message it
    /// could not parse. Reconnecting without changing something is pointless.
    Refused { reason: String },
}

/// One photo connection's worth of client state.
///
/// Built from what this device holds and what it lacks, driven with wire
/// bytes, and finished when [`PhotoSession::done`] says so. It owns no store:
/// both lists are handed in, and the bytes of an upload are handed in too.
pub struct PhotoSession {
    key: GroupKey,
    /// What was offered in the hello. The welcome may only name a subset of
    /// it back: a relay asking for a photo this device never claimed to hold
    /// would have the caller read a file that is not there.
    have: Vec<PhotoName>,
    /// What was asked for in the hello, and **the whole of what this session
    /// will store**. See the module note.
    want: Vec<PhotoName>,
    upload: VecDeque<PhotoName>,
    download: VecDeque<PhotoName>,
    /// Messages sent and not yet answered. There is one answer per push and
    /// per fetch, which is what makes "the queues are empty" mean "the work
    /// is finished" rather than "the work is all in flight".
    outstanding: usize,
    welcomed: bool,
    sent: u64,
    opened: u64,
    dropped: u64,
}

impl PhotoSession {
    /// `have` is every photo this device holds bytes for; `want` is what its
    /// replica references and it lacks.
    ///
    /// Both are sorted and deduplicated here rather than at each caller: it
    /// makes the hello a function of the two sets and not of the order a
    /// store happened to list them in, which is one less thing to hold still
    /// in a test and one less way for two devices to look different in a
    /// relay's logs.
    pub fn new(key: GroupKey, have: Vec<PhotoName>, want: Vec<PhotoName>) -> Self {
        let tidy = |mut names: Vec<PhotoName>| {
            names.sort();
            names.dedup();
            names
        };
        PhotoSession {
            key,
            have: tidy(have),
            want: tidy(want),
            upload: VecDeque::new(),
            download: VecDeque::new(),
            outstanding: 0,
            welcomed: false,
            sent: 0,
            opened: 0,
            dropped: 0,
        }
    }

    /// The opening message, and the whole of the reconciliation this device's
    /// side. Send it, then feed every incoming binary message to
    /// [`PhotoSession::handle`].
    pub fn hello(&self) -> Result<Vec<u8>> {
        photo::encode_client(&PhotoClientMessage::Hello {
            protocol: PHOTO_PROTOCOL,
            group: self.key.id(),
            have: self.have.clone(),
            want: self.want.clone(),
        })
    }

    /// One incoming binary message → one instruction to the caller.
    pub fn handle(&mut self, wire: &[u8]) -> Result<PhotoEvent> {
        match photo::decode_server(wire)? {
            PhotoServerMessage::Welcome { upload, available } => {
                if self.welcomed {
                    // The welcome is what fills both queues, so a second one
                    // would re-queue work already done — and a relay that
                    // sent one every time would keep this socket open for
                    // good. There is exactly one round trip (0080).
                    return Err(SyncError::Wire("a second welcome".to_string()));
                }
                self.welcomed = true;
                // Narrowed to what the hello actually offered and asked for.
                // The relay is untrusted (Rule 7) and these two lists decide
                // what this device reads off its disk and what it will accept
                // onto it, so they are bounded by what it said itself rather
                // than by what came back.
                self.upload = upload
                    .into_iter()
                    .filter(|id| self.have.contains(id))
                    .collect();
                self.download = available
                    .into_iter()
                    .filter(|id| self.want.contains(id))
                    .collect();
                Ok(PhotoEvent::Reconciled {
                    upload: self.upload.len(),
                    download: self.download.len(),
                })
            }
            PhotoServerMessage::Photo { id, payload } => {
                if !self.want.contains(&id) {
                    // Never asked for: not opened, not counted against the
                    // queue, and above all not stored (see the module note).
                    self.dropped += 1;
                    return Ok(PhotoEvent::Dropped { id });
                }
                self.settle();
                match seal::open(&self.key, &payload) {
                    Ok(bytes) => {
                        self.opened += 1;
                        Ok(PhotoEvent::Store { id, bytes })
                    }
                    Err(SyncError::Open) => {
                        self.dropped += 1;
                        Ok(PhotoEvent::Dropped { id })
                    }
                    Err(other) => Err(other),
                }
            }
            PhotoServerMessage::Absent { id } => {
                self.settle();
                Ok(PhotoEvent::Absent { id })
            }
            PhotoServerMessage::Stored { id } => {
                self.settle();
                self.sent += 1;
                Ok(PhotoEvent::Sent { id })
            }
            PhotoServerMessage::Rejected { id, reason } => {
                self.settle();
                Ok(PhotoEvent::Rejected { id, reason })
            }
            PhotoServerMessage::Refused { reason } => Ok(PhotoEvent::Refused { reason }),
        }
    }

    /// The next photo to ask for, already encoded — or nothing left to ask
    /// for.
    ///
    /// Whole, unlike [`PhotoSession::offer`], because a fetch needs nothing
    /// from this device: the name is the entire message. Call it as often as
    /// the caller wants messages in flight; one at a time is a queue, and the
    /// pace is the device's (DECISIONS 0080).
    pub fn fetch(&mut self) -> Result<Option<Vec<u8>>> {
        let Some(id) = self.download.pop_front() else {
            return Ok(None);
        };
        self.outstanding += 1;
        Ok(Some(photo::encode_client(&PhotoClientMessage::Fetch {
            id,
        })?))
    }

    /// The next photo to upload, by name — or nothing left to upload.
    ///
    /// Two calls rather than one because the bytes live in a store this type
    /// deliberately cannot reach, and reading them is asynchronous on every
    /// host. The caller reads them and answers with [`PhotoSession::push`];
    /// a name whose bytes have gone — swept between the hello and now — is
    /// simply not pushed, and the queue moves on.
    pub fn offer(&mut self) -> Option<PhotoName> {
        self.upload.pop_front()
    }

    /// Seals the bytes read for [`PhotoSession::offer`]'s name into a push.
    ///
    /// The payload is sealed here and stored verbatim by the relay, opened
    /// only by a device holding the group key (Rule 7). Nothing caps it on
    /// this side: what one photo may weigh is the app's rule and what a group
    /// may weigh is the relay's, and both answer with words rather than with
    /// a dropped socket (DECISIONS 0062, 0080).
    pub fn push(&mut self, id: &PhotoName, plaintext: &[u8]) -> Result<Vec<u8>> {
        let payload = seal::seal(&self.key, plaintext)?;
        self.outstanding += 1;
        photo::encode_client(&PhotoClientMessage::Push {
            id: id.clone(),
            payload,
        })
    }

    /// Whether the conversation is finished: reconciled, both queues drained,
    /// and every message sent answered.
    ///
    /// The device's call and nobody else's — the relay cannot compute it for
    /// a party whose wants it cannot see (DECISIONS 0080) — and the caller
    /// closes the socket on it. A transfer cut short costs one round trip
    /// next time and nothing else, because the next hello re-derives the
    /// difference from what is actually on disk at both ends.
    pub fn done(&self) -> bool {
        self.welcomed && self.upload.is_empty() && self.download.is_empty() && self.outstanding == 0
    }

    /// Photos still to move: queued, plus sent and not yet answered. What a
    /// progress indicator counts down.
    pub fn pending(&self) -> usize {
        self.upload.len() + self.download.len() + self.outstanding
    }

    /// Photos opened and handed to the caller on this connection — which is
    /// one step short of stored: what happens to the bytes afterwards is the
    /// caller's business, and `app::photos` counts that separately.
    pub fn opened(&self) -> u64 {
        self.opened
    }

    /// Photos this device pushed and the relay confirmed durable.
    pub fn sent(&self) -> u64 {
        self.sent
    }

    /// Photos that arrived and were not handed over — they did not open, or
    /// they were never asked for. Nonzero is either corruption or company.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// One message answered. Saturating because the count is arithmetic over
    /// what a party on the other side of a socket chose to send: a relay
    /// answering twice must not make this session believe it owes a message
    /// it never sent.
    fn settle(&mut self) {
        self.outstanding = self.outstanding.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::photo::decode_client;

    const PHRASE: &str = "abandon abandon abandon abandon abandon abandon \
                          abandon abandon abandon abandon abandon about";

    fn key() -> GroupKey {
        GroupKey::from_phrase(PHRASE).unwrap()
    }

    fn name(raw: &str) -> PhotoName {
        PhotoName::new(raw).unwrap()
    }

    fn wire(message: &PhotoServerMessage) -> Vec<u8> {
        photo::encode_server(message).unwrap()
    }

    /// A session that has already been told what to do: one photo to upload,
    /// one to fetch.
    fn reconciled() -> PhotoSession {
        let mut session = PhotoSession::new(
            key(),
            vec![name("pho_0000000000000001")],
            vec![name("pho_0000000000000002")],
        );
        session
            .handle(&wire(&PhotoServerMessage::Welcome {
                upload: vec![name("pho_0000000000000001")],
                available: vec![name("pho_0000000000000002")],
            }))
            .unwrap();
        session
    }

    /// What the relay does with a push: it stores the payload verbatim and
    /// hands the same bytes back to whoever fetches it (DECISIONS 0080).
    fn relayed(session: &mut PhotoSession, id: &PhotoName, plaintext: &[u8]) -> Vec<u8> {
        match decode_client(&session.push(id, plaintext).unwrap()).unwrap() {
            PhotoClientMessage::Push { payload, .. } => wire(&PhotoServerMessage::Photo {
                id: id.clone(),
                payload,
            }),
            other => panic!("expected a push, got {other:?}"),
        }
    }

    #[test]
    fn the_hello_offers_what_is_held_and_asks_for_what_is_missing() {
        let group = key().id();
        let session = PhotoSession::new(
            key(),
            vec![name("pho_0000000000000002"), name("pho_0000000000000001")],
            vec![name("pho_0000000000000003")],
        );

        match decode_client(&session.hello().unwrap()).unwrap() {
            PhotoClientMessage::Hello {
                protocol,
                group: id,
                have,
                want,
            } => {
                assert_eq!(protocol, PHOTO_PROTOCOL);
                assert_eq!(id, group);
                assert_eq!(
                    have,
                    vec![name("pho_0000000000000001"), name("pho_0000000000000002")],
                    "the hello is a function of the two sets, not of the order \
                     a store listed them in"
                );
                assert_eq!(want, vec![name("pho_0000000000000003")]);
            }
            other => panic!("expected a hello, got {other:?}"),
        }
    }

    #[test]
    fn the_welcome_states_the_whole_of_the_work() {
        let mut session = PhotoSession::new(
            key(),
            vec![name("pho_0000000000000001"), name("pho_0000000000000004")],
            vec![name("pho_0000000000000002"), name("pho_0000000000000003")],
        );
        assert!(
            !session.done(),
            "nothing is finished before it is reconciled"
        );

        let event = session
            .handle(&wire(&PhotoServerMessage::Welcome {
                // One of the two offered is already on the relay, so only the
                // other comes back — the rest is safe and must not be resent.
                upload: vec![name("pho_0000000000000004")],
                available: vec![name("pho_0000000000000002")],
            }))
            .unwrap();

        assert_eq!(
            event,
            PhotoEvent::Reconciled {
                upload: 1,
                download: 1
            }
        );
        assert_eq!(session.pending(), 2);
        assert_eq!(
            session.offer(),
            Some(name("pho_0000000000000004")),
            "the photo the relay already holds is not offered again"
        );
        assert!(session.offer().is_none());
    }

    /// The welcome decides what this device reads off its disk and what it
    /// accepts onto it, and it comes from an untrusted party (Rule 7).
    #[test]
    fn a_welcome_cannot_widen_what_the_hello_said() {
        let mut session = PhotoSession::new(
            key(),
            vec![name("pho_0000000000000001")],
            vec![name("pho_0000000000000002")],
        );

        let event = session
            .handle(&wire(&PhotoServerMessage::Welcome {
                upload: vec![name("pho_0000000000000001"), name("pho_00000000000000ff")],
                available: vec![name("pho_0000000000000002"), name("pho_00000000000000ee")],
            }))
            .unwrap();

        assert_eq!(
            event,
            PhotoEvent::Reconciled {
                upload: 1,
                download: 1
            },
            "a name this device never offered or asked for is not work"
        );
    }

    #[test]
    fn a_second_welcome_is_a_wire_error() {
        let mut session = reconciled();
        assert!(matches!(
            session.handle(&wire(&PhotoServerMessage::Welcome {
                upload: vec![],
                available: vec![]
            })),
            Err(SyncError::Wire(_))
        ));
    }

    #[test]
    fn a_fetch_is_the_name_and_nothing_else() {
        let mut session = reconciled();
        let asked = session.fetch().unwrap().expect("one photo to fetch");
        match decode_client(&asked).unwrap() {
            PhotoClientMessage::Fetch { id } => assert_eq!(id, name("pho_0000000000000002")),
            other => panic!("expected a fetch, got {other:?}"),
        }
        assert!(session.fetch().unwrap().is_none());
        assert!(
            !session.done(),
            "a fetch sent is a fetch still owed an answer"
        );
    }

    #[test]
    fn a_push_seals_and_the_group_key_opens_it() {
        let mut session = reconciled();
        let id = session.offer().expect("one photo to upload");

        match decode_client(&session.push(&id, b"jpeg bytes").unwrap()).unwrap() {
            PhotoClientMessage::Push {
                id: pushed,
                payload,
            } => {
                assert_eq!(pushed, id);
                assert_eq!(seal::open(&key(), &payload).unwrap(), b"jpeg bytes");
            }
            other => panic!("expected a push, got {other:?}"),
        }
    }

    #[test]
    fn a_fetched_photo_opens_and_is_handed_over() {
        let mut session = reconciled();
        let id = name("pho_0000000000000002");
        // The bytes take the round trip they would really take: sealed by the
        // device that held them, stored verbatim, handed back on a fetch.
        let mut other_device = PhotoSession::new(key(), vec![id.clone()], vec![]);
        let arriving = relayed(&mut other_device, &id, b"the other phone's photo");

        session.fetch().unwrap();
        let event = session.handle(&arriving).unwrap();

        assert_eq!(
            event,
            PhotoEvent::Store {
                id,
                bytes: b"the other phone's photo".to_vec()
            }
        );
        assert_eq!(session.opened(), 1);
        assert_eq!(session.dropped(), 0);
    }

    #[test]
    fn a_photo_that_does_not_open_is_dropped_and_stepped_over() {
        let mut session = reconciled();
        session.fetch().unwrap();

        let event = session
            .handle(&wire(&PhotoServerMessage::Photo {
                id: name("pho_0000000000000002"),
                payload: vec![0; 64],
            }))
            .unwrap();

        assert_eq!(
            event,
            PhotoEvent::Dropped {
                id: name("pho_0000000000000002")
            }
        );
        assert_eq!(session.dropped(), 1);
        assert_eq!(
            session.pending(),
            1,
            "the fetch is answered — refetching it would not make it open"
        );
    }

    /// The one place an untrusted relay could otherwise write bytes onto a
    /// phone (see the module note).
    #[test]
    fn a_photo_nobody_asked_for_is_dropped_without_being_opened() {
        let mut session = reconciled();
        let unwanted = name("pho_00000000000000ff");
        let mut other_device = PhotoSession::new(key(), vec![unwanted.clone()], vec![]);
        // Sealed with the right key: what is wrong with it is that it was
        // never asked for, and that alone is enough.
        let arriving = relayed(&mut other_device, &unwanted, b"not on this device's list");

        let before = session.pending();
        let event = session.handle(&arriving).unwrap();

        assert_eq!(event, PhotoEvent::Dropped { id: unwanted });
        assert_eq!(session.opened(), 0);
        assert_eq!(session.dropped(), 1);
        assert_eq!(
            session.pending(),
            before,
            "an unbidden photo answers nothing, so it settles nothing"
        );
    }

    #[test]
    fn an_absent_photo_settles_its_place_in_the_queue() {
        let mut session = reconciled();
        session.fetch().unwrap();
        session.offer();

        let event = session
            .handle(&wire(&PhotoServerMessage::Absent {
                id: name("pho_0000000000000002"),
            }))
            .unwrap();

        assert_eq!(
            event,
            PhotoEvent::Absent {
                id: name("pho_0000000000000002")
            }
        );
        assert!(
            session.done(),
            "a photo nobody holds is not a photo this device waits for"
        );
    }

    #[test]
    fn a_rejection_is_against_the_photo_and_the_queue_survives_it() {
        let mut session = PhotoSession::new(
            key(),
            vec![name("pho_0000000000000001"), name("pho_0000000000000004")],
            vec![],
        );
        session
            .handle(&wire(&PhotoServerMessage::Welcome {
                upload: vec![name("pho_0000000000000001"), name("pho_0000000000000004")],
                available: vec![],
            }))
            .unwrap();

        let first = session.offer().expect("first");
        session.push(&first, b"one").unwrap();
        let event = session
            .handle(&wire(&PhotoServerMessage::Rejected {
                id: first,
                reason: "this group is at its byte cap".to_string(),
            }))
            .unwrap();

        assert!(matches!(event, PhotoEvent::Rejected { .. }));
        assert_eq!(session.sent(), 0);
        assert_eq!(
            session.offer(),
            Some(name("pho_0000000000000004")),
            "the rest of the queue has nothing wrong with it"
        );
    }

    #[test]
    fn the_conversation_is_over_when_both_queues_drain() {
        let mut session = reconciled();
        assert!(!session.done());

        let id = session.offer().expect("one to upload");
        session.push(&id, b"pixels").unwrap();
        session.fetch().unwrap();
        assert_eq!(session.pending(), 2);
        assert!(!session.done(), "both are in flight, neither is finished");

        session
            .handle(&wire(&PhotoServerMessage::Stored { id: id.clone() }))
            .unwrap();
        assert!(!session.done());

        session
            .handle(&wire(&PhotoServerMessage::Absent {
                id: name("pho_0000000000000002"),
            }))
            .unwrap();

        assert!(session.done());
        assert_eq!(session.pending(), 0);
        assert_eq!(session.sent(), 1);
    }

    #[test]
    fn a_refusal_passes_through() {
        let mut session = reconciled();
        assert_eq!(
            session
                .handle(&wire(&PhotoServerMessage::Refused {
                    reason: format!("speak photo protocol {PHOTO_PROTOCOL}"),
                }))
                .unwrap(),
            PhotoEvent::Refused {
                reason: format!("speak photo protocol {PHOTO_PROTOCOL}"),
            }
        );
    }

    #[test]
    fn garbage_from_the_wire_is_an_error_not_a_panic() {
        let mut session = reconciled();
        assert!(matches!(
            session.handle(&[0xba, 0xad]),
            Err(SyncError::Wire(_))
        ));
    }

    /// A relay answering more often than it was asked must not leave this
    /// session believing it owes a message it never sent.
    #[test]
    fn an_answer_to_nothing_does_not_take_the_count_below_zero() {
        let mut session = reconciled();
        for _ in 0..3 {
            session
                .handle(&wire(&PhotoServerMessage::Stored {
                    id: name("pho_0000000000000001"),
                }))
                .unwrap();
        }
        assert_eq!(
            session.pending(),
            2,
            "the two queued photos are still queued"
        );
    }
}
