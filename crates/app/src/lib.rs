//! Commands and view-models — the only surface the frontend ever touches.
//!
//! The frontend renders view-models pushed from here and emits intents. It
//! holds no business state of its own, which is what lets the very same
//! Svelte code run against two different transports without knowing it:
//! `wasm-bindgen` calls in the PWA, Tauri `invoke` on Android and Linux.
//!
//! # Boundaries (CONSTITUTION Rules 8, 9)
//!
//! - **Builds for `wasm32-unknown-unknown` and for the host, always.** This
//!   crate is the one that must never regress on either. No
//!   `std::time::Instant` (use `web-time`), `getrandom` with its web backend,
//!   networking and storage behind traits.
//! - **Coarse-grained API.** A handful of commands and a single state stream,
//!   not a chatty getter surface: every call crosses an FFI or IPC boundary
//!   and every generated type is maintenance.
//! - **No business state escapes.** If the frontend needs a value, it comes
//!   from a view-model computed here.
//!
//! # The shape of it
//!
//! ```text
//! Command  ──►  App::apply   ──►  domain rule ──► store write
//!                    │
//!                    └──────────►  StateView   (the whole screen, every time)
//!
//!               App::persist ──►  Storage      (separately, never blocking a render)
//! ```
//!
//! [`App::apply`] is synchronous and [`App::persist`] is not, which is the
//! one structural decision in this crate: a tick in a shop renders at the
//! speed of a tap, and the storage write happens after (Rule 6). Hosts that
//! want the simple thing call [`App::dispatch`], which does both.
//!
//! # Where the French is
//!
//! Not here. Amounts arrive as text and leave as text, because turning an
//! exact rational into "1 1/2" is arithmetic; but every name — a unit, an
//! aisle, a check state — travels as a **tag** and is labelled by the
//! frontend. The app writes no sentence a user reads. That is what keeps the
//! eventual translation a UI change, and it is the only sense in which Rule 9
//! is relaxed: the frontend owns words, never numbers.

#![forbid(unsafe_code)]

mod app;
pub mod command;
pub mod error;
mod id;
mod library;
mod number;
pub mod platform;
mod project;
pub mod tags;
pub mod view;

#[cfg(target_family = "wasm")]
mod wasm;

pub use app::App;
pub use command::Command;
pub use error::{AppError, Result};
pub use platform::{Identity, Platform, SystemPlatform};
pub use view::StateView;

#[cfg(target_family = "wasm")]
pub use wasm::CabasApp;
