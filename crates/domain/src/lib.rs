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
//! Implementation lands in M1 (see ROADMAP.md); this file currently carries
//! only the identifiers, which every later milestone already depends on.

#![forbid(unsafe_code)]

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
