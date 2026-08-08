//! The check overlay — the only part of the cart that is ever written down.
//!
//! The cart itself is a pure derivation (Rule 3). What is persisted and
//! synced is this map of **explicit user actions**, and nothing else.

use std::collections::BTreeMap;

use crate::{IngredientId, Timestamp, UserId};

/// A deliberate act by a person, as opposed to a derived default.
///
/// [`Explicit::Unchecked`] is not redundant with an absent entry, and getting
/// that wrong is the classic bug here: an absent entry falls back to the
/// derived default, which for a staple is `AutoChecked` — so a staple the
/// user just unchecked would silently re-check itself on the next derivation
/// (DECISIONS 0023).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Explicit {
    Checked { by: UserId, at: Timestamp },
    Unchecked,
}

/// Explicit actions, keyed by canonical ingredient. Flat, because there is a
/// single list (DECISIONS 0018).
pub type Overlay = BTreeMap<IngredientId, Explicit>;

/// What the cart shows for a line: the explicit action if there is one, the
/// derived default otherwise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckState {
    /// Still to pick up.
    ToBuy,
    /// Someone put it in the trolley.
    Checked { by: UserId, at: Timestamp },
    /// A staple that only recipes asked for: out of the way, but still
    /// visible and still uncheckable.
    AutoChecked,
}

impl CheckState {
    /// Whether this line needs no further action — both "bought" and "already
    /// at home" count, which is what lets a recipe whose only remaining items
    /// are staples complete.
    pub fn is_settled(&self) -> bool {
        !matches!(self, CheckState::ToBuy)
    }
}

/// The derived default for a line with no explicit action.
///
/// A staple is out of the way only while it is *recipes* asking for it.
/// Putting it on the list by hand is an explicit statement that you need to
/// buy some (DECISIONS 0023).
pub fn default_state(staple: bool, from_manual_entry: bool) -> CheckState {
    if staple && !from_manual_entry {
        CheckState::AutoChecked
    } else {
        CheckState::ToBuy
    }
}

/// Resolves the state of one ingredient.
pub fn resolve(explicit: Option<&Explicit>, staple: bool, from_manual_entry: bool) -> CheckState {
    match explicit {
        Some(Explicit::Checked { by, at }) => CheckState::Checked {
            by: by.clone(),
            at: *at,
        },
        Some(Explicit::Unchecked) => CheckState::ToBuy,
        None => default_state(staple, from_manual_entry),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user() -> UserId {
        UserId::from_raw("alice")
    }

    #[test]
    fn an_explicit_action_always_beats_the_derived_default() {
        let checked = Explicit::Checked {
            by: user(),
            at: Timestamp(1),
        };
        // Even for a staple that would otherwise auto-check.
        assert!(resolve(Some(&checked), true, false).is_settled());
        assert_eq!(
            resolve(Some(&Explicit::Unchecked), true, false),
            CheckState::ToBuy
        );
    }

    #[test]
    fn unchecking_a_staple_survives_re_derivation() {
        // The bug this guards against: without a persisted `Unchecked`, the
        // absent entry would fall back to AutoChecked and undo the user.
        let overlay: Overlay = [(IngredientId::from_raw("salt"), Explicit::Unchecked)]
            .into_iter()
            .collect();
        let state = resolve(overlay.get(&IngredientId::from_raw("salt")), true, false);
        assert_eq!(state, CheckState::ToBuy);
        assert!(!state.is_settled());
    }

    #[test]
    fn a_staple_auto_checks_only_while_recipes_ask_for_it() {
        assert_eq!(default_state(true, false), CheckState::AutoChecked);
        assert_eq!(default_state(true, true), CheckState::ToBuy);
        assert_eq!(default_state(false, false), CheckState::ToBuy);
    }

    #[test]
    fn auto_checked_counts_as_settled() {
        assert!(CheckState::AutoChecked.is_settled());
        assert!(!CheckState::ToBuy.is_settled());
    }
}
