//! The PWA's edge of the world: JS objects in, JS objects out.
//!
//! Thin on purpose. Everything this module does is translate — nothing here
//! decides anything, because the moment it did, the Tauri host would need the
//! same decision made a second time (Rule 9, DECISIONS 0005).
//!
//! # Why `apply` and `flush` are two calls
//!
//! [`CabasApp::apply`] is synchronous and returns the new state; the write to
//! IndexedDB happens in [`CabasApp::flush`], which returns a promise the UI
//! is free to ignore. Rendering therefore never waits on a browser
//! transaction (Rule 6).
//!
//! There is a second, less obvious reason. An exported async method holds its
//! borrow of the app for as long as its promise is pending, so a version that
//! awaited the save inside `apply` would panic on the second tap of an
//! impatient thumb — in a shop, on a phone, which is precisely where nobody
//! is watching a console. Keeping the mutation synchronous means the borrow
//! is released before anything is awaited.

use std::cell::RefCell;

use cabas_domain::PhotoId;
use cabas_store::{IndexedDbPhotoStore, IndexedDbStorage, Storage};
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::app::App;
use crate::command::Command;
use crate::photos::Photos;
use crate::platform::{Identity, SystemPlatform};
use crate::sync::{SyncCursor, SyncSession};
use crate::transfer::{Imported, LibraryFile};

/// Serialises a view-model the way the generated TypeScript declares it.
///
/// The default serialiser turns `None` into `undefined`, and the types in
/// `ui/src/lib/bindings/` say `| null` — so `checked_by === null` would be a
/// test that never passes, against a declaration that says it should. One
/// setting keeps the runtime and the types telling the same story.
fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_missing_as_null(true);
    Ok(value.serialize(&serializer)?)
}

#[wasm_bindgen]
pub struct CabasApp {
    inner: RefCell<App<IndexedDbStorage, SystemPlatform>>,
    /// A second handle, held outside the `RefCell` so a save can be awaited
    /// without borrowing the app across it. It is a database *name*, not a
    /// connection — cloning it costs nothing.
    storage: IndexedDbStorage,
    /// The photos, in an object store of their own inside the same database.
    /// Outside the `RefCell` for the same reason `storage` is: every call on
    /// it is awaited, and the app must not be borrowed across one.
    photos: Photos<IndexedDbPhotoStore>,
    /// The current connection, or `None` between them. One session per
    /// socket: it is created from the persisted cursor when the socket opens
    /// and dropped when it closes, which is also when the key leaves memory.
    session: RefCell<Option<SyncSession>>,
}

#[wasm_bindgen]
impl CabasApp {
    /// Mints the id of a device that has never run before.
    ///
    /// Called **once**, by the host, which then persists the result and hands
    /// it to [`CabasApp::open`] on every launch. Where a device remembers
    /// things about itself is the host's business — `localStorage` here —
    /// and the group document holds only the records these ids point at.
    ///
    /// It says nothing about *who* is carrying the device: that is a row on
    /// the group's roster, and the roster arrives over the network
    /// (DECISIONS 0068). The host asks once the app is open, runs
    /// `choose_user` or `create_user`, and persists [`CabasApp::identity`].
    #[wasm_bindgen(js_name = mintDevice)]
    pub fn mint_device(device_name: String) -> Result<JsValue, JsError> {
        let identity = Identity::mint_device(&SystemPlatform, device_name)?;
        to_js(&identity)
    }

    /// Who this device says it is, after a command may have changed it.
    ///
    /// The host persists this: it holds the only durable copy (0031), and a
    /// device that chose a user and did not write the result down comes back
    /// from the next launch not knowing who it is.
    #[wasm_bindgen]
    pub fn identity(&self) -> Result<JsValue, JsError> {
        to_js(self.inner.borrow().identity())
    }

    /// Which build this is — the workspace version, and so the exact string
    /// the Supervisor compares to decide the add-on has an update (Rule 15).
    ///
    /// It answers a question nothing else on the device can: a service worker
    /// installs a new build behind the running one and takes over at the next
    /// launch (DECISIONS 0038), so "is this phone running what the relay
    /// serves" has a real answer and no way to read it. The core is the right
    /// place for it because the core *is* the artifact — the bundle is
    /// compiled into the relay binary (0048), so the version a phone displays
    /// is the version of the relay build that served it.
    #[wasm_bindgen(js_name = buildVersion)]
    pub fn build_version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    /// Mints the id of a recipe line the editor is about to create.
    ///
    /// Synchronous and free of the replica on purpose: it is called while a
    /// form is being filled, by an editor that holds a draft the document has
    /// never seen. See [`crate::mint_usage_id`] for why the id comes from here
    /// at all (DECISIONS 0039).
    #[wasm_bindgen(js_name = mintUsageId)]
    pub fn mint_usage_id() -> Result<String, JsError> {
        Ok(crate::mint_usage_id(&SystemPlatform)?)
    }

    /// Mints the id of an ingredient a picker is about to create.
    ///
    /// Synchronous and free of the replica, like the usage id above: it is
    /// called by a form that has to select the ingredient it just created,
    /// in a picker, before anything has been saved (DECISIONS 0056).
    #[wasm_bindgen(js_name = mintIngredientId)]
    pub fn mint_ingredient_id() -> Result<String, JsError> {
        Ok(crate::mint_ingredient_id(&SystemPlatform)?)
    }

    /// Mints the id of a shop the ingredient form is about to create.
    ///
    /// The third of the same shape, for the same reason (DECISIONS 0071): the
    /// field that creates a shop has to add it to the draft it is sitting in,
    /// the instant it exists.
    #[wasm_bindgen(js_name = mintShopId)]
    pub fn mint_shop_id() -> Result<String, JsError> {
        Ok(crate::mint_shop_id(&SystemPlatform)?)
    }

    /// Opens the replica stored in IndexedDB, or starts a new one.
    pub async fn open(identity: JsValue) -> Result<CabasApp, JsError> {
        let identity: Identity = serde_wasm_bindgen::from_value(identity)?;
        let storage = IndexedDbStorage::new();
        let app = App::open(storage.clone(), SystemPlatform, identity).await?;
        Ok(Self {
            inner: RefCell::new(app),
            photos: Photos::new(IndexedDbPhotoStore::new()),
            storage,
            session: RefCell::new(None),
        })
    }

    /// The current state — one call, at startup. Everything after that
    /// arrives as the return value of [`CabasApp::apply`].
    pub fn state(&self) -> Result<JsValue, JsError> {
        to_js(&self.inner.borrow().state()?)
    }

    /// Applies one command and returns the whole new state, synchronously.
    pub fn apply(&self, command: JsValue) -> Result<JsValue, JsError> {
        let command: Command = serde_wasm_bindgen::from_value(command)?;
        let state = self.inner.borrow_mut().apply(command)?;
        to_js(&state)
    }

    /// Writes the replica to IndexedDB if anything has changed since the last
    /// write. Resolves to `true` when it wrote.
    ///
    /// Safe to call after every command, and safe to debounce: the revision
    /// it saves is the one that was current when it started, so a command
    /// applied while the write was in flight stays pending rather than being
    /// counted as saved.
    pub async fn flush(&self) -> Result<bool, JsError> {
        // The borrow ends with this statement, before anything is awaited.
        let pending = self.inner.borrow_mut().pending_snapshot()?;
        let Some((snapshot, revision)) = pending else {
            return Ok(false);
        };
        self.storage.save(&snapshot).await?;
        self.inner.borrow_mut().mark_saved(revision);
        Ok(true)
    }

    // --- photos (DECISIONS 0062) --------------------------------------------
    //
    // A store of their own, beside the document: the document is rewritten
    // whole on every save and a photo library inside it would be rewritten
    // with it. These calls await a browser transaction, so — like `flush` —
    // they take no borrow of the app across it.

    /// The most a photo may weigh, so the encoder has a number to target.
    #[wasm_bindgen(js_name = maxPhotoBytes)]
    pub fn max_photo_bytes() -> usize {
        crate::photos::MAX_PHOTO_BYTES
    }

    /// Stores a photo and returns its id, to be put on the ingredient or the
    /// recipe by the ordinary save that follows.
    ///
    /// Two calls rather than one because they belong to different clocks: a
    /// photo is written once and a form is saved whenever the user is ready,
    /// and folding them together would make `apply` asynchronous for
    /// everything else too.
    #[wasm_bindgen(js_name = putPhoto)]
    pub async fn put_photo(&self, bytes: Vec<u8>) -> Result<String, JsError> {
        let id = self.photos.put(&SystemPlatform, &bytes).await?;
        Ok(id.to_string())
    }

    /// The bytes of a photo this device holds, or nothing.
    ///
    /// Absent is an ordinary answer: a photo taken on the other phone is
    /// named by the document from the moment the replicas merge, and its
    /// bytes arrive on their own afterwards. The screen shows a placeholder
    /// and never an error (Rule 6).
    pub async fn photo(&self, id: String) -> Result<Option<Vec<u8>>, JsError> {
        Ok(self.photos.get(&PhotoId::from_raw(id)).await?)
    }

    /// Ids the replica references and this device does not hold — what the
    /// transfer half of M10 will ask the relay for, in order.
    #[wasm_bindgen(js_name = missingPhotos)]
    pub async fn missing_photos(&self) -> Result<JsValue, JsError> {
        // The borrow ends with this statement; nothing is awaited under it.
        let referenced = self.inner.borrow().referenced_photos()?;
        let missing: Vec<String> = self
            .photos
            .missing(&referenced)
            .await?
            .into_iter()
            .map(|id| id.to_string())
            .collect();
        to_js(&missing)
    }

    // --- the library as a file (DECISIONS 0076) -----------------------------
    //
    // Two calls, both asynchronous, and both for the same reason as the photo
    // pair above: the document is read and written synchronously, and the
    // photos are a browser transaction each. The borrow of the app is taken
    // and released in a statement that ends, never held across an await.

    /// The whole library as JSON text, for the host to hand to a share sheet
    /// or a download.
    ///
    /// `with_photos` is not a detail. Without them the file is a few tens of
    /// kilobytes and can be read and edited in any text editor — which is the
    /// point of it being JSON. With them it carries every picture base64'd,
    /// which is a third more than the bytes themselves (DECISIONS 0062 has
    /// what that weighs), and is a backup rather than a document.
    #[wasm_bindgen(js_name = exportLibrary)]
    pub async fn export_library(&self, with_photos: bool) -> Result<String, JsError> {
        // The borrow ends with this statement; the awaits are all below it.
        let mut file = self.inner.borrow().export_library()?;
        if with_photos {
            for id in file.photo_ids() {
                if let Some(bytes) = self.photos.get(&id).await? {
                    file.attach_photo(&id, &bytes);
                }
                // A photo this device does not hold is skipped in silence: it
                // was taken on the other phone and its bytes have not arrived
                // (Rule 6). The entity keeps naming it, so the file stays
                // truthful about what exists.
            }
        }
        Ok(file.to_json()?)
    }

    /// Merges a file into this replica and returns `{ report, state }`.
    ///
    /// Photos go in **first**, before a single entity is written. That is
    /// what makes a file with one unreadable picture in it fail with nothing
    /// changed, rather than with a library half merged behind a photo nobody
    /// can open.
    #[wasm_bindgen(js_name = importLibrary)]
    pub async fn import_library(&self, json: String) -> Result<JsValue, JsError> {
        let file = LibraryFile::from_json(&json)?;

        let mut photos_added = 0u32;
        for (id, bytes) in file.carried_photos()? {
            self.photos.restore(&id, &bytes).await?;
            photos_added += 1;
        }

        // One statement, one borrow, no await under it.
        let (mut report, state) = {
            let mut app = self.inner.borrow_mut();
            let report = app.import_library(&file)?;
            let state = app.state()?;
            (report, state)
        };
        report.photos_added = photos_added;

        to_js(&Imported { report, state })
    }

    // --- sync ---------------------------------------------------------------
    //
    // The socket is the frontend's (DECISIONS 0043): these calls are what it
    // drives it with. Bytes cross as `Uint8Array` in both directions and are
    // opaque on the JS side — sealed frames outbound, wire messages inbound.
    // Plaintext never appears here: a frame that opens is merged inside the
    // core and what comes back is a state object like any other.

    /// A new group's recovery phrase. Called once, on the device that starts
    /// the group; the phrase is then the only secret there is (0042).
    #[wasm_bindgen(js_name = mintPhrase)]
    pub fn mint_phrase() -> Result<String, JsError> {
        Ok(crate::sync::mint_phrase()?)
    }

    /// The canonical spelling of a phrase that was typed or scanned, or an
    /// error whose message is meant to be shown next to the field (0021).
    #[wasm_bindgen(js_name = readPhrase)]
    pub fn read_phrase(phrase: &str) -> Result<String, JsError> {
        Ok(crate::sync::read_phrase(phrase)?)
    }

    /// Starts a connection and returns the hello to send on it.
    ///
    /// `cursor` is what [`CabasApp::sync_status`] returned when the last
    /// connection ended — `{ epoch: 0, since: 0 }` on a device that has never
    /// synced. Any session already open is dropped: a socket that is being
    /// replaced has nothing left to say.
    #[wasm_bindgen(js_name = syncHello)]
    pub fn sync_hello(&self, phrase: &str, cursor: JsValue) -> Result<Vec<u8>, JsError> {
        let cursor: SyncCursor = serde_wasm_bindgen::from_value(cursor)?;
        let session = SyncSession::open(phrase, cursor)?;
        let hello = session.hello()?;
        *self.session.borrow_mut() = Some(session);
        Ok(hello)
    }

    /// Feeds one binary message from the socket in, and gets one `SyncEvent`
    /// out. A frame that opens is merged before this returns, so a `merged`
    /// event already carries the new state.
    #[wasm_bindgen(js_name = syncHandle)]
    pub fn sync_handle(&self, wire: &[u8]) -> Result<JsValue, JsError> {
        let mut session = self.session.borrow_mut();
        let session = session.as_mut().ok_or_else(no_session)?;
        let mut app = self.inner.borrow_mut();
        to_js(&session.handle(&mut app, wire)?)
    }

    /// Seals everything this replica has produced since `shadow` into a push.
    ///
    /// `shadow` is the version this device last had acked — an empty array on
    /// a device that has never pushed, which means "everything I know".
    #[wasm_bindgen(js_name = syncPush)]
    pub fn sync_push(&self, shadow: &[u8]) -> Result<Vec<u8>, JsError> {
        let session = self.session.borrow();
        let session = session.as_ref().ok_or_else(no_session)?;
        Ok(session.push(&self.inner.borrow(), shadow)?)
    }

    /// Seals the whole replica into a push that truncates the relay's log.
    /// For a device that just sat through a long replay (`replayed` in
    /// [`CabasApp::sync_status`]) — compaction is device-driven, because the
    /// relay cannot merge what it cannot read (0042).
    #[wasm_bindgen(js_name = syncSnapshot)]
    pub fn sync_snapshot(&self) -> Result<Vec<u8>, JsError> {
        let session = self.session.borrow();
        let session = session.as_ref().ok_or_else(no_session)?;
        Ok(session.snapshot(&self.inner.borrow())?)
    }

    /// The replica's version right now — the shadow to adopt once the push
    /// that carried it is acked. Read it *before* sending that push.
    #[wasm_bindgen(js_name = syncVersion)]
    pub fn sync_version(&self) -> Vec<u8> {
        self.inner.borrow().version()
    }

    /// Whether this replica was created from nothing at launch. A stored
    /// cursor from a previous life must not be resumed on one — see
    /// [`crate::app::App::opened_fresh`].
    #[wasm_bindgen(js_name = openedFresh)]
    pub fn opened_fresh(&self) -> bool {
        self.inner.borrow().opened_fresh()
    }

    /// The cursor to persist, and the two counters a diagnostics screen shows.
    /// `null` when no connection is open.
    #[wasm_bindgen(js_name = syncStatus)]
    pub fn sync_status(&self) -> Result<JsValue, JsError> {
        match self.session.borrow().as_ref() {
            Some(session) => to_js(&session.status()),
            None => Ok(JsValue::NULL),
        }
    }

    /// Drops the session. Called when the socket closes, for the reason the
    /// field exists: the next connection derives the key again from the
    /// phrase, so there is no reason to keep this one in memory meanwhile.
    #[wasm_bindgen(js_name = syncClose)]
    pub fn sync_close(&self) {
        *self.session.borrow_mut() = None;
    }
}

/// Calling a session method with no socket open is a bug in the engine
/// driving it, not a condition the UI can recover from — so it says which
/// call was skipped rather than returning a silent `null`.
fn no_session() -> JsError {
    JsError::new("no sync session: syncHello has not been called on this connection")
}
