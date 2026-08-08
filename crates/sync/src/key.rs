//! The family key, and the 12-word phrase that is its only true form.
//!
//! The BIP39 mnemonic is the **single canonical secret** (DECISIONS 0042):
//! the symmetric key is the first 32 bytes of its standard seed, the family
//! id — the relay's routing and storage key — is the next 16. PBKDF2's
//! output blocks are computed independently, so publishing the id reveals
//! nothing of the key, and every device derives both from the same phrase
//! with no second channel. Pairing by QR and pairing by typing the phrase
//! are therefore the same operation with different input methods (0021).
//!
//! The wordlist is BIP39 English even though the UI is French: it is the
//! list every backup tool understands, its words are unique in their first
//! four letters, and the phrase carries its own checksum.

use std::fmt;

use bip39::{Language, Mnemonic};
use serde::{Deserialize, Serialize};

use crate::error::{Result, SyncError};

/// The number of words 0021 promises. `bip39` would happily parse 15 up to
/// 24, but a phrase of any other length here means a word was lost in
/// transcription, and "expected 12 words, got 11" is the error that says so.
pub const PHRASE_WORDS: usize = 12;

/// The relay's routing and storage key. Public in the sense that the relay
/// and its logs hold it in the clear; unguessable, which is the relay's
/// entire access story — whoever holds it may read ciphertext (the security
/// model working as intended) and append frames a client will refuse to
/// merge unless they open (DECISIONS 0042).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FamilyId([u8; 16]);

impl FamilyId {
    /// The hex spelling — directory names on the relay, and nothing else.
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Parses the hex spelling back. The relay uses this to rediscover its
    /// families from directory names after a restart.
    pub fn from_hex(hex: &str) -> Result<Self> {
        let bytes = hex.as_bytes();
        if bytes.len() != 32 {
            return Err(SyncError::Wire(format!(
                "family id: expected 32 hex digits, got {}",
                bytes.len()
            )));
        }
        let mut out = [0u8; 16];
        for (i, chunk) in bytes.chunks_exact(2).enumerate() {
            let digits = std::str::from_utf8(chunk)
                .ok()
                .and_then(|s| u8::from_str_radix(s, 16).ok());
            out[i] = digits.ok_or_else(|| SyncError::Wire("family id: not hex".to_string()))?;
        }
        Ok(FamilyId(out))
    }
}

impl fmt::Display for FamilyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl fmt::Debug for FamilyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FamilyId({})", self.to_hex())
    }
}

/// One shared symmetric key per family (DECISIONS 0009). Every paired device
/// holds it; the relay never does. Rotating it — the only way to revoke a
/// device — means generating a new phrase and re-pairing everyone (0024).
pub struct FamilyKey {
    key: [u8; 32],
    id: FamilyId,
    phrase: String,
}

impl FamilyKey {
    /// A fresh family: 128 bits from the OS, spelled as 12 words. No key
    /// stretching — the entropy is machine-generated, and no amount of
    /// stretching improves on 128 random bits (DECISIONS 0042).
    pub fn generate() -> Result<Self> {
        let mut entropy = [0u8; 16];
        getrandom::fill(&mut entropy).map_err(|e| SyncError::Entropy(e.to_string()))?;
        let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
            .map_err(|e| SyncError::Phrase(e.to_string()))?;
        Ok(Self::derive(mnemonic))
    }

    /// The same family on another device: the phrase as typed or scanned.
    /// Case and runs of whitespace are forgiven — a phrase read aloud across
    /// a kitchen or pasted from a note must not fail on formatting.
    pub fn from_phrase(phrase: &str) -> Result<Self> {
        let normalized = phrase
            .split_whitespace()
            .map(str::to_lowercase)
            .collect::<Vec<_>>()
            .join(" ");
        let words = normalized.split(' ').filter(|w| !w.is_empty()).count();
        if words != PHRASE_WORDS {
            return Err(SyncError::Phrase(format!(
                "expected {PHRASE_WORDS} words, got {words}"
            )));
        }
        let mnemonic = Mnemonic::parse_in_normalized(Language::English, &normalized)
            .map_err(|e| SyncError::Phrase(e.to_string()))?;
        Ok(Self::derive(mnemonic))
    }

    /// Key and id from the standard BIP39 seed, empty passphrase. The
    /// passphrase stays empty forever: it is BIP39's second factor for
    /// wallets, and here it would be a thirteenth word to lose.
    fn derive(mnemonic: Mnemonic) -> Self {
        let seed = mnemonic.to_seed_normalized("");
        let mut key = [0u8; 32];
        key.copy_from_slice(&seed[..32]);
        let mut id = [0u8; 16];
        id.copy_from_slice(&seed[32..48]);
        FamilyKey {
            key,
            id: FamilyId(id),
            phrase: mnemonic.to_string(),
        }
    }

    /// The phrase in canonical form — lowercase, single spaces. This is what
    /// the QR code carries and what the pairing screen displays (0021).
    pub fn phrase(&self) -> &str {
        &self.phrase
    }

    pub fn id(&self) -> FamilyId {
        self.id
    }

    /// The raw key, visible to this crate only: the cipher is the single
    /// consumer, and no other crate seals or opens anything (Rule 7).
    pub(crate) fn bytes(&self) -> &[u8; 32] {
        &self.key
    }
}

/// The phrase is the key; a `Debug` line in a log must not be a backup of it.
impl fmt::Debug for FamilyKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FamilyKey({}, key redacted)", self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The BIP39 test phrase — a valid 12-word mnemonic with a checksum,
    /// safe to hardcode because no real family will ever roll all-zero
    /// entropy.
    const PHRASE: &str = "abandon abandon abandon abandon abandon abandon \
                          abandon abandon abandon abandon abandon about";

    #[test]
    fn a_generated_key_round_trips_through_its_phrase() {
        let a = FamilyKey::generate().unwrap();
        let b = FamilyKey::from_phrase(a.phrase()).unwrap();
        assert_eq!(a.key, b.key);
        assert_eq!(a.id(), b.id());
        assert_eq!(a.phrase(), b.phrase());
    }

    #[test]
    fn two_generated_keys_differ() {
        let a = FamilyKey::generate().unwrap();
        let b = FamilyKey::generate().unwrap();
        assert_ne!(a.key, b.key, "two families must never share a key");
        assert_ne!(a.id(), b.id());
    }

    #[test]
    fn derivation_is_deterministic() {
        let a = FamilyKey::from_phrase(PHRASE).unwrap();
        let b = FamilyKey::from_phrase(PHRASE).unwrap();
        assert_eq!(a.key, b.key);
        assert_eq!(a.id(), b.id());
    }

    #[test]
    fn formatting_is_forgiven() {
        let sloppy = format!("  {}  ", PHRASE.to_uppercase().replace(' ', "   "));
        let a = FamilyKey::from_phrase(&sloppy).unwrap();
        let b = FamilyKey::from_phrase(PHRASE).unwrap();
        assert_eq!(a.key, b.key);
        assert_eq!(
            a.phrase(),
            PHRASE.split_whitespace().collect::<Vec<_>>().join(" ")
        );
    }

    #[test]
    fn a_lost_word_names_the_count() {
        let eleven = PHRASE
            .split_whitespace()
            .take(11)
            .collect::<Vec<_>>()
            .join(" ");
        let err = FamilyKey::from_phrase(&eleven).unwrap_err();
        assert!(matches!(err, SyncError::Phrase(ref m) if m.contains("11")));
    }

    #[test]
    fn a_swapped_word_fails_the_checksum() {
        let wrong = PHRASE.replace("about", "zoo");
        assert!(matches!(
            FamilyKey::from_phrase(&wrong),
            Err(SyncError::Phrase(_))
        ));
    }

    #[test]
    fn the_id_is_not_a_prefix_of_the_key() {
        // The id is published; the key is not. They come from disjoint
        // regions of the seed, and this pins that no refactor ever makes
        // one a window into the other.
        let k = FamilyKey::from_phrase(PHRASE).unwrap();
        assert!(!k.key.starts_with(&k.id.0));
        assert!(!k.key.ends_with(&k.id.0));
    }

    #[test]
    fn family_id_round_trips_through_hex() {
        let k = FamilyKey::from_phrase(PHRASE).unwrap();
        let hex = k.id().to_hex();
        assert_eq!(hex.len(), 32);
        assert_eq!(FamilyId::from_hex(&hex).unwrap(), k.id());
        assert!(FamilyId::from_hex("not hex at all").is_err());
        assert!(FamilyId::from_hex(&hex[..30]).is_err());
    }

    #[test]
    fn debug_never_prints_the_key() {
        let k = FamilyKey::from_phrase(PHRASE).unwrap();
        let debug = format!("{k:?}");
        assert!(debug.contains("redacted"));
        assert!(!debug.contains(PHRASE.split(' ').next().unwrap()));
    }
}
