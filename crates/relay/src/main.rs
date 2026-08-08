//! Zero-knowledge relay, and host of the PWA.
//!
//! Runs on the RPi4 as a Home Assistant OS add-on. It does two things from a
//! single origin: serve the static PWA bundle (embedded in this binary at
//! build time) and broker encrypted sync messages between devices.
//!
//! # Boundaries (CONSTITUTION Rules 6, 7)
//!
//! - **It cannot read anything.** Payloads arrive sealed and are stored
//!   sealed. It holds no key.
//! - **It is stateful, and that is the point.** A pure broadcast relay would
//!   never reconcile two devices that are never online at the same time — the
//!   normal case for a phone in a shop and a laptop at home. It persists the
//!   encrypted snapshot and the deltas.
//! - **`/data` is the durable volume.** Home Assistant's own backups cover
//!   it, which makes the relay the recovery point if every device is lost.
//!
//! Implementation lands in M5, packaging in M6 (see ROADMAP.md).

#![forbid(unsafe_code)]

fn main() {
    eprintln!("cabas-relay: not implemented yet (M5) — see ROADMAP.md");
}
