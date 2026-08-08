//! Replicated state: CRDT schema, snapshots and persistence.
//!
//! Every device holds a full replica of the family document (recipes,
//! ingredients, the single shopping list, the check overlay, users/devices
//! and the event log). Concurrent edits converge without a server arbitrating
//! — the case that matters is one person checking items in the shop while the
//! other adds to the list from home.
//!
//! # Boundaries (CONSTITUTION Rules 2, 3, 6)
//!
//! - **Loro stops here.** No Loro type crosses into `cabas-domain`,
//!   `cabas-app`, `cabas-sync` or the UI; this crate translates in both
//!   directions. That containment is what keeps swapping the CRDT a
//!   one-crate rewrite.
//! - **Sources only.** Recipes, ingredients, the list, the *explicit* check
//!   overlay, users, devices and the event log are persisted and synced. The
//!   cart is a pure derivation and is never stored.
//! - **Storage is a trait.** A file on Tauri/relay, IndexedDB on the PWA.
//!   Data volume is small enough (a family library is well under a megabyte)
//!   that a serialized snapshot beats a relational store, and it removes
//!   schema migrations and a SQLite build on four targets.
//!
//! Implementation lands in M2 (see ROADMAP.md).

#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn crate_builds() {}
}
