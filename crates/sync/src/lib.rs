//! Encrypted transport between devices.
//!
//! One symmetric key per family, shared by every paired device. Sync
//! messages are sealed before they leave the device, so the relay stores
//! and forwards ciphertext it cannot read.
//!
//! # Boundaries (CONSTITUTION Rules 6, 7, 8)
//!
//! - **All cryptography lives here.** No other crate seals, opens or derives
//!   a key. [`seal`] is one cipher (XChaCha20-Poly1305) and [`key`] one
//!   derivation (the BIP39 seed of the 12-word phrase, DECISIONS 0042).
//! - **The relay is zero-knowledge.** Plaintext never leaves the device.
//!   `cabas-relay` depends on this crate for the [`protocol`] types and
//!   never for a key: what it cannot name it cannot misuse.
//! - **Declarative attribution.** A single shared key means anyone holding
//!   it can write as anyone. `checked_by` / `added_by` answer "who did
//!   this" between two people who trust each other; they are not access
//!   control, and signatures would not change that under this threat model
//!   (DECISIONS 0024).
//! - **Two transports, one client.** [`Session`] is sans-IO — cursor
//!   discipline, sealing and the epoch reset as plain calls on bytes — so
//!   `tokio-tungstenite` natively and `ws_stream_wasm` on the PWA stay thin
//!   adapters next to their event loops. No socket type appears in this
//!   crate's public API, and neither does `std::time::Instant`.
//!
//! The relay side of the protocol lives in `cabas-relay`; the convergence
//! test that drives both ends is `crates/relay/tests/convergence.rs`.

#![forbid(unsafe_code)]

pub mod error;
pub mod key;
pub mod protocol;
mod seal;
mod session;

pub use error::{Result, SyncError};
pub use key::{FamilyId, FamilyKey, PHRASE_WORDS};
pub use session::{Event, Session};
