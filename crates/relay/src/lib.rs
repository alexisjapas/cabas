//! Zero-knowledge relay, and host of the PWA.
//!
//! Runs on the RPi4 as a Home Assistant OS add-on. It does two things from
//! a single origin: serve the static PWA bundle — embedded into this binary
//! at build time by `build.rs` (DECISIONS 0048) — and broker encrypted sync
//! between devices. One origin because an installed PWA *is* its origin
//! (DECISIONS 0012).
//!
//! # Boundaries (CONSTITUTION Rules 6, 7)
//!
//! - **It cannot read anything.** Payloads arrive sealed and are stored
//!   sealed. It holds no key — it depends on `cabas-sync` for the protocol
//!   types and the family id, never for the cipher (Rule 7).
//! - **It is stateful, and that is the point.** A pure broadcast relay
//!   would never reconcile two devices that are never online at the same
//!   time — the normal case for a phone in a shop and a laptop at home. It
//!   persists the sealed log and replays it (DECISIONS 0009, 0042).
//! - **`/data` is the durable volume.** Home Assistant's own backups cover
//!   it, which makes the relay the recovery point if every device is lost.
//!
//! The shape is three verbs — append, replay, forward — and `log` and
//! `server` hold one and two of them respectively; `assets` holds the
//! static half, which shares nothing with them but the port. The convergence test
//! in `tests/convergence.rs` is M5's exit criterion: two replicas that are
//! never online at the same time still converge through this process.

#![forbid(unsafe_code)]

mod assets;
mod log;
mod server;

pub use assets::embedded;
pub use server::{Relay, router};
