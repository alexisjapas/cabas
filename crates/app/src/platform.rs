//! The impure things the domain refuses to do: read a clock, draw a random
//! number, and know which device this is.
//!
//! Rule 1 keeps `domain` a pure function of its inputs, which means every
//! timestamp it stores was handed to it by somebody. That somebody is here.
//! Both operations are behind a trait for the same reason the storage backend
//! is (Rule 8): the wasm and the native answer differ, and a test wants
//! neither.

use cabas_domain::{DeviceId, Timestamp, UserId};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

/// What the host has to provide before a command can be applied.
pub trait Platform {
    /// Wall clock, in milliseconds since the Unix epoch.
    ///
    /// Infallible on purpose: a timestamp is attribution garnish, and no
    /// command is worth refusing because a clock is unset. Implementations
    /// that cannot answer return `Timestamp(0)` and the UI shows 1970, which
    /// is visibly wrong rather than silently wrong.
    fn now(&self) -> Timestamp;

    /// A fresh 64 bits, used only to mint identifiers.
    ///
    /// Fallible, unlike the clock: an id collision is a merge that silently
    /// fuses two recipes, so a host with no entropy must be able to say so
    /// instead of being handed a counter.
    fn random_u64(&self) -> Result<u64>;
}

/// The real one: `web-time` for the clock, `getrandom` for the entropy. Works
/// on both targets, which is the whole point (Rule 8).
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemPlatform;

impl Platform for SystemPlatform {
    fn now(&self) -> Timestamp {
        // `web_time::SystemTime` is `std`'s on native and `Date.now()` in a
        // browser; `std::time::SystemTime::now()` panics on wasm32.
        let millis = web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .map(|since| since.as_millis())
            .unwrap_or_default();
        Timestamp(i64::try_from(millis).unwrap_or(i64::MAX))
    }

    fn random_u64(&self) -> Result<u64> {
        // On wasm32 this is `crypto.getRandomValues`, reachable only because
        // the `wasm_js` feature and the `getrandom_backend` cfg are both set
        // — see `.cargo/config.toml`.
        getrandom::u64().map_err(|e| AppError::Platform(format!("no randomness available: {e}")))
    }
}

/// Who this device says it is.
///
/// **Supplied by the host, never invented here.** The ids have to survive a
/// restart or every launch would look like a new person to the rest of the
/// family, and where a device remembers things about *itself* is a host
/// concern: `localStorage` in the PWA, a config file under Tauri. The family
/// document holds the [`cabas_domain::User`] and [`cabas_domain::Device`]
/// records these ids point at — that half *is* shared, and [`crate::App`]
/// writes it on first open.
///
/// Plain strings rather than domain id types because this crosses the JS
/// boundary verbatim: the host stores what it is given and hands it back.
///
/// Attribution built on this is declarative, never access control (Rule 7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct Identity {
    pub user: String,
    pub user_name: String,
    pub device: String,
    pub device_name: String,
}

impl Identity {
    /// Mints a brand-new identity. The host calls this **once**, on the very
    /// first launch, and persists the result.
    pub fn mint(
        platform: &impl Platform,
        user_name: impl Into<String>,
        device_name: impl Into<String>,
    ) -> Result<Self> {
        Ok(Self {
            user: crate::id::mint(platform, crate::id::USER)?,
            user_name: user_name.into(),
            device: crate::id::mint(platform, crate::id::DEVICE)?,
            device_name: device_name.into(),
        })
    }

    pub(crate) fn user_id(&self) -> UserId {
        UserId::from_raw(self.user.clone())
    }

    pub(crate) fn device_id(&self) -> DeviceId {
        DeviceId::from_raw(self.device.clone())
    }

    /// This replica's peer id in the CRDT.
    ///
    /// Derived from the device id rather than drawn at random, because
    /// `Document::set_peer` wants something *stable*: a replica that picks a
    /// fresh peer on every launch leaves a trail of dead peers in the history.
    /// Derived rather than reused, because the peer id is Loro's internal
    /// business and the device id is attribution — DECISIONS 0024 keeps those
    /// two apart, and this is that boundary seen from the other side.
    pub(crate) fn peer(&self) -> u64 {
        // FNV-1a. Not a hash with any security claim; a spreading function.
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in self.device.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A platform whose answers a test can predict.
    #[derive(Debug, Default)]
    struct Stub {
        counter: std::cell::Cell<u64>,
    }

    impl Platform for Stub {
        fn now(&self) -> Timestamp {
            Timestamp(1_000)
        }

        fn random_u64(&self) -> Result<u64> {
            self.counter.set(self.counter.get() + 1);
            Ok(self.counter.get())
        }
    }

    #[test]
    fn a_minted_identity_has_two_distinct_ids() {
        let identity = Identity::mint(&Stub::default(), "Alice", "Alice's iPhone").expect("mint");
        assert_ne!(identity.user, identity.device);
        assert!(identity.user.starts_with(crate::id::USER));
        assert!(identity.device.starts_with(crate::id::DEVICE));
    }

    #[test]
    fn the_peer_id_is_stable_for_a_device_and_differs_between_devices() {
        let one = Identity {
            user: "usr_1".into(),
            user_name: "Alice".into(),
            device: "dev_1".into(),
            device_name: "iPhone".into(),
        };
        let two = Identity {
            device: "dev_2".into(),
            ..one.clone()
        };
        assert_eq!(one.peer(), one.clone().peer());
        assert_ne!(one.peer(), two.peer());
    }

    #[test]
    fn the_system_clock_answers_something_after_the_epoch() {
        // Cheap, but it is the assertion that fails loudly on wasm32 if
        // `web-time` is ever swapped back for `std::time`.
        assert!(SystemPlatform.now().0 > 0);
    }
}
