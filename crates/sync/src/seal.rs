//! XChaCha20-Poly1305, and nothing else, anywhere (Rule 7, DECISIONS 0009).
//!
//! A sealed frame is `nonce ‖ ciphertext`, the ciphertext ending in the
//! 16-byte Poly1305 tag. The nonce is 24 random bytes per frame — XChaCha's
//! extended nonce is the reason it was chosen over plain ChaCha: at this
//! size, random nonces need no coordination between devices that are by
//! design never online at the same time. A counter would have to be
//! per-device and persisted; randomness only has to be random.
//!
//! No associated data. The only party who could usefully tamper with the
//! plaintext metadata around a frame is the relay, and a malicious relay
//! can already drop or withhold frames wholesale — availability was never
//! the property the seal buys (DECISIONS 0042). What it buys is that the
//! relay reads nothing and forges nothing.

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};

use crate::error::{Result, SyncError};
use crate::key::FamilyKey;

/// XChaCha20's extended nonce, the first bytes of every sealed frame.
pub const NONCE_LEN: usize = 24;

/// Poly1305's tag, the last bytes — a sealed frame is never shorter than
/// the two combined.
const TAG_LEN: usize = 16;

/// Seals a plaintext under the family key. Every call draws a fresh nonce,
/// so sealing the same bytes twice yields unrelated frames.
pub fn seal(key: &FamilyKey, plaintext: &[u8]) -> Result<Vec<u8>> {
    let mut nonce = [0u8; NONCE_LEN];
    getrandom::fill(&mut nonce).map_err(|e| SyncError::Entropy(e.to_string()))?;
    let ciphertext = XChaCha20Poly1305::new(key.bytes().into())
        .encrypt(XNonce::from_slice(&nonce), plaintext)
        .map_err(|_| SyncError::Seal)?;
    let mut sealed = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    sealed.extend_from_slice(&nonce);
    sealed.extend_from_slice(&ciphertext);
    Ok(sealed)
}

/// Opens a sealed frame. One bit of failure, on purpose: a frame that does
/// not open is dropped and counted by the session, never merged — that is
/// the entire client-side handling of a stranger who found the family id
/// (DECISIONS 0042).
pub fn open(key: &FamilyKey, sealed: &[u8]) -> Result<Vec<u8>> {
    if sealed.len() < NONCE_LEN + TAG_LEN {
        return Err(SyncError::Open);
    }
    let (nonce, ciphertext) = sealed.split_at(NONCE_LEN);
    XChaCha20Poly1305::new(key.bytes().into())
        .decrypt(XNonce::from_slice(nonce), ciphertext)
        .map_err(|_| SyncError::Open)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> FamilyKey {
        FamilyKey::generate().unwrap()
    }

    #[test]
    fn sealed_frames_round_trip() {
        let k = key();
        let plaintext = b"5 tomatoes, checked by margaux";
        let sealed = seal(&k, plaintext).unwrap();
        assert_eq!(open(&k, &sealed).unwrap(), plaintext);
    }

    #[test]
    fn the_empty_plaintext_round_trips() {
        let k = key();
        let sealed = seal(&k, b"").unwrap();
        assert_eq!(open(&k, &sealed).unwrap(), b"");
    }

    #[test]
    fn sealing_twice_yields_unrelated_frames() {
        let k = key();
        let a = seal(&k, b"same bytes").unwrap();
        let b = seal(&k, b"same bytes").unwrap();
        assert_ne!(a, b, "a repeated nonce would be a key-recovery bug");
    }

    #[test]
    fn every_flipped_bit_is_caught() {
        let k = key();
        let sealed = seal(&k, b"do not trust the relay").unwrap();
        for i in 0..sealed.len() {
            let mut tampered = sealed.clone();
            tampered[i] ^= 1;
            assert_eq!(
                open(&k, &tampered),
                Err(SyncError::Open),
                "flipping byte {i} went unnoticed"
            );
        }
    }

    #[test]
    fn truncation_is_caught() {
        let k = key();
        let sealed = seal(&k, b"short me").unwrap();
        for len in 0..sealed.len() {
            assert_eq!(open(&k, &sealed[..len]), Err(SyncError::Open));
        }
    }

    #[test]
    fn the_wrong_key_opens_nothing() {
        let sealed = seal(&key(), b"for one family only").unwrap();
        assert_eq!(open(&key(), &sealed), Err(SyncError::Open));
    }
}
