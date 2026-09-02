//! The Android and Linux edge of the world: JSON in, JSON out.
//!
//! The counterpart of `cabas_app::wasm`, and thin for the same reason —
//! everything here translates and nothing decides, because a decision made
//! here would be a decision the PWA makes differently (Rule 9, DECISIONS
//! 0005, 0093). Read `crates/app/src/wasm.rs` beside this file: the two are
//! the same surface twice, and a method that appears in one and not the
//! other is a bug in whichever is missing it.
//!
//! # Why every command is `async` even where nothing waits
//!
//! Tauri's IPC is promise-based and has no synchronous form, so the
//! TypeScript surface is uniformly asynchronous on both hosts (DECISIONS
//! 0093). `state` and `apply` return in microseconds here and are still
//! commands like any other; the uniformity is the point, since a surface
//! whose timing differs per host is two surfaces wearing one name.
//!
//! # Why a `Mutex` and not a `RefCell`
//!
//! `wasm.rs` runs on one thread and holds its app in a `RefCell`. Tauri runs
//! commands on a thread pool, so managed state must be `Send + Sync` and the
//! same fields are behind [`Mutex`]. **The discipline is identical and it is
//! not the mutex's doing**: a lock is taken and dropped inside a statement
//! that ends, never held across an `.await`. On the PWA that rule stops a
//! second tap panicking at the wasm boundary (0032); here it would be a
//! second tap deadlocking behind a write, which is the same bug in the same
//! shop.
//!
//! The difference is who enforces it. On the PWA nothing does — a borrow held
//! across an await panics at runtime, on a phone, in an aisle. Here a
//! `MutexGuard` held across an `.await` makes the future `!Send`, and
//! `generate_handler!` requires `Send`: **the same mistake is a compile
//! error**. That is worth knowing before anyone "tidies" a two-statement
//! command into one expression.
//!
//! # What is *not* here
//!
//! The sockets. They stay in the frontend on both hosts — the webview has
//! `WebSocket`, `visibilitychange` and `pagehide`, which is the whole of
//! 0043's argument for where that policy lives, and 0093 confirmed it for
//! this host. So `sync_*` and `photo_*` below are byte pumps with no
//! transport under them, exactly as they are in `wasm.rs`.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use cabas_app::{
    App, Command, Identity, LibraryFile, MAX_PHOTO_BYTES, PhotoSync, Photos, StateView, SyncCursor,
    SyncSession, SystemPlatform,
};
use cabas_domain::PhotoId;
use cabas_store::{FilePhotoStore, FileStorage, Storage};
use serde::Serialize;
use tauri::{Manager, State};

/// What a refused command becomes on the way to the frontend.
///
/// A string and nothing else, because that is what the PWA already sends: a
/// `JsError` carries a message and the frontend renders it next to the
/// control that was refused. Giving this host a richer error would give the
/// two platforms different vocabularies for the same refusal.
#[derive(Debug, Serialize)]
pub struct HostError(String);

/// One `From` per error that actually crosses, rather than a blanket
/// `impl<E: Display>`: the blanket one collides with the reflexive
/// `impl<T> From<T> for T` in core and does not compile. Naming them is also
/// the list of what can go wrong in this file, which is short on purpose.
macro_rules! host_error_from {
    ($($ty:ty),* $(,)?) => {
        $(impl From<$ty> for HostError {
            fn from(error: $ty) -> Self {
                Self(error.to_string())
            }
        })*
    };
}

host_error_from!(
    cabas_app::AppError,
    cabas_store::StoreError,
    std::io::Error,
    serde_json::Error,
    tauri::Error,
);

impl From<&str> for HostError {
    fn from(message: &str) -> Self {
        Self(message.to_string())
    }
}

/// A poisoned lock means another command panicked while holding it. The
/// replica is then in an unknown state, so this says so rather than carrying
/// on — and it is generic over the guard because every field has its own.
impl<T> From<std::sync::PoisonError<T>> for HostError {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        Self("the replica is in an unknown state: a command panicked".into())
    }
}

type Answer<T> = std::result::Result<T, HostError>;

/// Everything one launch holds. The fields are `wasm.rs`'s, one for one.
pub struct Host {
    inner: Mutex<Option<App<FileStorage, SystemPlatform>>>,
    /// A second handle, held outside the lock so a save can be awaited
    /// without holding the app across it. It is a *path*, not an open file —
    /// cloning it costs nothing.
    storage: Mutex<Option<FileStorage>>,
    photos: Mutex<Option<Arc<Photos<FilePhotoStore>>>>,
    /// The current sync connection, or `None` between them. One session per
    /// socket: created when the socket opens, dropped when it closes, which
    /// is also when the key leaves memory.
    session: Mutex<Option<SyncSession>>,
    /// The current *photo* connection, on its own socket (DECISIONS 0080).
    photo_session: Mutex<Option<PhotoSync>>,
    /// Where this device keeps its own things: the replica, the photos, and
    /// the identity file. Set once at startup from Tauri's app-data path.
    home: Mutex<Option<PathBuf>>,
}

impl Host {
    fn new() -> Self {
        Self {
            inner: Mutex::new(None),
            storage: Mutex::new(None),
            photos: Mutex::new(None),
            session: Mutex::new(None),
            photo_session: Mutex::new(None),
            home: Mutex::new(None),
        }
    }

    fn home(&self) -> Answer<PathBuf> {
        self.home
            .lock()?
            .clone()
            .ok_or_else(|| HostError("the host has no data directory".into()))
    }

    /// The identity file — the Tauri half of DECISIONS 0031, which named it
    /// and left it unbuilt until now. `localStorage` in the PWA, this here.
    fn identity_path(&self) -> Answer<PathBuf> {
        Ok(self.home()?.join("identity.json"))
    }

    /// A handle on the photo store for the length of one command.
    ///
    /// `Arc::clone`, because `Photos` is not `Clone` and — more to the point —
    /// because the lock must not be held across the awaits that follow. The
    /// store outlives the guard; the guard ends with this statement.
    fn photos(&self) -> Answer<Arc<Photos<FilePhotoStore>>> {
        self.photos
            .lock()?
            .clone()
            .ok_or_else(|| HostError(NOT_OPEN.into()))
    }

    fn storage(&self) -> Answer<FileStorage> {
        self.storage
            .lock()?
            .clone()
            .ok_or_else(|| HostError(NOT_OPEN.into()))
    }
}

const NOT_OPEN: &str = "no replica: open has not been called on this launch";

/// Calling a session method with no socket open is a bug in the engine
/// driving it, not a condition the UI can recover from — so it says which
/// call was skipped rather than answering a silent `null`.
fn no_session() -> HostError {
    HostError("no sync session: syncHello has not been called on this connection".into())
}

/// The same, for the photo socket — and it also covers the shape peculiar to
/// this pair: `photo_handle` and `photo_push` take the session *out* of its
/// cell while they await, so two of them running at once would find it empty.
/// The engine drives them one at a time; this is what says so if it stops.
fn no_photo_session() -> HostError {
    HostError(
        "no photo session: photoHello has not been called on this connection, \
         or two photo calls are in flight at once"
            .into(),
    )
}

// --- the host's own memory (DECISIONS 0031) ---------------------------------

/// The identity this device already has, or `null` on one that has never run.
///
/// A file that does not parse is treated as absent, exactly as a malformed
/// `localStorage` value is on the PWA: the recovery is to mint a new identity,
/// and refusing to start would strand the person on a broken screen with no
/// way out. The cost is a new name in the roster, which is cosmetic (Rule 7).
#[tauri::command]
async fn read_identity(host: State<'_, Host>) -> Answer<Option<Identity>> {
    let path = host.identity_path()?;
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(None);
    };
    Ok(serde_json::from_str(&text).ok())
}

/// Writes the identity down, through a temporary file and a rename.
///
/// The same care `FileStorage` takes with the replica, for a smaller reason
/// that is still fatal: a half-written identity is a device that has
/// forgotten who it is, and the next launch pairs again — a new device id and
/// a dead peer left in the group's history.
#[tauri::command]
async fn remember_identity(host: State<'_, Host>, identity: Identity) -> Answer<()> {
    let path = host.identity_path()?;
    let scratch = path.with_extension("json.new");
    std::fs::write(&scratch, serde_json::to_vec(&identity)?)?;
    std::fs::rename(&scratch, &path)?;
    Ok(())
}

// --- minting, and the two constants (no replica needed) ---------------------

#[tauri::command]
async fn mint_device(device_name: String) -> Answer<Identity> {
    Ok(Identity::mint_device(&SystemPlatform, device_name)?)
}

#[tauri::command]
async fn mint_usage_id() -> Answer<String> {
    Ok(cabas_app::mint_usage_id(&SystemPlatform)?)
}

#[tauri::command]
async fn mint_ingredient_id() -> Answer<String> {
    Ok(cabas_app::mint_ingredient_id(&SystemPlatform)?)
}

#[tauri::command]
async fn mint_shop_id() -> Answer<String> {
    Ok(cabas_app::mint_shop_id(&SystemPlatform)?)
}

/// Which build is running. `CARGO_PKG_VERSION` of *this* crate, which is the
/// workspace version — so an APK reports the same string the relay does, and
/// the two can be held against each other (Rule 15).
#[tauri::command]
async fn build_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
async fn max_photo_bytes() -> usize {
    MAX_PHOTO_BYTES
}

/// Where an Android install syncs unless somebody says otherwise.
///
/// The PWA has no such constant and does not need one: it is served by the
/// relay, so its own origin is the answer (DECISIONS 0012, 0048). This host is
/// served from `http://tauri.localhost`, so the same rule would point both
/// sockets at the app itself — which is exactly what stopped a phone joining
/// its group, and what DECISIONS 0095 is about.
///
/// Compiled in rather than configured, for the reason 0012 makes the origin
/// permanent in the first place: there is one address and changing it is not a
/// setting, it is a migration. `Réglages · Serveur` still overrides it, which
/// is the only door a development relay has on this host.
const HOME_RELAY: &str = "wss://cabas.cladelabs.com/sync";

#[tauri::command]
async fn default_relay() -> &'static str {
    HOME_RELAY
}

#[tauri::command]
async fn mint_phrase() -> Answer<String> {
    Ok(cabas_app::sync::mint_phrase()?)
}

#[tauri::command]
async fn read_phrase(phrase: String) -> Answer<String> {
    Ok(cabas_app::sync::read_phrase(&phrase)?)
}

// --- the replica ------------------------------------------------------------

/// Opens the replica in this device's data directory, or starts a new one.
///
/// The PWA's counterpart is a constructor returning an object; here it fills
/// the managed state, because Tauri's state is set up before any window
/// exists and the frontend decides when there is an identity to open with.
#[tauri::command]
async fn open(host: State<'_, Host>, identity: Identity) -> Answer<()> {
    let home = host.home()?;
    std::fs::create_dir_all(&home)?;
    let photo_dir = home.join("photos");
    std::fs::create_dir_all(&photo_dir)?;

    let storage = FileStorage::new(home.join("replica.loro"));
    let app = App::open(storage.clone(), SystemPlatform, identity).await?;

    *host.inner.lock()? = Some(app);
    *host.storage.lock()? = Some(storage);
    *host.photos.lock()? = Some(Arc::new(Photos::new(FilePhotoStore::new(photo_dir))));
    Ok(())
}

#[tauri::command]
async fn state(host: State<'_, Host>) -> Answer<StateView> {
    let app = host.inner.lock()?;
    Ok(app.as_ref().ok_or(NOT_OPEN)?.state()?)
}

/// Applies one command and answers with the whole new state.
#[tauri::command]
async fn apply(host: State<'_, Host>, command: Command) -> Answer<StateView> {
    let mut app = host.inner.lock()?;
    Ok(app.as_mut().ok_or(NOT_OPEN)?.apply(command)?)
}

#[tauri::command]
async fn identity(host: State<'_, Host>) -> Answer<Identity> {
    let app = host.inner.lock()?;
    Ok(app.as_ref().ok_or(NOT_OPEN)?.identity().clone())
}

#[tauri::command]
async fn opened_fresh(host: State<'_, Host>) -> Answer<bool> {
    let app = host.inner.lock()?;
    Ok(app.as_ref().ok_or(NOT_OPEN)?.opened_fresh())
}

/// Writes the replica if anything has changed since the last write.
///
/// The lock is taken and dropped in the first statement, before the write is
/// awaited — the rule stated in the module note, and the one that keeps a
/// second tap from waiting on a disk.
#[tauri::command]
async fn flush(host: State<'_, Host>) -> Answer<bool> {
    let pending = {
        let mut app = host.inner.lock()?;
        app.as_mut().ok_or(NOT_OPEN)?.pending_snapshot()?
    };
    let Some((snapshot, revision)) = pending else {
        return Ok(false);
    };
    host.storage()?.save(&snapshot).await?;
    host.inner
        .lock()?
        .as_mut()
        .ok_or(NOT_OPEN)?
        .mark_saved(revision);
    Ok(true)
}

// --- photos (DECISIONS 0062) ------------------------------------------------
//
// A store of their own, beside the document: the document is rewritten whole
// on every save and a photo library inside it would be rewritten with it.
// These calls await a file, so — like `flush` — they hold no lock across one.

/// Stores a photo and answers with its id, to be put on the ingredient or the
/// recipe by the ordinary save that follows.
#[tauri::command]
async fn put_photo(host: State<'_, Host>, bytes: Vec<u8>) -> Answer<String> {
    let photos = host.photos()?;
    Ok(photos.put(&SystemPlatform, &bytes).await?.to_string())
}

/// The bytes of a photo this device holds, or nothing.
///
/// Absent is an ordinary answer: a photo taken on the other phone is named by
/// the document from the moment the replicas merge, and its bytes arrive on
/// their own afterwards. The screen shows a placeholder, never an error
/// (Rule 6).
#[tauri::command]
async fn photo(host: State<'_, Host>, id: String) -> Answer<Option<Vec<u8>>> {
    let photos = host.photos()?;
    Ok(photos.get(&PhotoId::from_raw(id)).await?)
}

#[tauri::command]
async fn missing_photos(host: State<'_, Host>) -> Answer<Vec<String>> {
    let referenced = {
        let app = host.inner.lock()?;
        app.as_ref().ok_or(NOT_OPEN)?.referenced_photos()?
    };
    let photos = host.photos()?;
    Ok(photos
        .missing(&referenced)
        .await?
        .into_iter()
        .map(|id| id.to_string())
        .collect())
}

// --- the library as a file (DECISIONS 0076) ---------------------------------

/// The whole library as JSON text, for the host to hand to a share sheet or a
/// download.
///
/// `with_photos` is not a detail. Without them the file is a few tens of
/// kilobytes and can be read in any text editor — the point of it being JSON.
/// With them it carries every picture base64'd, and is a backup.
#[tauri::command]
async fn export_library(host: State<'_, Host>, with_photos: bool) -> Answer<String> {
    let mut file = {
        let app = host.inner.lock()?;
        app.as_ref().ok_or(NOT_OPEN)?.export_library()?
    };
    if with_photos {
        let photos = host.photos()?;
        for id in file.photo_ids() {
            if let Some(bytes) = photos.get(&id).await? {
                file.attach_photo(&id, &bytes);
            }
            // A photo this device does not hold is skipped in silence: it was
            // taken on the other phone and its bytes have not arrived. The
            // entity keeps naming it, so the file stays truthful.
        }
    }
    Ok(file.to_json()?)
}

/// Merges a file in and answers with `{ report, state }`.
///
/// Photos go in **first**, before a single entity is written: that is what
/// makes a file with one unreadable picture fail with nothing changed, rather
/// than with a library half merged behind a photo nobody can open.
#[tauri::command]
async fn import_library(host: State<'_, Host>, json: String) -> Answer<cabas_app::Imported> {
    let file = LibraryFile::from_json(&json)?;

    let photos = host.photos()?;
    let mut photos_added = 0u32;
    for (id, bytes) in file.carried_photos()? {
        photos.restore(&id, &bytes).await?;
        photos_added += 1;
    }

    // One statement, one lock, no await under it.
    let (mut report, state) = {
        let mut app = host.inner.lock()?;
        let app = app.as_mut().ok_or(NOT_OPEN)?;
        let report = app.import_library(&file)?;
        let state = app.state()?;
        (report, state)
    };
    report.photos_added = photos_added;

    Ok(cabas_app::Imported { report, state })
}

// --- sync -------------------------------------------------------------------
//
// The socket is the frontend's on this host too (DECISIONS 0043, 0093): these
// are the calls it drives it with. Bytes are opaque in both directions —
// sealed frames outbound, wire messages inbound — and plaintext never appears
// here: a frame that opens is merged inside the core, and what comes back is
// a state object like any other.

#[tauri::command]
async fn sync_hello(host: State<'_, Host>, phrase: String, cursor: SyncCursor) -> Answer<Vec<u8>> {
    let session = SyncSession::open(&phrase, cursor)?;
    let hello = session.hello()?;
    *host.session.lock()? = Some(session);
    Ok(hello)
}

#[tauri::command]
async fn sync_handle(host: State<'_, Host>, wire: Vec<u8>) -> Answer<cabas_app::SyncEvent> {
    let mut session = host.session.lock()?;
    let session = session.as_mut().ok_or_else(no_session)?;
    let mut app = host.inner.lock()?;
    Ok(session.handle(app.as_mut().ok_or(NOT_OPEN)?, &wire)?)
}

#[tauri::command]
async fn sync_push(host: State<'_, Host>, shadow: Vec<u8>) -> Answer<Vec<u8>> {
    let session = host.session.lock()?;
    let session = session.as_ref().ok_or_else(no_session)?;
    let app = host.inner.lock()?;
    Ok(session.push(app.as_ref().ok_or(NOT_OPEN)?, &shadow)?)
}

#[tauri::command]
async fn sync_snapshot(host: State<'_, Host>) -> Answer<Vec<u8>> {
    let session = host.session.lock()?;
    let session = session.as_ref().ok_or_else(no_session)?;
    let app = host.inner.lock()?;
    Ok(session.snapshot(app.as_ref().ok_or(NOT_OPEN)?)?)
}

#[tauri::command]
async fn sync_version(host: State<'_, Host>) -> Answer<Vec<u8>> {
    let app = host.inner.lock()?;
    Ok(app.as_ref().ok_or(NOT_OPEN)?.version())
}

#[tauri::command]
async fn sync_status(host: State<'_, Host>) -> Answer<Option<cabas_app::SyncStatus>> {
    Ok(host.session.lock()?.as_ref().map(SyncSession::status))
}

/// Drops the session, for the reason the field exists: the next connection
/// derives the key again from the phrase, so there is no reason to keep this
/// one in memory meanwhile.
#[tauri::command]
async fn sync_close(host: State<'_, Host>) -> Answer<()> {
    *host.session.lock()? = None;
    Ok(())
}

// --- the photo transfer (DECISIONS 0080, 0092) ------------------------------
//
// A second socket with the same shape as the one above. What differs is that
// every call touches a file, which is exactly why photos are not on `/sync`:
// a tick in a shop must not queue behind a picture of a jar (Rule 6).
//
// **The session is taken out of its cell and put back**, rather than held
// across the await — the same shape `wasm.rs` uses, and the reason the
// frontend chains these calls one at a time.

#[tauri::command]
async fn photo_hello(host: State<'_, Host>, phrase: String) -> Answer<Vec<u8>> {
    let referenced = {
        let app = host.inner.lock()?;
        app.as_ref().ok_or(NOT_OPEN)?.referenced_photos()?
    };
    let photos = host.photos()?;
    let session = PhotoSync::open(&phrase, &photos, &referenced).await?;
    let hello = session.hello()?;
    *host.photo_session.lock()? = Some(session);
    Ok(hello)
}

#[tauri::command]
async fn photo_handle(host: State<'_, Host>, wire: Vec<u8>) -> Answer<cabas_app::PhotoEvent> {
    let photos = host.photos()?;
    let mut session = host
        .photo_session
        .lock()?
        .take()
        .ok_or_else(no_photo_session)?;
    let outcome = session.handle(&photos, &wire).await;
    // Put back before the `?`: a message this build could not make sense of
    // must not also lose the connection its queue is on.
    *host.photo_session.lock()? = Some(session);
    Ok(outcome?)
}

#[tauri::command]
async fn photo_fetch(host: State<'_, Host>) -> Answer<Option<Vec<u8>>> {
    let mut session = host.photo_session.lock()?;
    let session = session.as_mut().ok_or_else(no_photo_session)?;
    Ok(session.fetch()?)
}

#[tauri::command]
async fn photo_push(host: State<'_, Host>) -> Answer<Option<Vec<u8>>> {
    let photos = host.photos()?;
    let mut session = host
        .photo_session
        .lock()?
        .take()
        .ok_or_else(no_photo_session)?;
    let outcome = session.push(&photos).await;
    *host.photo_session.lock()? = Some(session);
    Ok(outcome?)
}

#[tauri::command]
async fn photo_status(host: State<'_, Host>) -> Answer<Option<cabas_app::PhotoStatus>> {
    Ok(host.photo_session.lock()?.as_ref().map(PhotoSync::status))
}

#[tauri::command]
async fn photo_close(host: State<'_, Host>) -> Answer<()> {
    *host.photo_session.lock()? = None;
    Ok(())
}

/// The entry point both `main.rs` and the Android Gradle project call.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Host::new())
        .setup(|app| {
            // Tauri's own per-platform answer to "where may this app write":
            // an app-private directory on Android, `~/.local/share/…` on
            // Linux. The core is told a path and knows nothing else about it,
            // which is what keeps `FileStorage` the same type on both.
            let home = app.path().app_data_dir()?;
            *app.state::<Host>().home.lock().map_err(|_| "poisoned")? = Some(home);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            read_identity,
            remember_identity,
            mint_device,
            mint_usage_id,
            mint_ingredient_id,
            mint_shop_id,
            build_version,
            max_photo_bytes,
            default_relay,
            mint_phrase,
            read_phrase,
            open,
            state,
            apply,
            identity,
            opened_fresh,
            flush,
            put_photo,
            photo,
            missing_photos,
            export_library,
            import_library,
            sync_hello,
            sync_handle,
            sync_push,
            sync_snapshot,
            sync_version,
            sync_status,
            sync_close,
            photo_hello,
            photo_handle,
            photo_fetch,
            photo_push,
            photo_status,
            photo_close,
        ])
        .run(tauri::generate_context!())
        .expect("cabas: the Tauri host failed to start");
}
