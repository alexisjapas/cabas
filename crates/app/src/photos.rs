//! The photo library: the bytes the document only names.
//!
//! Shaped like [`crate::sync`] rather than folded into [`crate::App`]: it
//! holds its own store and is handed what the replica references when it
//! needs to know. Two reasons, and the second is the one that decided it —
//! the replica and the photos are written on completely different rhythms
//! (every tick in a shop against once per photo, DECISIONS 0062), and M7's
//! Tauri host will drive this exact composition over a directory instead of
//! over IndexedDB.
//!
//! Nothing here takes an `&App`, on purpose. A host with one thread keeps its
//! app in a `RefCell`, and every method below awaits — a borrow held across
//! one of those awaits is the second tap panicking in a shop (DECISIONS
//! 0032). Reading the replica and touching the store stay two statements.
//!
//! # What is enforced here, and why not in the frontend
//!
//! A photo arrives already downscaled and encoded, because a canvas is the
//! only image encoder a PWA has. What it is *allowed* to be is decided here:
//! one format and one ceiling, checked once, on the one path that stores
//! anything. A frontend that promised itself the same thing would be a second
//! implementation of a rule (Rule 9), and the failure it prevents is not a
//! rendering glitch — the bytes stored here are the only copy until the relay
//! has one, and nothing downstream re-reads them.
//!
//! # And the transfer, which is the other half of the same argument
//!
//! [`PhotoSync`] is [`cabas_sync::PhotoSession`] met with that store, exactly
//! as [`crate::SyncSession`] is [`cabas_sync::Session`] met with the replica:
//! reconciliation and sealing on that side, "and now write it" on this one.
//! It lives here rather than in [`crate::sync`] because it composes with a
//! [`Photos`] and never with an [`crate::App`] — which is also why a photo
//! transfer cannot delay a tick in a shop.

use serde::{Deserialize, Serialize};

use cabas_domain::PhotoId;
use cabas_store::PhotoStore;
use cabas_sync::{GroupKey, PhotoEvent as Incoming, PhotoName, PhotoSession};

use crate::error::{AppError, Result};
use crate::id;
use crate::platform::Platform;

/// The most a single photo may weigh.
///
/// Sized against what it is for rather than against what a phone could hold:
/// a picture that identifies a product on a shelf is legible well under this,
/// and every byte over it is paid on the relay, in a Home Assistant backup,
/// and in the transfer to the other phone (DECISIONS 0062). The frontend's
/// encoder targets it; this is where it is true.
pub const MAX_PHOTO_BYTES: usize = 512 * 1024;

/// What every photo is, and the only thing this accepts.
///
/// A JPEG starts `FF D8 FF`. The check is three bytes and it catches the one
/// mistake that would otherwise be discovered on the *other* phone, weeks
/// later: a host handing over a PNG, a HEIC straight off the camera, or a
/// truncated encode. There is no re-encoding here — refusing is the whole
/// response, because the caller is the only party that can encode.
const JPEG_MAGIC: [u8; 3] = [0xff, 0xd8, 0xff];

/// This device's photos.
pub struct Photos<F: PhotoStore> {
    store: F,
}

impl<F: PhotoStore> Photos<F> {
    pub fn new(store: F) -> Self {
        Self { store }
    }

    /// Stores a photo and hands back the id to attach to an entity.
    ///
    /// The id is minted here — random, never derived from the bytes: the
    /// relay ends up holding the ciphertext, and an id that said something
    /// about the plaintext would let it confirm a guessed photo (Rule 7,
    /// DECISIONS 0062).
    ///
    /// Storing is deliberately separate from attaching. The id goes on an
    /// [`crate::command::IngredientInput`] or a
    /// [`crate::command::RecipeInput`] and is saved by the command that
    /// already exists, which keeps `apply` synchronous — a photo write is a
    /// browser transaction, and no render may wait on one (Rule 6).
    pub async fn put(&self, platform: &impl Platform, bytes: &[u8]) -> Result<PhotoId> {
        acceptable(bytes)?;
        let id = PhotoId::from_raw(id::mint(platform, id::PHOTO)?);
        self.store.save(&id, bytes).await?;
        Ok(id)
    }

    /// Stores a photo **under an id minted somewhere else**.
    ///
    /// The one thing [`Photos::put`] cannot do, and the two callers that need
    /// it both have the same shape: the document already names this photo, so
    /// the bytes have to arrive under the name it uses. Today that is an
    /// imported file (DECISIONS 0076); it is also exactly what M10's transfer
    /// half will do with a photo fetched from the relay.
    ///
    /// Accepting a foreign id is safe for the reason ids are random rather
    /// than derived (see [`crate::id`]): 64 bits minted on another device
    /// cannot collide with one minted here. What is *not* taken on trust is
    /// the content — the same JPEG check and the same ceiling apply, because
    /// this is the path bytes take when they came from somewhere.
    pub async fn restore(&self, id: &PhotoId, bytes: &[u8]) -> Result<()> {
        acceptable(bytes)?;
        self.store.save(id, bytes).await?;
        Ok(())
    }

    /// The bytes of a photo this device holds.
    ///
    /// `None` is an ordinary answer and not a failure: a photo taken on the
    /// other phone is referenced by the document from the moment the two
    /// replicas merge, and its bytes arrive later on their own socket. The
    /// screen shows a placeholder, never an error (Rule 6).
    pub async fn get(&self, id: &PhotoId) -> Result<Option<Vec<u8>>> {
        Ok(self.store.load(id).await?)
    }

    /// Every id this device holds.
    pub async fn held(&self) -> Result<Vec<PhotoId>> {
        Ok(self.store.ids().await?)
    }

    /// What the replica references and this device does not have — the list
    /// the transfer half of M10 asks the relay for, in order.
    ///
    /// `referenced` comes from [`crate::App::referenced_photos`] rather than
    /// from an `&App` argument — see the module note.
    pub async fn missing(&self, referenced: &[PhotoId]) -> Result<Vec<PhotoId>> {
        let held = self.store.ids().await?;
        let mut missing: Vec<PhotoId> = referenced
            .iter()
            .filter(|id| !held.contains(id))
            .cloned()
            .collect();
        missing.sort();
        missing.dedup();
        Ok(missing)
    }

    /// Drops this device's copy of every photo the replica no longer
    /// references, and says which.
    ///
    /// Safe only because the relay keeps its own sealed copy and never
    /// deletes one (DECISIONS 0062): if the reference comes back — an undo on
    /// the other phone, a merge that arrives late — the bytes are fetched
    /// again. **Until the transfer half exists, a device's copy is the only
    /// copy**, and calling this would be a deletion rather than a cleanup.
    pub async fn forget_unreferenced(&self, referenced: &[PhotoId]) -> Result<Vec<PhotoId>> {
        let mut forgotten = Vec::new();
        for id in self.store.ids().await? {
            if !referenced.contains(&id) {
                self.store.remove(&id).await?;
                forgotten.push(id);
            }
        }
        Ok(forgotten)
    }
}

/// One format and one ceiling, checked on every path that stores anything.
///
/// It lives in one function rather than at each call site for the reason the
/// module note gives: a second implementation of the rule is a second thing
/// to forget, and the bytes here are the only copy until the relay has one.
fn acceptable(bytes: &[u8]) -> Result<()> {
    if bytes.len() < JPEG_MAGIC.len() || bytes[..JPEG_MAGIC.len()] != JPEG_MAGIC {
        return Err(AppError::invalid(
            "photo",
            "not a JPEG — the encoder handed over something else",
        ));
    }
    if bytes.len() > MAX_PHOTO_BYTES {
        return Err(AppError::invalid(
            "photo",
            format!(
                "{} kB is over the {} kB a photo may weigh",
                bytes.len() / 1024,
                MAX_PHOTO_BYTES / 1024
            ),
        ));
    }
    Ok(())
}

// --- the transfer (DECISIONS 0080) ------------------------------------------

/// One photo message, after this device's store has been brought up to date
/// with it.
///
/// Tagged the way [`crate::SyncEvent`] is, because the same kind of host
/// switches on it: the tag is `event`, the variants are `snake_case`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum PhotoEvent {
    /// The relay answered the hello and the whole of the work is known: this
    /// many photos to send, this many to fetch. Everything after it is one of
    /// those.
    Reconciled { upload: usize, download: usize },

    /// A photo arrived and **is on this device now**. The screen showing a
    /// placeholder for it can ask for it again and get bytes (Rule 6).
    Received { id: String },

    /// A photo this device held is durable on the relay. It is no longer the
    /// only copy.
    Sent { id: String },

    /// The relay does not hold this photo: it is on a device that has not
    /// connected since, or on none at all.
    Absent { id: String },

    /// The relay refused to store this photo — its byte cap, or its message
    /// size. Against the photo and not the connection, so the rest of the
    /// queue still goes (DECISIONS 0080).
    Rejected { id: String, reason: String },

    /// Something arrived and was not stored. Not fatal and not worth
    /// retrying: it did not open, it was never asked for, or it opened to
    /// something that is not a photo.
    Dropped { id: String },

    /// The relay hung up with a reason. Reconnecting without changing
    /// something will not help.
    Refused { reason: String },
}

/// Where one photo connection has got to.
///
/// Nothing here is persisted, which is the whole difference from
/// [`crate::SyncStatus`]: a photo transfer has no cursor to remember, because
/// the next hello re-derives the work from what is actually on disk at both
/// ends (DECISIONS 0080). It is read to draw progress and to decide when to
/// close the socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhotoStatus {
    /// Photos still to move: queued, plus sent and not yet answered.
    pub pending: usize,
    /// Photos written to this device's store on this connection.
    pub received: u64,
    /// Photos the relay confirmed durable on this connection.
    pub sent: u64,
    /// Photos that arrived and were not stored, for any of the three reasons
    /// [`PhotoEvent::Dropped`] names.
    pub dropped: u64,
    /// The queues have drained and every message is answered — close the
    /// socket.
    pub done: bool,
}

/// One photo connection's worth of client state, met with this device's
/// store.
///
/// [`cabas_sync::PhotoSession`] decides what to offer, what to ask for and
/// what may be stored, and stops there — it never sees a store (DECISIONS
/// 0080). This is the other half: what those instructions mean to a
/// [`Photos`]. It exists so the PWA's engine and M7's Tauri host share one
/// answer to "a photo arrived, now what" instead of writing it twice, which
/// is the same argument [`crate::sync`] makes for the log (DECISIONS 0043).
///
/// It holds no [`Photos`] of its own: the store is passed in on the calls
/// that need it, for the reason the module note gives — every one of those
/// calls awaits, and a host with one thread must not be holding a borrow
/// while it does (DECISIONS 0032).
///
/// The loop, in the order a connection runs it:
///
/// ```text
/// socket opens      → hello()                → send
/// every message     → handle(photos, wire)   → render what it says
/// whenever idle     → fetch() / push(photos) → send, one at a time
/// done()            → close the socket
/// ```
pub struct PhotoSync {
    inner: PhotoSession,
    /// Photos written on this connection. Counted here rather than read off
    /// the session because the session's count stops at "opened": a payload
    /// that opens and is not a photo is stored by neither of us, and only
    /// this side finds out.
    received: u64,
    /// Photos that opened to something [`Photos::restore`] refused. Added to
    /// the session's own count of what never opened.
    unstorable: u64,
}

impl PhotoSync {
    /// Opens a transfer for what this device holds and what its replica
    /// names but it lacks.
    ///
    /// `referenced` is [`crate::App::referenced_photos`], read in a statement
    /// that ends before this one is awaited. The phrase is the group's, and
    /// the only failure is one that does not decode — the same failure, with
    /// the same message, that [`crate::SyncSession::open`] gives (0021).
    pub async fn open<F: PhotoStore>(
        phrase: &str,
        photos: &Photos<F>,
        referenced: &[PhotoId],
    ) -> Result<Self> {
        let key = GroupKey::from_phrase(phrase)?;
        let have = speakable(&photos.held().await?);
        let want = speakable(&photos.missing(referenced).await?);
        Ok(PhotoSync {
            inner: PhotoSession::new(key, have, want),
            received: 0,
            unstorable: 0,
        })
    }

    /// The opening message: send it as soon as the socket is open, then feed
    /// every binary message to [`PhotoSync::handle`].
    pub fn hello(&self) -> Result<Vec<u8>> {
        Ok(self.inner.hello()?)
    }

    /// One incoming message, applied — which for a photo means written.
    ///
    /// A payload that opens is still not necessarily a photo: it crossed a
    /// relay, and it was sealed by whatever version of this app the other
    /// device runs. [`Photos::restore`] re-runs the format and the ceiling on
    /// it for that reason, and a payload that fails them is **dropped, not
    /// raised** — the queue behind it has nothing wrong with it, which is the
    /// same trade [`PhotoEvent::Rejected`] makes in the other direction. A
    /// store that fails to write *is* raised: that is this device breaking,
    /// not the group.
    pub async fn handle<F: PhotoStore>(
        &mut self,
        photos: &Photos<F>,
        wire: &[u8],
    ) -> Result<PhotoEvent> {
        Ok(match self.inner.handle(wire)? {
            Incoming::Reconciled { upload, download } => {
                PhotoEvent::Reconciled { upload, download }
            }
            Incoming::Store { id, bytes } => {
                match photos
                    .restore(&PhotoId::from_raw(id.as_str()), &bytes)
                    .await
                {
                    Ok(()) => {
                        self.received += 1;
                        PhotoEvent::Received { id: id.to_string() }
                    }
                    Err(AppError::Invalid { .. }) => {
                        self.unstorable += 1;
                        PhotoEvent::Dropped { id: id.to_string() }
                    }
                    Err(other) => return Err(other),
                }
            }
            Incoming::Sent { id } => PhotoEvent::Sent { id: id.to_string() },
            Incoming::Absent { id } => PhotoEvent::Absent { id: id.to_string() },
            Incoming::Rejected { id, reason } => PhotoEvent::Rejected {
                id: id.to_string(),
                reason,
            },
            Incoming::Dropped { id } => PhotoEvent::Dropped { id: id.to_string() },
            Incoming::Refused { reason } => PhotoEvent::Refused { reason },
        })
    }

    /// The next photo to ask for, encoded — or nothing left to ask for.
    pub fn fetch(&mut self) -> Result<Option<Vec<u8>>> {
        Ok(self.inner.fetch()?)
    }

    /// The next photo to offer, read from the store and sealed — or nothing
    /// left to offer.
    ///
    /// A photo whose bytes have gone since the hello — swept, or a store
    /// cleared under the app — is skipped rather than raised. The relay asked
    /// for it because this device offered it, and the offer is a snapshot of
    /// a disk that is allowed to move.
    pub async fn push<F: PhotoStore>(&mut self, photos: &Photos<F>) -> Result<Option<Vec<u8>>> {
        while let Some(name) = self.inner.offer() {
            if let Some(bytes) = photos.get(&PhotoId::from_raw(name.as_str())).await? {
                return Ok(Some(self.inner.push(&name, &bytes)?));
            }
        }
        Ok(None)
    }

    /// Whether the socket has done its work and may be closed.
    pub fn done(&self) -> bool {
        self.inner.done()
    }

    /// Where this connection has got to.
    pub fn status(&self) -> PhotoStatus {
        PhotoStatus {
            pending: self.inner.pending(),
            received: self.received,
            sent: self.inner.sent(),
            dropped: self.inner.dropped() + self.unstorable,
            done: self.inner.done(),
        }
    }
}

/// The ids that can be spoken on the wire, and only those.
///
/// A [`PhotoId`] is opaque and a document takes whatever another device wrote
/// onto an ingredient — including whatever an imported file said (DECISIONS
/// 0076) — while a `PhotoName` is a checked token, because the relay names a
/// file after one and its port faces the internet (DECISIONS 0080).
///
/// `PhotoStore` refuses the same shapes, so bytes under such an id were never
/// stored here and never will be; what this stops is different, and it is on
/// the asking side. One malformed reference in a library would otherwise fail
/// [`PhotoSync::open`] and take every other photo's transfer down with it,
/// permanently and for a reason nobody would find. It is left out instead.
fn speakable(ids: &[PhotoId]) -> Vec<PhotoName> {
    ids.iter()
        .filter_map(|id| PhotoName::new(id.as_str()).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cabas_store::MemoryPhotoStore;

    /// The smallest thing that passes for a JPEG here.
    fn jpeg(tail: &[u8]) -> Vec<u8> {
        let mut bytes = JPEG_MAGIC.to_vec();
        bytes.extend_from_slice(tail);
        bytes
    }

    struct Sequence(std::cell::Cell<u64>);

    impl Platform for Sequence {
        fn now(&self) -> cabas_domain::Timestamp {
            cabas_domain::Timestamp(0)
        }

        fn random_u64(&self) -> Result<u64> {
            self.0.set(self.0.get() + 1);
            Ok(self.0.get())
        }
    }

    fn block_on<T: std::future::Future>(future: T) -> T::Output {
        use std::pin::pin;
        use std::task::{Context, Poll, Waker};

        let mut future = pin!(future);
        let mut context = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
                return value;
            }
        }
    }

    #[test]
    fn a_stored_photo_comes_back_under_the_id_it_was_given() {
        let photos = Photos::new(MemoryPhotoStore::new());
        let platform = Sequence(std::cell::Cell::new(0));

        let id = block_on(photos.put(&platform, &jpeg(b"pixels"))).expect("put");
        assert!(id.as_str().starts_with(id::PHOTO));
        assert_eq!(
            block_on(photos.get(&id)).expect("get"),
            Some(jpeg(b"pixels"))
        );
    }

    #[test]
    fn two_photos_never_share_an_id_even_with_identical_bytes() {
        // The id is minted, not derived from the content (Rule 7).
        let photos = Photos::new(MemoryPhotoStore::new());
        let platform = Sequence(std::cell::Cell::new(0));

        let one = block_on(photos.put(&platform, &jpeg(b"same"))).expect("put");
        let two = block_on(photos.put(&platform, &jpeg(b"same"))).expect("put");
        assert_ne!(one, two);
    }

    #[test]
    fn a_photo_over_the_ceiling_is_refused_and_not_stored() {
        let photos = Photos::new(MemoryPhotoStore::new());
        let platform = Sequence(std::cell::Cell::new(0));

        let too_big = jpeg(&vec![0u8; MAX_PHOTO_BYTES]);
        let error = block_on(photos.put(&platform, &too_big)).expect_err("refused");
        assert!(matches!(error, AppError::Invalid { field: "photo", .. }));
        assert!(block_on(photos.held()).expect("held").is_empty());
    }

    #[test]
    fn anything_that_is_not_a_jpeg_is_refused() {
        let photos = Photos::new(MemoryPhotoStore::new());
        let platform = Sequence(std::cell::Cell::new(0));

        // A PNG's magic, an empty file, and three bytes of nothing.
        for bytes in [
            vec![0x89, b'P', b'N', b'G'],
            Vec::new(),
            vec![0x00, 0x00, 0x00],
        ] {
            assert!(block_on(photos.put(&platform, &bytes)).is_err());
        }
        assert!(block_on(photos.held()).expect("held").is_empty());
    }

    // --- the transfer (DECISIONS 0080) --------------------------------------

    const PHRASE: &str = "abandon abandon abandon abandon abandon abandon \
                          abandon abandon abandon abandon abandon about";

    fn photo_id(raw: &str) -> PhotoId {
        PhotoId::from_raw(raw)
    }

    fn wire(message: &cabas_sync::photo::PhotoServerMessage) -> Vec<u8> {
        cabas_sync::photo::encode_server(message).unwrap()
    }

    /// A relay's whole part in this: it stores a pushed payload verbatim and
    /// hands the same bytes back to whoever fetches the name (DECISIONS 0080).
    /// Driven through a second device, because sealing lives in `cabas-sync`
    /// and nothing here may reach it (Rule 7).
    fn sealed_by_the_other_phone(id: &str, bytes: &[u8]) -> Vec<u8> {
        let name = cabas_sync::PhotoName::new(id).unwrap();
        let mut other = cabas_sync::PhotoSession::new(
            cabas_sync::GroupKey::from_phrase(PHRASE).unwrap(),
            vec![name.clone()],
            vec![],
        );
        match cabas_sync::photo::decode_client(&other.push(&name, bytes).unwrap()).unwrap() {
            cabas_sync::photo::PhotoClientMessage::Push { payload, .. } => {
                wire(&cabas_sync::photo::PhotoServerMessage::Photo { id: name, payload })
            }
            other => panic!("expected a push, got {other:?}"),
        }
    }

    /// The welcome that puts one photo in each queue.
    fn welcome(upload: &[&str], available: &[&str]) -> Vec<u8> {
        let names = |ids: &[&str]| {
            ids.iter()
                .map(|id| cabas_sync::PhotoName::new(*id).unwrap())
                .collect()
        };
        wire(&cabas_sync::photo::PhotoServerMessage::Welcome {
            upload: names(upload),
            available: names(available),
        })
    }

    #[test]
    fn a_transfer_offers_what_is_on_disk_and_asks_for_what_the_replica_names() {
        let photos = Photos::new(MemoryPhotoStore::new());
        let platform = Sequence(std::cell::Cell::new(0));
        let held = block_on(photos.put(&platform, &jpeg(b"here"))).expect("put");
        let elsewhere = photo_id("pho_00000000000000ff");

        let transfer = block_on(PhotoSync::open(
            PHRASE,
            &photos,
            &[held.clone(), elsewhere.clone()],
        ))
        .expect("open");

        match cabas_sync::photo::decode_client(&transfer.hello().unwrap()).unwrap() {
            cabas_sync::photo::PhotoClientMessage::Hello { have, want, .. } => {
                assert_eq!(
                    have,
                    vec![cabas_sync::PhotoName::new(held.as_str()).unwrap()]
                );
                assert_eq!(
                    want,
                    vec![cabas_sync::PhotoName::new(elsewhere.as_str()).unwrap()],
                    "a photo the replica names and this device lacks is the \
                     whole of what it asks for"
                );
            }
            other => panic!("expected a hello, got {other:?}"),
        }
    }

    #[test]
    fn an_arriving_photo_is_written_under_the_id_it_was_sent_as() {
        let photos = Photos::new(MemoryPhotoStore::new());
        let wanted = photo_id("pho_00000000000000ff");
        let mut transfer = block_on(PhotoSync::open(
            PHRASE,
            &photos,
            std::slice::from_ref(&wanted),
        ))
        .expect("open");

        assert_eq!(
            block_on(transfer.handle(&photos, &welcome(&[], &[wanted.as_str()]))).expect("welcome"),
            PhotoEvent::Reconciled {
                upload: 0,
                download: 1
            }
        );
        transfer.fetch().expect("fetch").expect("one to fetch");
        let arriving = sealed_by_the_other_phone(wanted.as_str(), &jpeg(b"a shelf"));

        let event = block_on(transfer.handle(&photos, &arriving)).expect("handle");

        assert_eq!(
            event,
            PhotoEvent::Received {
                id: wanted.to_string()
            }
        );
        assert_eq!(
            block_on(photos.get(&wanted)).expect("get"),
            Some(jpeg(b"a shelf")),
            "the bytes are on this device, under the name the document uses"
        );
        assert!(transfer.done(), "the only photo asked for has arrived");
        assert_eq!(transfer.status().received, 1);
    }

    /// It opened, so it came from inside the group — and it is still not a
    /// photo. The queue behind it has nothing wrong with it.
    #[test]
    fn a_payload_that_opens_to_something_that_is_not_a_photo_is_dropped() {
        let photos = Photos::new(MemoryPhotoStore::new());
        let wanted = photo_id("pho_00000000000000ff");
        let mut transfer = block_on(PhotoSync::open(
            PHRASE,
            &photos,
            std::slice::from_ref(&wanted),
        ))
        .expect("open");
        block_on(transfer.handle(&photos, &welcome(&[], &[wanted.as_str()]))).expect("welcome");
        transfer.fetch().expect("fetch");

        let arriving = sealed_by_the_other_phone(wanted.as_str(), &[0x89, b'P', b'N', b'G']);
        let event = block_on(transfer.handle(&photos, &arriving)).expect("handle");

        assert_eq!(
            event,
            PhotoEvent::Dropped {
                id: wanted.to_string()
            }
        );
        assert_eq!(block_on(photos.get(&wanted)).expect("get"), None);
        assert_eq!(transfer.status().received, 0);
        assert_eq!(transfer.status().dropped, 1);
        assert!(transfer.done(), "the connection outlives one bad blob");
    }

    #[test]
    fn a_photo_offered_and_then_gone_is_skipped_and_the_queue_moves_on() {
        let photos = Photos::new(MemoryPhotoStore::new());
        let platform = Sequence(std::cell::Cell::new(0));
        let first = block_on(photos.put(&platform, &jpeg(b"one"))).expect("put");
        let second = block_on(photos.put(&platform, &jpeg(b"two"))).expect("put");

        let mut transfer = block_on(PhotoSync::open(
            PHRASE,
            &photos,
            &[first.clone(), second.clone()],
        ))
        .expect("open");
        block_on(transfer.handle(&photos, &welcome(&[first.as_str(), second.as_str()], &[])))
            .expect("welcome");

        // Swept between the hello and the push, which the offer is allowed to
        // be a snapshot of.
        block_on(photos.forget_unreferenced(std::slice::from_ref(&second))).expect("sweep");

        let pushed = block_on(transfer.push(&photos))
            .expect("push")
            .expect("one");
        match cabas_sync::photo::decode_client(&pushed).unwrap() {
            cabas_sync::photo::PhotoClientMessage::Push { id, .. } => assert_eq!(
                id.as_str(),
                second.as_str(),
                "the photo that is still on disk is the one that goes"
            ),
            other => panic!("expected a push, got {other:?}"),
        }
        assert!(
            block_on(transfer.push(&photos)).expect("push").is_none(),
            "nothing is left to offer"
        );
    }

    /// A photo id is opaque and a document takes whatever another device — or
    /// an imported file (DECISIONS 0076) — wrote onto an ingredient; a name
    /// the relay would turn into a file is not. The store refuses such an id
    /// too, so the bytes were never here; what this stops is one malformed
    /// reference taking the whole transfer down with it.
    #[test]
    fn an_id_that_could_not_be_a_name_is_not_asked_for() {
        let photos = Photos::new(MemoryPhotoStore::new());
        let platform = Sequence(std::cell::Cell::new(0));
        let ordinary = block_on(photos.put(&platform, &jpeg(b"real"))).expect("put");
        let traversing = photo_id("../../data/00112233445566778899aabbccddeeff/log");
        let elsewhere = photo_id("pho_00000000000000ff");

        let transfer = block_on(PhotoSync::open(
            PHRASE,
            &photos,
            &[ordinary.clone(), traversing, elsewhere.clone()],
        ))
        .expect("one unusable reference does not fail the connection");

        match cabas_sync::photo::decode_client(&transfer.hello().unwrap()).unwrap() {
            cabas_sync::photo::PhotoClientMessage::Hello { have, want, .. } => {
                assert_eq!(
                    have,
                    vec![cabas_sync::PhotoName::new(ordinary.as_str()).unwrap()]
                );
                assert_eq!(
                    want,
                    vec![cabas_sync::PhotoName::new(elsewhere.as_str()).unwrap()],
                    "the traversing name is not asked for, and everything \
                     else still is"
                );
            }
            other => panic!("expected a hello, got {other:?}"),
        }
    }

    #[test]
    fn a_pushed_photo_the_relay_confirms_stops_being_the_only_copy() {
        let photos = Photos::new(MemoryPhotoStore::new());
        let platform = Sequence(std::cell::Cell::new(0));
        let mine = block_on(photos.put(&platform, &jpeg(b"mine"))).expect("put");

        let mut transfer = block_on(PhotoSync::open(
            PHRASE,
            &photos,
            std::slice::from_ref(&mine),
        ))
        .expect("open");
        block_on(transfer.handle(&photos, &welcome(&[mine.as_str()], &[]))).expect("welcome");
        block_on(transfer.push(&photos))
            .expect("push")
            .expect("one");
        assert!(!transfer.done(), "a push is owed an answer");

        let event = block_on(transfer.handle(
            &photos,
            &wire(&cabas_sync::photo::PhotoServerMessage::Stored {
                id: cabas_sync::PhotoName::new(mine.as_str()).unwrap(),
            }),
        ))
        .expect("handle");

        assert_eq!(
            event,
            PhotoEvent::Sent {
                id: mine.to_string()
            }
        );
        assert_eq!(transfer.status().sent, 1);
        assert!(transfer.done());
    }
}
