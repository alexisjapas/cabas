//! The one error type this crate exposes.
//!
//! Every variant carries a `String` rather than the underlying CRDT error,
//! and that is the point: a public enum holding a `LoroError` would put a
//! Loro type in the signature of every caller in `app` and `sync`, which is
//! exactly what Rule 2 forbids. Containment has to hold on the error path
//! too, or swapping the CRDT stops being a one-crate rewrite.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, StoreError>;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StoreError {
    /// The CRDT refused an operation. In practice this means a detached
    /// container or a genuine bug on our side, not user input.
    #[error("crdt: {0}")]
    Crdt(String),

    /// A snapshot could not be produced or consumed — a truncated file, a
    /// blob from a different tool, or a corrupted transfer.
    #[error("snapshot: {0}")]
    Snapshot(String),

    /// The document does not hold what the schema says it should: a missing
    /// key, a value of the wrong type, an unparsable rational, an enum tag
    /// this build has never heard of.
    #[error("corrupt document at {path}: {detail}")]
    Corrupt { path: String, detail: String },

    /// The storage backend refused. A missing document is **not** one of
    /// these — a first run has nothing to load, and that is a `None`, not a
    /// failure.
    #[error("storage: {0}")]
    Io(String),

    /// Written by a newer build. Refusing beats guessing: a partial read
    /// would silently drop the fields this version cannot see, and the next
    /// save would then propagate that loss to every other device.
    #[error(
        "document schema v{found} was written by a newer version of cabas \
         (this build understands v{supported}) — update the app"
    )]
    FutureSchema { found: i64, supported: i64 },
}

impl StoreError {
    /// The document is not shaped the way the schema says.
    pub(crate) fn corrupt(path: impl Into<String>, detail: impl Into<String>) -> Self {
        StoreError::Corrupt {
            path: path.into(),
            detail: detail.into(),
        }
    }
}

/// Wraps a CRDT failure by its message.
///
/// Takes `impl Display` rather than the concrete error so that no Loro type
/// appears in a signature, not even a private one that a future refactor
/// might widen (Rule 2).
pub(crate) fn crdt(error: impl std::fmt::Display) -> StoreError {
    StoreError::Crdt(error.to_string())
}

/// Same, for the encode/decode path.
pub(crate) fn snapshot(error: impl std::fmt::Display) -> StoreError {
    StoreError::Snapshot(error.to_string())
}
