//! Where the snapshot actually lives.
//!
//! One trait, one blob. There is a single family document (DECISIONS 0018),
//! so a storage backend does not need keys, queries or a schema — it needs to
//! hand back the bytes it was given, and to not lose them when the process
//! dies halfway through a write.
//!
//! # Why the trait is async
//!
//! Not for the native backends, which are a `read` and a `write`. For
//! IndexedDB: every operation in it is a request that completes on a later
//! turn of the event loop, and there is no blocking form. One async trait
//! that native implementations satisfy trivially beats two traits that `app`
//! would have to abstract over anyway (Rule 8).

use crate::error::{Result, StoreError};

/// Loads and stores the serialized document.
///
/// Deliberately no `Send` bound on the returned futures. On wasm32 they are
/// `!Send` — there is one thread and `JsValue` cannot cross one — so
/// requiring `Send` here would make the target that actually matters
/// impossible to implement.
#[allow(async_fn_in_trait)]
pub trait Storage {
    /// The stored document, or `None` on a first run.
    async fn load(&self) -> Result<Option<Vec<u8>>>;

    /// Replaces the stored document.
    ///
    /// Implementations must be **all-or-nothing**. A snapshot is the entire
    /// library, so a write interrupted halfway is not a partial save, it is a
    /// destroyed one — and on a phone the process is killed at the OS's
    /// convenience, not ours.
    async fn save(&self, snapshot: &[u8]) -> Result<()>;
}

/// In-memory storage, for tests and for anything that wants a replica without
/// a disk behind it. Available on every target.
#[derive(Debug, Default, Clone)]
pub struct MemoryStorage {
    bytes: std::sync::Arc<std::sync::Mutex<Option<Vec<u8>>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Storage for MemoryStorage {
    async fn load(&self) -> Result<Option<Vec<u8>>> {
        Ok(self
            .bytes
            .lock()
            .map_err(|_| StoreError::Io("in-memory storage lock was poisoned".into()))?
            .clone())
    }

    async fn save(&self, snapshot: &[u8]) -> Result<()> {
        *self
            .bytes
            .lock()
            .map_err(|_| StoreError::Io("in-memory storage lock was poisoned".into()))? =
            Some(snapshot.to_vec());
        Ok(())
    }
}

#[cfg(not(target_family = "wasm"))]
pub use file::FileStorage;

#[cfg(not(target_family = "wasm"))]
mod file {
    use std::path::{Path, PathBuf};

    use super::{Storage, StoreError};
    use crate::error::Result;

    /// A snapshot in a file — the backend for Tauri (M7, M8) and the relay.
    #[derive(Debug, Clone)]
    pub struct FileStorage {
        path: PathBuf,
    }

    impl FileStorage {
        pub fn new(path: impl Into<PathBuf>) -> Self {
            Self { path: path.into() }
        }

        pub fn path(&self) -> &Path {
            &self.path
        }

        /// The file the next save writes through.
        fn scratch(&self) -> PathBuf {
            let mut name = self.path.file_name().unwrap_or_default().to_os_string();
            name.push(".new");
            self.path.with_file_name(name)
        }
    }

    fn io(error: std::io::Error) -> StoreError {
        StoreError::Io(error.to_string())
    }

    impl Storage for FileStorage {
        async fn load(&self) -> Result<Option<Vec<u8>>> {
            match std::fs::read(&self.path) {
                Ok(bytes) => Ok(Some(bytes)),
                // A missing file is a first run, not a failure.
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(io(e)),
            }
        }

        /// Writes beside the target and renames over it.
        ///
        /// `rename` within a directory is atomic, so a reader sees either the
        /// old snapshot or the new one — never the half of the new one that
        /// had been flushed when the process died. Writing in place would
        /// make every save a window in which the family library can be lost,
        /// and the saves happen on a phone that gets killed without warning.
        ///
        /// The blocking calls are deliberate: the document is a few hundred
        /// kilobytes (DECISIONS 0008), which is one `write` syscall, and
        /// handing that to a thread pool would cost more than it saves.
        async fn save(&self, snapshot: &[u8]) -> Result<()> {
            if let Some(parent) = self.path.parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(parent).map_err(io)?;
            }

            let scratch = self.scratch();
            std::fs::write(&scratch, snapshot).map_err(io)?;
            std::fs::rename(&scratch, &self.path).map_err(io)?;
            Ok(())
        }
    }
}

#[cfg(target_family = "wasm")]
pub use indexed_db::IndexedDbStorage;

#[cfg(target_family = "wasm")]
mod indexed_db {
    //! The PWA's backend, and the only durable store an installed iOS web app
    //! has (DECISIONS 0003).
    //!
    //! Everything here is a request that completes on a later turn of the
    //! event loop, so each operation is a hand-built promise bridged with
    //! `JsFuture`. There is no blocking form of any of it; that is what
    //! decided the shape of the [`Storage`](super::Storage) trait.

    use js_sys::{Promise, Uint8Array};
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{IdbDatabase, IdbRequest, IdbTransaction, IdbTransactionMode};

    use super::{Storage, StoreError};
    use crate::error::Result;

    /// Bumping this runs `onupgradeneeded` again. It tracks the *object
    /// store* layout, which is one blob under one key, and has nothing to do
    /// with [`crate::SCHEMA_VERSION`] — that one versions the document
    /// inside the blob.
    const VERSION: u32 = 1;
    const STORE: &str = "document";
    const KEY: &str = "snapshot";

    /// The family document in IndexedDB.
    #[derive(Debug, Clone)]
    pub struct IndexedDbStorage {
        database: String,
    }

    impl Default for IndexedDbStorage {
        fn default() -> Self {
            Self::new()
        }
    }

    impl IndexedDbStorage {
        pub fn new() -> Self {
            Self::with_database("cabas")
        }

        /// A named database — what the tests use to keep runs from colliding.
        pub fn with_database(name: impl Into<String>) -> Self {
            Self {
                database: name.into(),
            }
        }

        /// Opens the database, creating the object store on first use.
        ///
        /// Opened per operation rather than cached on the struct. A handle
        /// would have to survive across awaits in a `!Send` world, and it
        /// goes stale the moment another tab triggers a version change —
        /// while `open` on an already-open database is cheap, because the
        /// browser keeps the connection.
        async fn open(&self) -> Result<IdbDatabase> {
            let window = web_sys::window().ok_or_else(|| {
                StoreError::Io("IndexedDB needs a window; none in this context".into())
            })?;
            let factory = window
                .indexed_db()
                .map_err(|e| js_error("indexedDB", e))?
                .ok_or_else(|| {
                    StoreError::Io(
                        "IndexedDB is unavailable — private browsing, or storage is blocked".into(),
                    )
                })?;

            let request = factory
                .open_with_u32(&self.database, VERSION)
                .map_err(|e| js_error("open", e))?;

            let upgrading = request.clone();
            let on_upgrade = Closure::once_into_js(move || {
                // Fires before the open request settles, on a brand-new
                // database or after a version bump.
                if let Ok(value) = upgrading.result() {
                    let db: IdbDatabase = value.unchecked_into();
                    if !db.object_store_names().contains(STORE) {
                        let _ = db.create_object_store(STORE);
                    }
                }
            });
            request.set_onupgradeneeded(Some(on_upgrade.unchecked_ref()));

            let db = JsFuture::from(settled(&request))
                .await
                .map_err(|e| js_error("open", e))?;
            Ok(db.unchecked_into())
        }
    }

    impl Storage for IndexedDbStorage {
        async fn load(&self) -> Result<Option<Vec<u8>>> {
            let db = self.open().await?;
            let transaction = db
                .transaction_with_str(STORE)
                .map_err(|e| js_error("read transaction", e))?;
            let store = transaction
                .object_store(STORE)
                .map_err(|e| js_error("object store", e))?;
            let request = store
                .get(&JsValue::from_str(KEY))
                .map_err(|e| js_error("get", e))?;

            let value = JsFuture::from(settled(&request))
                .await
                .map_err(|e| js_error("get", e))?;

            // A key that was never written comes back `undefined`. That is a
            // first run, not a failure.
            if value.is_undefined() || value.is_null() {
                return Ok(None);
            }
            Ok(Some(Uint8Array::new(&value).to_vec()))
        }

        async fn save(&self, snapshot: &[u8]) -> Result<()> {
            let db = self.open().await?;
            let transaction = db
                .transaction_with_str_and_mode(STORE, IdbTransactionMode::Readwrite)
                .map_err(|e| js_error("write transaction", e))?;
            let store = transaction
                .object_store(STORE)
                .map_err(|e| js_error("object store", e))?;
            store
                .put_with_key(&Uint8Array::from(snapshot), &JsValue::from_str(KEY))
                .map_err(|e| js_error("put", e))?;

            // Waits on the *transaction*, not on the put. IndexedDB's
            // atomicity is per transaction, and a resolved request only means
            // the write is queued — the all-or-nothing guarantee the trait
            // demands is `oncomplete`, which is also the point at which the
            // browser has actually durably stored the bytes.
            JsFuture::from(committed(&transaction))
                .await
                .map_err(|e| js_error("commit", e))?;
            Ok(())
        }
    }

    /// A promise that settles when an IndexedDB request does.
    fn settled(request: &IdbRequest) -> Promise {
        let request = request.clone();
        Promise::new(&mut |resolve, reject| {
            let succeeded = request.clone();
            let on_success = Closure::once_into_js(move || {
                let value = succeeded.result().unwrap_or(JsValue::UNDEFINED);
                let _ = resolve.call1(&JsValue::UNDEFINED, &value);
            });
            let failed = request.clone();
            let on_error = Closure::once_into_js(move || {
                let _ = reject.call1(&JsValue::UNDEFINED, &reason(&failed));
            });
            // The handles are dropped at the end of this block, but JS keeps
            // the functions alive through the request itself, and
            // `once_into_js` frees the Rust side after the single call.
            request.set_onsuccess(Some(on_success.unchecked_ref()));
            request.set_onerror(Some(on_error.unchecked_ref()));
        })
    }

    /// A promise that settles when a transaction commits, aborts or fails.
    fn committed(transaction: &IdbTransaction) -> Promise {
        let transaction = transaction.clone();
        Promise::new(&mut |resolve, reject| {
            let on_complete = Closure::once_into_js(move || {
                let _ = resolve.call0(&JsValue::UNDEFINED);
            });
            let aborted = reject.clone();
            let on_error = Closure::once_into_js(move || {
                let _ = reject.call1(
                    &JsValue::UNDEFINED,
                    &JsValue::from_str("transaction failed"),
                );
            });
            let on_abort = Closure::once_into_js(move || {
                let _ = aborted.call1(
                    &JsValue::UNDEFINED,
                    &JsValue::from_str("transaction aborted"),
                );
            });
            transaction.set_oncomplete(Some(on_complete.unchecked_ref()));
            transaction.set_onerror(Some(on_error.unchecked_ref()));
            transaction.set_onabort(Some(on_abort.unchecked_ref()));
        })
    }

    fn reason(request: &IdbRequest) -> JsValue {
        match request.error() {
            Ok(Some(exception)) => exception.into(),
            _ => JsValue::from_str("request failed"),
        }
    }

    fn js_error(context: &str, value: JsValue) -> StoreError {
        let detail = value
            .as_string()
            .or_else(|| {
                value
                    .dyn_ref::<web_sys::DomException>()
                    .map(|e| e.message())
            })
            .unwrap_or_else(|| format!("{value:?}"));
        StoreError::Io(format!("IndexedDB {context}: {detail}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drives a future to completion without pulling in an async runtime.
    ///
    /// Every backend here completes without ever yielding — the natives do no
    /// real waiting — so a no-op waker is enough, and it keeps `tokio` out of
    /// this crate's dependencies for the sake of two tests.
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

    #[test]
    fn memory_storage_round_trips_and_starts_empty() {
        let storage = MemoryStorage::new();
        assert_eq!(block_on(storage.load()).expect("load"), None);
        block_on(storage.save(b"snapshot")).expect("save");
        assert_eq!(
            block_on(storage.load()).expect("load"),
            Some(b"snapshot".to_vec())
        );
    }

    #[cfg(not(target_family = "wasm"))]
    mod file {
        use super::*;
        use crate::storage::FileStorage;

        /// A directory that cleans itself up, so the tests need no dev-dep.
        struct TempDir(std::path::PathBuf);

        impl TempDir {
            fn new(tag: &str) -> Self {
                let mut path = std::env::temp_dir();
                path.push(format!(
                    "cabas-store-{tag}-{}-{:?}",
                    std::process::id(),
                    std::thread::current().id()
                ));
                std::fs::create_dir_all(&path).expect("create temp dir");
                Self(path)
            }

            fn join(&self, name: &str) -> std::path::PathBuf {
                self.0.join(name)
            }
        }

        impl Drop for TempDir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }

        #[test]
        fn a_missing_file_is_a_first_run_not_an_error() {
            let dir = TempDir::new("missing");
            let storage = FileStorage::new(dir.join("cabas.loro"));
            assert_eq!(block_on(storage.load()).expect("load"), None);
        }

        #[test]
        fn a_snapshot_round_trips_through_the_filesystem() {
            let dir = TempDir::new("roundtrip");
            let storage = FileStorage::new(dir.join("cabas.loro"));
            block_on(storage.save(b"a snapshot")).expect("save");
            assert_eq!(
                block_on(storage.load()).expect("load"),
                Some(b"a snapshot".to_vec())
            );

            // Saving again replaces rather than appends.
            block_on(storage.save(b"newer")).expect("save");
            assert_eq!(
                block_on(storage.load()).expect("load"),
                Some(b"newer".to_vec())
            );
        }

        #[test]
        fn saving_creates_the_directory_it_was_pointed_at() {
            // The first launch on a device: the app's data directory does not
            // exist yet, and failing there would look like data loss.
            let dir = TempDir::new("nested");
            let storage = FileStorage::new(dir.join("deep").join("cabas.loro"));
            block_on(storage.save(b"x")).expect("save");
            assert_eq!(block_on(storage.load()).expect("load"), Some(b"x".to_vec()));
        }

        #[test]
        fn the_scratch_file_does_not_survive_a_save() {
            let dir = TempDir::new("scratch");
            let path = dir.join("cabas.loro");
            let storage = FileStorage::new(&path);
            block_on(storage.save(b"x")).expect("save");

            let leftovers: Vec<_> = std::fs::read_dir(&dir.0)
                .expect("read dir")
                .filter_map(|e| e.ok())
                .map(|e| e.file_name())
                .filter(|name| name.to_string_lossy().ends_with(".new"))
                .collect();
            assert!(leftovers.is_empty(), "left behind: {leftovers:?}");
        }
    }
}
