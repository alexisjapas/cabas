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
//!
//! One wrinkle the host has to know about: when `status().reset` is set, the
//! relay is serving a log that does not contain what this device's cursor
//! claimed — a restored backup, most often — and the push at `CaughtUp` is
//! owed **even if nothing changed locally**, because the shadow is a claim
//! about a log that no longer exists (DECISIONS 0054). What goes out in that
//! case is a whole replica; [`SyncSession::push`] decides that on its own.

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
    ///
    /// **Text across the boundary, and only there** (DECISIONS 0046). The
    /// relay mints it from 64 bits of the OS's randomness, so almost every
    /// epoch there is falls outside what a JavaScript number holds exactly,
    /// and `serde_wasm_bindgen` is right to refuse it rather than round it.
    /// The host stores this value and hands it back; it never does arithmetic
    /// on it. Same answer `store` gives an exact rational, for the same
    /// reason (DECISIONS 0029).
    #[serde(with = "text_u64")]
    #[cfg_attr(feature = "typescript", ts(type = "string"))]
    pub epoch: u64,
    /// The last sequence number applied. A plain number: the relay hands
    /// these out one per frame from 1, so reaching the point where a double
    /// stops being exact would take more frames than a family will ever
    /// produce.
    pub since: u64,
}

/// A `u64` that survives JavaScript: decimal text in, decimal text out.
mod text_u64 {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(D::Error::custom)
    }
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
    /// The relay served a log that does not hold what the cursor claimed —
    /// restored from a backup, or reset. The host reads this to know that its
    /// shadow is void and that it owes the family a push even if nothing
    /// changed locally (DECISIONS 0054); [`SyncSession::push`] acts on it by
    /// itself, so nothing has to be recomputed from it.
    pub reset: bool,
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
    ///
    /// # After a reset, `shadow` is ignored
    ///
    /// A shadow means "the relay already holds everything up to here", and a
    /// restored backup makes that false without changing anything the shadow
    /// can see: the epoch is inside `/data`, so it comes back identical
    /// (DECISIONS 0053). A delta measured from such a shadow is *causally*
    /// dangling — it names operations the log no longer carries — so a device
    /// that missed that window accepts the frame, advances its cursor, and
    /// applies nothing, for good and with no error anywhere. Meanwhile the
    /// device holding those operations never offers them again, because as
    /// far as its shadow knows they were delivered.
    ///
    /// So a session that saw a reset pushes the **whole replica** instead. It
    /// is self-contained by construction, which is what lets the relay
    /// truncate to it, and it puts the rolled-back window back into the log —
    /// the recovery point that is supposed to survive every device
    /// (DECISIONS 0054).
    pub fn push<S: Storage, P: Platform>(&self, app: &App<S, P>, shadow: &[u8]) -> Result<Vec<u8>> {
        if self.inner.reset() {
            return self.snapshot(app);
        }
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
            reset: self.inner.reset(),
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
