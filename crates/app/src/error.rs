//! The one error type the frontend ever sees.
//!
//! Errors reaching here are, almost without exception, *not* the user's
//! doing: a corrupt document, a CRDT refusing an operation, a browser with no
//! source of randomness. The exception is [`AppError::Invalid`], which is what
//! a badly typed quantity in an input field becomes — and which the UI is
//! expected to render next to that field rather than as a failure of the app.
//!
//! Deliberately **not** an error variant: anything a concurrent edit can
//! cause. A list entry whose recipe another device deleted is a
//! [`crate::view::ProblemView`] on the state, not a failed command — the
//! screen has to keep rendering (DECISIONS 0022).

use cabas_domain::CartError;
use cabas_store::StoreError;
use cabas_sync::SyncError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, PartialEq, Eq, Error)]
pub enum AppError {
    /// Persistence or the CRDT. Carries no Loro type — `store` already made
    /// sure of that, and Rule 2 holds on this side of the boundary too.
    #[error(transparent)]
    Store(#[from] StoreError),

    /// A derivation the domain refused. Reaching this is a bug here rather
    /// than bad data: the app filters unusable list entries into
    /// [`crate::view::ProblemView`]s *before* deriving, precisely so the cart
    /// cannot fail as a whole.
    #[error(transparent)]
    Cart(#[from] CartError),

    /// A command named something the document does not hold.
    #[error("no {kind} with id `{id}`")]
    NotFound { kind: &'static str, id: String },

    /// User input that could not be read as what it claims to be — "1,5" is
    /// fine, "one and a half" is not.
    #[error("{field}: {detail}")]
    Invalid { field: &'static str, detail: String },

    /// The host could not provide something only it can: a clock, randomness.
    #[error("platform: {0}")]
    Platform(String),

    /// Sync: a recovery phrase that does not decode, a frame that does not
    /// open, a relay speaking a protocol this build does not. Carries no
    /// vendor type — `sync` already saw to that, for the same reason `store`
    /// never lets a `LoroError` out (Rules 2, 7).
    #[error(transparent)]
    Sync(#[from] SyncError),
}

impl AppError {
    pub(crate) fn invalid(field: &'static str, detail: impl Into<String>) -> Self {
        AppError::Invalid {
            field,
            detail: detail.into(),
        }
    }

    pub(crate) fn not_found(kind: &'static str, id: impl Into<String>) -> Self {
        AppError::NotFound {
            kind,
            id: id.into(),
        }
    }
}
