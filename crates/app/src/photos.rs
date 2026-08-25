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

use cabas_domain::PhotoId;
use cabas_store::PhotoStore;

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

        let id = PhotoId::from_raw(id::mint(platform, id::PHOTO)?);
        self.store.save(&id, bytes).await?;
        Ok(id)
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
}
