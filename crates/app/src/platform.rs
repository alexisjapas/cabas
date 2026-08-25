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
/// group, and where a device remembers things about *itself* is a host
/// concern: `localStorage` in the PWA, a config file under Tauri. The group
/// document holds the [`cabas_domain::User`] and [`cabas_domain::Device`]
/// records these ids point at — that half *is* shared, and [`crate::App`]
/// writes it on first open.
///
/// Plain strings rather than domain id types because this crosses the JS
/// boundary verbatim: the host stores what it is given and hands it back.
///
/// Attribution built on this is declarative, never access control (Rule 7).
///
/// # The user half can be missing, and the device half cannot
///
/// A device knows what it is the moment it exists — the replica's peer id is
/// derived from the device id, so there is no launch without one. *Who* is
/// carrying it is a different question, and on a phone joining an existing
/// group it is one only the group's roster can answer: the members are in the
/// document, and the document arrives over the network (DECISIONS 0068). So
/// `user` and `user_name` are `None` between the twelve words and the moment
/// somebody picks a name off that roster or adds one to it.
///
/// The two move together — both `Some` or both `None`, which `belongs_to` is
/// the only way to set. `user_name` is not redundant with the document: a
/// device whose replica is lost while `localStorage` survives has to be able
/// to put its own user back, and the name is the part no id can reconstruct.
///
/// The shape is also what an identity written before 0068 already looks like
/// from here — both fields present, both strings — so a paired device reads
/// its own stored identity across the change without a migration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct Identity {
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub user_name: Option<String>,
    pub device: String,
    pub device_name: String,
}

impl Identity {
    /// Mints the identity of a device that has never run before, before
    /// anybody has said who is carrying it.
    ///
    /// The host calls this **once**, on the very first launch, and persists
    /// the result. The user half is filled in later, by
    /// [`crate::Command::ChooseUser`] or [`crate::Command::CreateUser`], and
    /// the host persists the identity again when it is.
    pub fn mint_device(platform: &impl Platform, device_name: impl Into<String>) -> Result<Self> {
        Ok(Self {
            user: None,
            user_name: None,
            device: crate::id::mint(platform, crate::id::DEVICE)?,
            device_name: device_name.into(),
        })
    }

    /// Points this device at a person. Both halves move at once, which is
    /// what keeps "there is a user id but no name" from existing.
    pub(crate) fn belongs_to(&mut self, user: &UserId, name: &str) {
        self.user = Some(user.to_string());
        self.user_name = Some(name.to_owned());
    }

    pub(crate) fn user_id(&self) -> Option<UserId> {
        self.user.clone().map(UserId::from_raw)
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
    fn a_minted_device_knows_itself_and_not_who_carries_it() {
        let identity = Identity::mint_device(&Stub::default(), "Alice's iPhone").expect("mint");
        assert!(identity.device.starts_with(crate::id::DEVICE));
        // The half the roster answers, and the network brings the roster
        // (DECISIONS 0068).
        assert_eq!(identity.user, None);
        assert_eq!(identity.user_name, None);
        assert_eq!(identity.user_id(), None);
    }

    #[test]
    fn a_device_points_at_a_person_with_both_halves_at_once() {
        let mut identity = Identity::mint_device(&Stub::default(), "iPhone").expect("mint");
        identity.belongs_to(&UserId::from_raw("usr_1"), "Alice");
        assert_eq!(identity.user.as_deref(), Some("usr_1"));
        assert_eq!(identity.user_name.as_deref(), Some("Alice"));
        assert_eq!(identity.user_id(), Some(UserId::from_raw("usr_1")));
    }

    #[test]
    fn the_peer_id_is_stable_for_a_device_and_differs_between_devices() {
        let one = Identity {
            user: Some("usr_1".into()),
            user_name: Some("Alice".into()),
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
