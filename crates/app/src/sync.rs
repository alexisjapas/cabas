//! The client sync loop, minus the socket.
//!
//! [`cabas_sync::Session`] turns wire bytes into instructions and local bytes
//! into sealed frames, and deliberately stops there — it never sees a replica
//! (DECISIONS 0042). This module is the other half: what those instructions
//! mean to an [`App`]. It exists so the two hosts that will drive it — the
//! PWA's TypeScript engine through [`crate::wasm`], Tauri's event loop at M7 —
//! share one answer to "a frame arrived, now what" instead of writing it twice
//! (DECISIONS 0043).
//!
//! # Plaintext stops here
//!
//! A frame that opens is merged **inside** [`SyncSession::handle`], and what
//! comes back is the same whole `StateView` every other mutation produces
//! (Rule 9). No document bytes cross to the host, which is what lets the wasm
//! binding stay pure translation and keeps the frontend unable to hold
//! something it should not.
//!
//! # What the caller still owns
//!
//! The socket, the reconnection policy, and four values that outlive a
//! connection: the phrase, the relay URL, the [`SyncCursor`] and the shadow
//! version. They are device-local — never synced, never in the document — and
//! on the PWA they sit in `localStorage` next to the identity (DECISIONS 0031,
//! 0043).
//!
//! The loop, in the order a connection runs it:
//!
//! ```text
//! socket opens        → hello()            → send
//! every message       → handle(app, wire)  → render the state it returns
//! CaughtUp            → push(app, shadow)  → send, then wait for Acked
//! Acked               → shadow = version taken *before* the push
//! anything moved      → persist status().cursor
//! ```

use serde::{Deserialize, Serialize};

use cabas_store::Storage;
use cabas_sync::{Event, FamilyKey, Session};

use crate::app::App;
use crate::error::Result;
use crate::platform::Platform;
use crate::view::StateView;

/// What a device remembers between connections: which log it was reading, and
/// how far into it (DECISIONS 0042). Zeros on a device that has never synced,
/// which is also what [`Default`] gives.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct SyncCursor {
    /// The relay's log identity. A different one means the log this cursor
    /// pointed into is gone — restored from a backup, or reset — and the
    /// session replays from the beginning rather than from `since`.
    pub epoch: u64,
    /// The last sequence number applied.
    pub since: u64,
}

/// The cursor, plus what this one connection has seen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct SyncStatus {
    /// The part worth persisting, and the part to hand back to
    /// [`SyncSession::open`] next time.
    pub cursor: SyncCursor,
    /// Frames applied on this connection, replay and live alike. A long
    /// replay is the signal to volunteer a [`SyncSession::snapshot`], since
    /// compaction is device-driven — the relay cannot merge what it cannot
    /// read (0042).
    pub replayed: u64,
    /// Frames the family key refused to open. Nonzero is either corruption or
    /// company: someone holding the family id, which the relay stores in the
    /// clear, but not the phrase.
    pub dropped: u64,
}

/// One server message, after the replica has been brought up to date with it.
///
/// Tagged the way commands are, because the frontend switches on it: the tag
/// is `event`, the variants are `snake_case`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum SyncEvent {
    /// The relay answered the hello. The cursor may have been reset; read it
    /// back from [`SyncSession::status`] rather than assuming.
    Connected,

    /// A frame opened and was merged. `state` is the whole new screen —
    /// render it, and let the ordinary save policy write the replica.
    ///
    /// Boxed because it dwarfs every other variant, and an enum is as large
    /// as its widest arm.
    Merged { state: Box<StateView> },

    /// The replay is exhausted: everything the relay held has been applied.
    /// This is the moment to push what this device produced offline.
    CaughtUp,

    /// A push is durable. Advance the shadow to the version read before that
    /// push went out.
    Acked { seq: u64 },

    /// The relay hung up with a reason. Reconnecting without changing
    /// something — the protocol version, the family — will not help.
    Refused { reason: String },

    /// A frame did not open and was stepped over. Not fatal, and not worth
    /// refetching: it would not open the second time either (0042).
    Dropped { seq: u64 },
}

/// One connection's worth of client state.
///
/// It owns no replica: the [`App`] is passed in on the calls that need it, so
/// a host can hold the two independently and `App` stays unaware that sync
/// exists at all.
pub struct SyncSession {
    inner: Session,
}

impl SyncSession {
    /// Derives the family key from the phrase and resumes at `cursor`.
    ///
    /// The only failure is a phrase that does not decode — wrong word count,
    /// a word off the list, a checksum that says one was mistyped — and its
    /// message is written to be shown to whoever is retyping it (0021).
    pub fn open(phrase: &str, cursor: SyncCursor) -> Result<Self> {
        let key = FamilyKey::from_phrase(phrase)?;
        Ok(SyncSession {
            inner: Session::new(key, cursor.epoch, cursor.since),
        })
    }

    /// The opening message: send it as soon as the socket is open, then feed
    /// every binary message to [`SyncSession::handle`].
    pub fn hello(&self) -> Result<Vec<u8>> {
        Ok(self.inner.hello()?)
    }

    /// One incoming message, applied.
    pub fn handle<S: Storage, P: Platform>(
        &mut self,
        app: &mut App<S, P>,
        wire: &[u8],
    ) -> Result<SyncEvent> {
        Ok(match self.inner.handle(wire)? {
            Event::Connected => SyncEvent::Connected,
            Event::Merge(plaintext) => SyncEvent::Merged {
                state: Box::new(app.merge(&plaintext)?),
            },
            Event::CaughtUp => SyncEvent::CaughtUp,
            Event::Acked { seq } => SyncEvent::Acked { seq },
            Event::Refused { reason } => SyncEvent::Refused { reason },
            Event::Dropped { seq } => SyncEvent::Dropped { seq },
        })
    }

    /// Seals everything the replica has produced since `shadow` — the version
    /// [`App::version`] returned when the last push was acked.
    ///
    /// Read the new version *before* sending and adopt it only on the ack:
    /// a command applied while the push is in flight then stays unpushed
    /// rather than being counted as sent. `pending_snapshot` guards the save
    /// path with the same reasoning.
    pub fn push<S: Storage, P: Platform>(&self, app: &App<S, P>, shadow: &[u8]) -> Result<Vec<u8>> {
        Ok(self.inner.delta(&app.changes_since(shadow)?)?)
    }

    /// Seals the whole replica into a push that lets the relay drop every
    /// frame this session has applied.
    ///
    /// The empty version means "since the beginning", so what goes out is a
    /// document a fresh device can merge on its own — which is the property
    /// that makes truncating the log safe.
    pub fn snapshot<S: Storage, P: Platform>(&self, app: &App<S, P>) -> Result<Vec<u8>> {
        Ok(self.inner.snapshot(&app.changes_since(&[])?)?)
    }

    /// Where this session has got to. The cursor half is what to persist,
    /// whenever it moves.
    pub fn status(&self) -> SyncStatus {
        let (epoch, since) = self.inner.cursor();
        SyncStatus {
            cursor: SyncCursor { epoch, since },
            replayed: self.inner.replayed(),
            dropped: self.inner.dropped(),
        }
    }
}

/// A new family: twelve words, from the OS's entropy (0042).
///
/// Called once ever, by the device that starts the family. Every other device
/// joins by scanning or typing the same words, which is why pairing by QR and
/// pairing by hand are one operation with two input methods (0021).
pub fn mint_phrase() -> Result<String> {
    Ok(FamilyKey::generate()?.phrase().to_string())
}

/// The canonical spelling of a phrase as typed or scanned — lowercase, single
/// spaces — or the reason it is not a phrase.
///
/// The pairing screen calls this before storing anything: a phrase that only
/// fails at the first connection would look like the relay being down.
pub fn read_phrase(phrase: &str) -> Result<String> {
    Ok(FamilyKey::from_phrase(phrase)?.phrase().to_string())
}
