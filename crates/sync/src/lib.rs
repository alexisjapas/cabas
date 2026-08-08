//! Encrypted transport between devices.
//!
//! One symmetric key per family, shared by every paired device. Sync messages
//! are sealed before they leave the device, so the relay stores and forwards
//! ciphertext it cannot read.
//!
//! # Boundaries (CONSTITUTION Rules 6, 7, 8)
//!
//! - **All cryptography lives here.** No other crate seals, opens or derives
//!   a key.
//! - **The relay is zero-knowledge.** Plaintext never leaves the device. This
//!   is what makes the hosting choice a matter of convenience rather than of
//!   trust.
//! - **Declarative attribution.** A single shared key means anyone holding it
//!   can write as anyone. `checked_by` / `added_by` answer "who did this"
//!   between two people who trust each other; they are not access control,
//!   and signatures would not change that under this threat model
//!   (DECISIONS 0024).
//! - **Two transports, one trait.** `tokio-tungstenite` natively,
//!   `ws_stream_wasm` on the PWA. No `std::time::Instant`, no direct socket
//!   type in the public API.
//!
//! Implementation lands in M5 (see ROADMAP.md).

#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn crate_builds() {}
}
