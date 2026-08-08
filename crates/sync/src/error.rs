//! The one error type this crate exposes.
//!
//! Nothing here wraps a vendor error type: `bip39`, `chacha20poly1305` and
//! `postcard` failures all cross this boundary as strings or as nothing at
//! all, for the same reason `store` never exposes a `LoroError` — the cipher
//! and the codec must stay swappable without touching a caller (Rule 7 keeps
//! all cryptography in this crate; this keeps this crate's choices in it
//! too).

use thiserror::Error;

pub type Result<T> = std::result::Result<T, SyncError>;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SyncError {
    /// The recovery phrase does not decode: wrong word count, a word not in
    /// the list, or a checksum that says a word was swapped or mistyped.
    /// The message is meant to be shown to the person retyping it.
    #[error("phrase: {0}")]
    Phrase(String),

    /// A sealed frame failed to open. Deliberately detail-free: the AEAD
    /// gives one bit ("wrong"), and inventing distinctions between a wrong
    /// key, tampering and truncation would be guessing.
    #[error("a sealed frame failed to open")]
    Open,

    /// Sealing failed. In practice unreachable — the cipher only refuses a
    /// plaintext too large to fit its counter, orders of magnitude beyond a
    /// family library.
    #[error("sealing failed")]
    Seal,

    /// The platform refused to hand out randomness. On wasm this means the
    /// `getrandom` backend opt-ins are missing (a compile-time error today);
    /// natively it means something is deeply wrong with the OS.
    #[error("randomness unavailable: {0}")]
    Entropy(String),

    /// A wire message did not decode. A relay and a device speaking
    /// different protocol versions surface here, which is why `Hello`
    /// carries a version byte: the refusal is clean instead of a misparse.
    #[error("wire: {0}")]
    Wire(String),
}
