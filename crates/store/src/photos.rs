//! Where the photo bytes live — beside the document, never inside it.
//!
//! [`Storage`](crate::storage::Storage) is one blob in and one blob out, and
//! that blob is the whole library: every save rewrites it. Photos are the
//! first thing here big enough for that to matter — a library of them inside
//! the document turns ticking an item off in a shop into a multi-megabyte
//! write and every launch into an import of the lot (DECISIONS 0062, which
//! carries the measurement).
//!
//! So they get a store of their own: **one record per photo**, keyed by the
//! [`PhotoId`] the document carries. Nothing here is ever rewritten — a photo
//! is immutable, created once and deleted once — which is the whole point.
//!
//! # What this trait is not
//!
//! It is not a cache. A photo that exists only here and nowhere else is the
//! only copy, until the transfer half of M10 puts a sealed copy on the relay.
//! `remove` therefore means "this device no longer needs it", never "throw it
//! away": the caller has to know the relay can hand it back.

use cabas_domain::PhotoId;

use crate::error::{Result, StoreError};

/// The bytes of one photo, addressed by the id the document holds.
///
/// Async for the same reason [`Storage`](crate::storage::Storage) is: every
/// IndexedDB operation completes on a later turn of the event loop and there
/// is no blocking form. The native backends satisfy it without ever yielding.
///
/// Deliberately no `Send` bound on the returned futures — on wasm32 they are
/// `!Send`, and requiring it would make the target that matters impossible to
/// implement.
#[allow(async_fn_in_trait)]
pub trait PhotoStore {
    /// The photo's bytes, or `None` if this device does not hold it.
    ///
    /// `None` is an ordinary answer, not a failure: a photo the other phone
    /// took is referenced by the document long before its bytes arrive.
    async fn load(&self, id: &PhotoId) -> Result<Option<Vec<u8>>>;

    /// Stores a photo under `id`. Writing the same id twice is a no-op in
    /// effect — ids are minted per capture, so the bytes cannot differ.
    async fn save(&self, id: &PhotoId, bytes: &[u8]) -> Result<()>;

    /// Drops this device's copy. Removing what is not there succeeds.
    async fn remove(&self, id: &PhotoId) -> Result<()>;

    /// Every id this device holds — what a prefetch diffs the document
    /// against to know what is missing, and what an upload diffs the relay's
    /// answer against to know what is owed.
    async fn ids(&self) -> Result<Vec<PhotoId>>;
}

/// Rejects an id that could name something other than a record.
///
/// The file backend turns an id into a path and the IndexedDB one into a key,
/// and an id arrives from a document that another device wrote. Ids are
/// minted as hex here, so anything outside this alphabet is either corruption
/// or an attempt to escape the directory — both of which are refusals rather
/// than best-effort reads.
pub(crate) fn checked(id: &PhotoId) -> Result<&str> {
    let raw = id.as_str();
    let ok = !raw.is_empty()
        && raw.len() <= 64
        && raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if ok {
        Ok(raw)
    } else {
        Err(StoreError::Io(format!("not a usable photo id: {raw:?}")))
    }
}

/// Photos in memory, for tests and for a replica with no disk behind it.
#[derive(Debug, Default, Clone)]
pub struct MemoryPhotoStore {
    photos: std::sync::Arc<std::sync::Mutex<std::collections::BTreeMap<String, Vec<u8>>>>,
}

impl MemoryPhotoStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, std::collections::BTreeMap<String, Vec<u8>>>> {
        self.photos
            .lock()
            .map_err(|_| StoreError::Io("in-memory photo store lock was poisoned".into()))
    }
}

impl PhotoStore for MemoryPhotoStore {
    async fn load(&self, id: &PhotoId) -> Result<Option<Vec<u8>>> {
        Ok(self.lock()?.get(checked(id)?).cloned())
    }

    async fn save(&self, id: &PhotoId, bytes: &[u8]) -> Result<()> {
        self.lock()?.insert(checked(id)?.to_owned(), bytes.to_vec());
        Ok(())
    }

    async fn remove(&self, id: &PhotoId) -> Result<()> {
        self.lock()?.remove(checked(id)?);
        Ok(())
    }

    async fn ids(&self) -> Result<Vec<PhotoId>> {
        Ok(self.lock()?.keys().map(PhotoId::from_raw).collect())
    }
}

#[cfg(not(target_family = "wasm"))]
pub use file::FilePhotoStore;

#[cfg(not(target_family = "wasm"))]
mod file {
    use std::path::{Path, PathBuf};

    use cabas_domain::PhotoId;

    use super::{PhotoStore, StoreError, checked};
    use crate::error::Result;

    /// One file per photo in one directory — the backend for Tauri (M7, M8)
    /// and for anything native that holds a replica.
    #[derive(Debug, Clone)]
    pub struct FilePhotoStore {
        dir: PathBuf,
    }

    impl FilePhotoStore {
        pub fn new(dir: impl Into<PathBuf>) -> Self {
            Self { dir: dir.into() }
        }

        pub fn dir(&self) -> &Path {
            &self.dir
        }

        fn path(&self, id: &PhotoId) -> Result<PathBuf> {
            Ok(self.dir.join(checked(id)?))
        }
    }

    fn io(error: std::io::Error) -> StoreError {
        StoreError::Io(error.to_string())
    }

    impl PhotoStore for FilePhotoStore {
        async fn load(&self, id: &PhotoId) -> Result<Option<Vec<u8>>> {
            match std::fs::read(self.path(id)?) {
                Ok(bytes) => Ok(Some(bytes)),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(io(e)),
            }
        }

        /// Writes beside the target and renames over it.
        ///
        /// A photo is written once and never rewritten, so this is not the
        /// lost-library problem `Storage::save` guards against — but a torn
        /// file is a permanently broken image that nothing would report, and
        /// a rename costs one syscall.
        async fn save(&self, id: &PhotoId, bytes: &[u8]) -> Result<()> {
            let path = self.path(id)?;
            std::fs::create_dir_all(&self.dir).map_err(io)?;

            let mut name = path.file_name().unwrap_or_default().to_os_string();
            name.push(".new");
            let scratch = path.with_file_name(name);

            std::fs::write(&scratch, bytes).map_err(io)?;
            std::fs::rename(&scratch, &path).map_err(io)?;
            Ok(())
        }

        async fn remove(&self, id: &PhotoId) -> Result<()> {
            match std::fs::remove_file(self.path(id)?) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(io(e)),
            }
        }

        /// A missing directory is an empty store, not a failure: it is what
        /// a device that has never held a photo looks like.
        async fn ids(&self) -> Result<Vec<PhotoId>> {
            let entries = match std::fs::read_dir(&self.dir) {
                Ok(entries) => entries,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
                Err(e) => return Err(io(e)),
            };

            let mut ids = Vec::new();
            for entry in entries {
                let entry = entry.map_err(io)?;
                let Ok(name) = entry.file_name().into_string() else {
                    continue;
                };
                // Anything that is not a well-formed id is something this
                // store did not write — an interrupted `.new`, or a stray
                // file — and reporting it would make a caller ask the relay
                // for a photo nobody ever took.
                let id = PhotoId::from_raw(name);
                if checked(&id).is_ok() {
                    ids.push(id);
                }
            }
            ids.sort();
            Ok(ids)
        }
    }
}

#[cfg(target_family = "wasm")]
pub use crate::storage::IndexedDbPhotoStore;

#[cfg(test)]
mod tests {
    use super::*;

    /// Drives a future to completion without an async runtime — the native
    /// backends never yield, so a no-op waker is enough (`storage.rs` does
    /// the same, for the same reason).
    fn block_on<F: std::future::Future>(future: F) -> F::Output {
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

    fn id(raw: &str) -> PhotoId {
        PhotoId::from_raw(raw)
    }

    fn round_trip(store: &impl PhotoStore) {
        let one = id("ph_0000000000000001");
        let two = id("ph_0000000000000002");

        assert_eq!(block_on(store.load(&one)).expect("load"), None);
        assert!(block_on(store.ids()).expect("ids").is_empty());

        block_on(store.save(&one, b"first")).expect("save");
        block_on(store.save(&two, b"second")).expect("save");

        assert_eq!(
            block_on(store.load(&one)).expect("load"),
            Some(b"first".to_vec())
        );
        assert_eq!(
            block_on(store.ids()).expect("ids"),
            vec![one.clone(), two.clone()]
        );

        block_on(store.remove(&one)).expect("remove");
        assert_eq!(block_on(store.load(&one)).expect("load"), None);
        assert_eq!(block_on(store.ids()).expect("ids"), vec![two]);

        // Removing what is not there is how a cleanup pass ends, not an
        // error it has to guard against.
        block_on(store.remove(&one)).expect("remove twice");
    }

    #[test]
    fn memory_photos_round_trip() {
        round_trip(&MemoryPhotoStore::new());
    }

    #[test]
    fn an_id_that_could_name_a_path_is_refused() {
        let store = MemoryPhotoStore::new();
        for bad in ["../secret", "a/b", "", "ph 01", "ph.01"] {
            assert!(
                block_on(store.save(&id(bad), b"x")).is_err(),
                "{bad:?} was accepted"
            );
        }
    }

    #[cfg(not(target_family = "wasm"))]
    mod file {
        use super::*;
        use crate::photos::FilePhotoStore;

        /// A directory that cleans itself up, so the tests need no dev-dep.
        struct TempDir(std::path::PathBuf);

        impl TempDir {
            fn new(tag: &str) -> Self {
                let mut path = std::env::temp_dir();
                path.push(format!(
                    "cabas-photos-{tag}-{}-{:?}",
                    std::process::id(),
                    std::thread::current().id()
                ));
                let _ = std::fs::remove_dir_all(&path);
                std::fs::create_dir_all(&path).expect("create temp dir");
                Self(path)
            }
        }

        impl Drop for TempDir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }

        #[test]
        fn file_photos_round_trip() {
            let dir = TempDir::new("round-trip");
            round_trip(&FilePhotoStore::new(dir.0.join("photos")));
        }

        #[test]
        fn a_directory_that_does_not_exist_yet_is_an_empty_store() {
            let dir = TempDir::new("absent");
            let store = FilePhotoStore::new(dir.0.join("never-created"));
            assert!(block_on(store.ids()).expect("ids").is_empty());
            assert_eq!(
                block_on(store.load(&id("ph_0000000000000001"))).expect("load"),
                None
            );
        }

        #[test]
        fn a_stray_file_is_not_reported_as_a_photo() {
            let dir = TempDir::new("stray");
            let store = FilePhotoStore::new(dir.0.clone());
            block_on(store.save(&id("ph_0000000000000001"), b"real")).expect("save");
            std::fs::write(dir.0.join("photo.new"), b"interrupted").expect("write");

            assert_eq!(
                block_on(store.ids()).expect("ids"),
                vec![id("ph_0000000000000001")]
            );
        }

        #[test]
        fn an_id_that_escapes_the_directory_never_reaches_the_filesystem() {
            let dir = TempDir::new("escape");
            let store = FilePhotoStore::new(dir.0.join("photos"));
            assert!(block_on(store.save(&id("../escaped"), b"x")).is_err());
            assert!(!dir.0.join("escaped").exists());
        }
    }
}
