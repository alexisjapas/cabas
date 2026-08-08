//! The people sharing the list, and the devices they carry.
//!
//! Users and devices are modelled separately (DECISIONS 0024): a person owns
//! several devices, and revocation is a per-device act while attribution is a
//! per-person one.
//!
//! Everything keyed on these names is **declarative, never access control**
//! (Rule 7). One shared family key decrypts the whole document, so any holder
//! can write as anyone; `added_by` says who most likely did something, not
//! who was allowed to.

use crate::{DeviceId, Timestamp, UserId};

/// A person, as far as attribution is concerned.
///
/// There are two of them in practice, and nothing here enforces that — a cap
/// would forbid a case that costs nothing to allow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: UserId,
    /// Shown wherever attribution surfaces: "added by Alexis", "checked by
    /// Marie".
    pub name: String,
}

impl User {
    pub fn new(id: UserId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
        }
    }
}

/// A paired device, belonging to exactly one user.
///
/// The device screen is the one place the UI must state the limit of all this
/// (Rule 7): revoking a lost device means rotating the family key and
/// re-pairing everyone, because there is no per-device credential to revoke
/// on its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub id: DeviceId,
    pub owner: UserId,
    /// What the device list calls it — "Alexis' iPhone".
    pub name: String,
    /// When this device joined the family document. Supplied by the caller,
    /// like every timestamp here: the domain reads no clock (Rule 1).
    pub paired_at: Timestamp,
}

impl Device {
    pub fn new(id: DeviceId, owner: UserId, name: impl Into<String>, paired_at: Timestamp) -> Self {
        Self {
            id,
            owner,
            name: name.into(),
            paired_at,
        }
    }
}

/// The devices belonging to `owner`, in the order given.
///
/// Reported, never enforced: a device whose owner is not in the roster is
/// simply absent from every user's list rather than an error. Under a CRDT
/// one replica can delete a user while another pairs a device to it, and a
/// dangling owner must not be able to break the device screen (same reasoning
/// as [`crate::Recipe::dangling_refs`], DECISIONS 0022).
pub fn devices_of<'a, I>(devices: I, owner: &UserId) -> impl Iterator<Item = &'a Device>
where
    I: IntoIterator<Item = &'a Device>,
{
    let owner = owner.clone();
    devices.into_iter().filter(move |d| d.owner == owner)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alice() -> User {
        User::new(UserId::from_raw("alice"), "Alice")
    }

    fn phone() -> Device {
        Device::new(
            DeviceId::from_raw("d_phone"),
            UserId::from_raw("alice"),
            "Alice's iPhone",
            Timestamp(1_000),
        )
    }

    fn laptop() -> Device {
        Device::new(
            DeviceId::from_raw("d_laptop"),
            UserId::from_raw("bob"),
            "Bob's laptop",
            Timestamp(2_000),
        )
    }

    #[test]
    fn a_user_owns_several_devices() {
        let second = Device::new(
            DeviceId::from_raw("d_tablet"),
            alice().id,
            "Alice's iPad",
            Timestamp(3_000),
        );
        let devices = vec![phone(), laptop(), second];

        let mine: Vec<_> = devices_of(&devices, &alice().id).collect();
        assert_eq!(mine.len(), 2);
        assert!(mine.iter().all(|d| d.owner == alice().id));
    }

    #[test]
    fn a_device_whose_owner_is_gone_is_merely_invisible() {
        // The concurrent case: one replica deleted the user, another had
        // already paired a device to it. Nothing panics, nothing is lost.
        let devices = vec![laptop()];
        assert_eq!(devices_of(&devices, &alice().id).count(), 0);
        assert_eq!(devices[0].owner, UserId::from_raw("bob"));
    }
}
