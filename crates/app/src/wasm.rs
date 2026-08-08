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

use cabas_store::{IndexedDbStorage, Storage};
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::app::App;
use crate::command::Command;
use crate::platform::{Identity, SystemPlatform};

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
}

#[wasm_bindgen]
impl CabasApp {
    /// Mints the ids for a device that has never run before.
    ///
    /// Called **once**, by the host, which then persists the result and hands
    /// it to [`CabasApp::open`] on every launch. Where a device remembers
    /// things about itself is the host's business — `localStorage` here —
    /// and the family document holds only the records these ids point at.
    #[wasm_bindgen(js_name = mintIdentity)]
    pub fn mint_identity(user_name: String, device_name: String) -> Result<JsValue, JsError> {
        let identity = Identity::mint(&SystemPlatform, user_name, device_name)?;
        to_js(&identity)
    }

    /// Opens the replica stored in IndexedDB, or starts a new one.
    pub async fn open(identity: JsValue) -> Result<CabasApp, JsError> {
        let identity: Identity = serde_wasm_bindgen::from_value(identity)?;
        let storage = IndexedDbStorage::new();
        let app = App::open(storage.clone(), SystemPlatform, identity).await?;
        Ok(Self {
            inner: RefCell::new(app),
            storage,
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
}
