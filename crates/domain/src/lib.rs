//! Pure domain logic for cabas.
//!
//! This crate is where all the real difficulty of the product lives: unit
//! dimensions and conversions, exact scaling, expansion of the recipe DAG,
//! aggregation of the cart, and the derivation rules that decide whether a
//! line is to buy, checked, or auto-checked.
//!
//! # Boundaries (CONSTITUTION Rules 1, 2, 4, 5)
//!
//! - **No I/O, no async, no platform.** Everything here is a pure function
//!   over owned types, testable with `cargo test` alone — no browser, no
//!   relay, no device.
//! - **No CRDT type.** Loro exists only inside `cabas-store`, which
//!   translates to and from the plain structs defined here.
//! - **No floating point in quantities.** Exact rationals end to end;
//!   `f64` appears only in the last rendering step, outside this crate.
//! - **No implicit conversion.** Crossing mass/volume/count requires the
//!   ingredient's explicit coefficient, or the quantities stay on separate
//!   lines.
//!
//! Implementation is M1 (see ROADMAP.md).

#![forbid(unsafe_code)]

pub mod cart;
pub mod expand;
pub mod ingredient;
pub mod list;
pub mod overlay;
pub mod quantity;
pub mod recipe;
pub mod units;

pub use cart::{Cart, CartError, CartLine, EntryProgress, IngredientIndex, finish_shopping};
pub use expand::{Contribution, ExpandError, RecipeIndex, expand};
pub use ingredient::{Aisle, Ingredient};
pub use list::{ListEntry, ListItem, ShoppingList};
pub use overlay::{CheckState, Explicit, Overlay};
pub use quantity::Quantity;
pub use recipe::{
    Component, IngredientUsage, Recipe, RefDisplay, Segment, Step, SubRecipeAmount, SubRecipeUsage,
};
pub use units::{Dimension, MassUnit, Unit, VolumeUnit, convert};

/// Every magnitude in the domain, exact (Rule 4).
///
/// `i128` rather than `i64` because the exact imperial factors have large
/// denominators — one ounce is 28.349523125 g — and products of those
/// overflow 64 bits during conversion.
pub type Rational = num_rational::Ratio<i128>;

/// A wall-clock instant, in milliseconds since the Unix epoch.
///
/// The domain never *reads* a clock (Rule 1): a timestamp is always supplied
/// by the caller, which is what keeps every derivation here a pure function
/// of its inputs and therefore reproducible in a test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(pub i64);

/// Stable identifiers. Opaque on purpose: they are minted once and travel
/// through the CRDT, so their representation is a `store` concern and must
/// never be parsed or ordered by meaning anywhere else.
macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// Wraps an already-minted identifier (read back from the store).
            pub fn from_raw(raw: impl Into<String>) -> Self {
                Self(raw.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

id_type!(
    /// A canonical ingredient — the aggregation key of the cart. Free-text
    /// entry resolves to one of these through the alias table; two spellings
    /// that do not resolve to the same `IngredientId` will not merge.
    IngredientId
);

id_type!(
    /// A recipe. May reference other recipes as sub-recipes, forming a DAG
    /// that M1 expands with cycle detection.
    RecipeId
);

id_type!(
    /// One line of a recipe's ingredient list. Instruction steps reference a
    /// *usage*, not an ingredient: a recipe may use flour twice, in different
    /// amounts, in different steps.
    UsageId
);

id_type!(
    /// An entry of the shopping list — a recipe or a bare ingredient.
    ListEntryId
);

id_type!(
    /// A person. Two of them, sharing 4–5 devices. Attribution keyed on this
    /// is declarative, not cryptographic (DECISIONS 0024).
    UserId
);

id_type!(
    /// A paired device. Belongs to exactly one `UserId`.
    DeviceId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_opaque_wrappers() {
        let id = IngredientId::from_raw("ing_01");
        assert_eq!(id.as_str(), "ing_01");
        assert_eq!(id, IngredientId::from_raw("ing_01"));
        assert_ne!(id, IngredientId::from_raw("ing_02"));
    }
}
