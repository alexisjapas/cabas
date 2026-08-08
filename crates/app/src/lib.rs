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
//! Implementation lands in M3 (see ROADMAP.md).

#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn crate_builds() {}
}
