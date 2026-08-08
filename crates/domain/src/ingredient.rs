//! Canonical ingredients — the aggregation key of the cart.
//!
//! Two spellings that do not resolve to the same [`IngredientId`] will not
//! merge, which is why free-text entry has to go through the alias table
//! before it reaches anything here.

use crate::units::{Dimension, MassUnit, Unit, VolumeUnit};
use crate::{IngredientId, Quantity, Rational};

/// Where an item is found in the shop. Declaration order **is** the walking
/// order used to sort the cart — sorting by route is the single largest
/// usability win available to a shopping list, and it costs one `derive(Ord)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Aisle {
    Produce,
    Butcher,
    Fish,
    Deli,
    Dairy,
    Bakery,
    Grocery,
    Frozen,
    Beverages,
    Household,
    Other,
}

/// An elementary component. Quantities of the same ingredient sum; quantities
/// of different ingredients never do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ingredient {
    pub id: IngredientId,
    pub name: String,
    /// Alternative spellings that resolve to this ingredient.
    pub aliases: Vec<String>,
    pub aisle: Aisle,
    /// Salt, pepper, oil, flour: excluded from the cart by default when it is
    /// only recipes that asked for them (DECISIONS 0023). Not stock tracking —
    /// there is no quantity to keep up to date.
    pub staple: bool,
    /// Grams per millilitre. Enables mass ↔ volume.
    pub density: Option<Rational>,
    /// Grams per piece. Enables count ↔ mass.
    pub unit_weight: Option<Rational>,
}

impl Ingredient {
    /// A plain ingredient with no conversion coefficients.
    pub fn new(id: IngredientId, name: impl Into<String>, aisle: Aisle) -> Self {
        Self {
            id,
            name: name.into(),
            aliases: Vec::new(),
            aisle,
            staple: false,
            density: None,
            unit_weight: None,
        }
    }

    pub fn with_density(mut self, grams_per_ml: Rational) -> Self {
        self.density = Some(grams_per_ml);
        self
    }

    pub fn with_unit_weight(mut self, grams_per_piece: Rational) -> Self {
        self.unit_weight = Some(grams_per_piece);
        self
    }

    pub fn as_staple(mut self) -> Self {
        self.staple = true;
        self
    }

    /// Does `name` denote this ingredient? Case-insensitive over the canonical
    /// name and every alias.
    pub fn matches(&self, name: &str) -> bool {
        let name = name.trim();
        self.name.eq_ignore_ascii_case(name)
            || self.aliases.iter().any(|a| a.eq_ignore_ascii_case(name))
    }

    /// Converts a quantity of *this* ingredient into `target`, crossing
    /// dimensions when — and only when — the needed coefficient is present
    /// (Rule 5). Grams are the pivot.
    ///
    /// Returns `None` rather than an approximation: a plausible wrong
    /// conversion produces a shopping quantity nobody can trace back, whereas
    /// two honest lines are merely slightly less tidy.
    pub fn convert(&self, quantity: &Quantity, target: Dimension) -> Option<Quantity> {
        if quantity.dimension() == target {
            return quantity.convert_to(target.base_unit()?);
        }
        self.grams_to(self.to_grams(quantity)?, target)
    }

    /// Whether [`Ingredient::convert`] would succeed, without doing the work.
    pub fn can_convert(&self, from: Dimension, to: Dimension) -> bool {
        if from == to {
            return from.is_measurable();
        }
        let pivot = match from {
            Dimension::Mass => true,
            Dimension::Volume => positive(self.density).is_some(),
            Dimension::Count => positive(self.unit_weight).is_some(),
            Dimension::Unmeasured => false,
        };
        let out = match to {
            Dimension::Mass => true,
            Dimension::Volume => positive(self.density).is_some(),
            Dimension::Count => positive(self.unit_weight).is_some(),
            Dimension::Unmeasured => false,
        };
        pivot && out
    }

    fn to_grams(&self, quantity: &Quantity) -> Option<Rational> {
        let base = quantity.in_base()?;
        match quantity.dimension() {
            Dimension::Mass => Some(base),
            Dimension::Volume => Some(base * positive(self.density)?),
            Dimension::Count => Some(base * positive(self.unit_weight)?),
            Dimension::Unmeasured => None,
        }
    }

    fn grams_to(&self, grams: Rational, target: Dimension) -> Option<Quantity> {
        match target {
            Dimension::Mass => Some(Quantity::new(grams, Unit::Mass(MassUnit::Gram))),
            Dimension::Volume => Some(Quantity::new(
                grams / positive(self.density)?,
                Unit::Volume(VolumeUnit::Milliliter),
            )),
            Dimension::Count => Some(Quantity::new(
                grams / positive(self.unit_weight)?,
                Unit::Piece,
            )),
            Dimension::Unmeasured => None,
        }
    }
}

/// A coefficient is usable only if it is strictly positive — a zero density
/// would otherwise divide by zero, and a negative one is meaningless.
fn positive(coefficient: Option<Rational>) -> Option<Rational> {
    coefficient.filter(|c| *c.numer() > 0 && *c.denom() > 0)
}

/// Resolves free text to a canonical ingredient, by name or by alias.
///
/// Deliberately stops at "not found" rather than creating one: minting an
/// ingredient is a write, and this crate performs none (Rule 1). Deciding
/// whether an unknown word becomes a new ingredient or a prompt is the app
/// layer's call.
pub fn resolve<'a, I>(candidates: I, text: &str) -> Option<&'a Ingredient>
where
    I: IntoIterator<Item = &'a Ingredient>,
{
    candidates.into_iter().find(|i| i.matches(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rat(n: i128, d: i128) -> Rational {
        Rational::new(n, d)
    }

    fn flour() -> Ingredient {
        // ~0.55 g/ml
        Ingredient::new(IngredientId::from_raw("flour"), "Flour", Aisle::Grocery)
            .with_density(rat(55, 100))
            .as_staple()
    }

    fn tomato() -> Ingredient {
        // ~150 g each
        Ingredient::new(IngredientId::from_raw("tomato"), "Tomato", Aisle::Produce)
            .with_unit_weight(rat(150, 1))
    }

    const G: Unit = Unit::Mass(MassUnit::Gram);
    const ML: Unit = Unit::Volume(VolumeUnit::Milliliter);

    #[test]
    fn mass_and_volume_convert_through_density() {
        let f = flour();
        let volume = Quantity::whole(100, ML);
        let mass = f.convert(&volume, Dimension::Mass).expect("density known");
        assert_eq!(mass, Quantity::new(rat(55, 1), G));
        // Round-trips exactly.
        assert_eq!(
            f.convert(&mass, Dimension::Volume),
            Some(Quantity::whole(100, ML))
        );
    }

    #[test]
    fn count_and_mass_convert_through_unit_weight() {
        let t = tomato();
        let three = Quantity::whole(3, Unit::Piece);
        assert_eq!(
            t.convert(&three, Dimension::Mass),
            Some(Quantity::new(rat(450, 1), G))
        );
        assert_eq!(
            t.convert(&Quantity::whole(300, G), Dimension::Count),
            Some(Quantity::whole(2, Unit::Piece))
        );
    }

    #[test]
    fn conversion_without_the_coefficient_is_refused() {
        let bare = Ingredient::new(IngredientId::from_raw("x"), "X", Aisle::Other);
        assert_eq!(
            bare.convert(&Quantity::whole(100, ML), Dimension::Mass),
            None
        );
        assert_eq!(
            bare.convert(&Quantity::whole(2, Unit::Piece), Dimension::Mass),
            None
        );
        assert!(!bare.can_convert(Dimension::Volume, Dimension::Mass));
        // Same dimension still works with no coefficient at all.
        assert!(bare.can_convert(Dimension::Mass, Dimension::Mass));
    }

    #[test]
    fn count_to_volume_needs_both_coefficients() {
        let t = tomato(); // unit weight only
        assert!(!t.can_convert(Dimension::Count, Dimension::Volume));
        assert_eq!(
            t.convert(&Quantity::whole(1, Unit::Piece), Dimension::Volume),
            None
        );

        let both = t.clone().with_density(rat(1, 1));
        assert!(both.can_convert(Dimension::Count, Dimension::Volume));
        assert_eq!(
            both.convert(&Quantity::whole(1, Unit::Piece), Dimension::Volume),
            Some(Quantity::whole(150, ML))
        );
    }

    #[test]
    fn unmeasured_never_converts() {
        let f = flour();
        assert_eq!(f.convert(&Quantity::to_taste(), Dimension::Mass), None);
        assert!(!f.can_convert(Dimension::Unmeasured, Dimension::Mass));
        assert!(!f.can_convert(Dimension::Mass, Dimension::Unmeasured));
    }

    #[test]
    fn a_zero_coefficient_is_not_usable() {
        let broken =
            Ingredient::new(IngredientId::from_raw("z"), "Z", Aisle::Other).with_density(rat(0, 1));
        assert!(!broken.can_convert(Dimension::Volume, Dimension::Mass));
        // Must not panic on division.
        assert_eq!(
            broken.convert(&Quantity::whole(10, G), Dimension::Volume),
            None
        );
    }

    #[test]
    fn aisles_sort_in_walking_order() {
        let mut aisles = [Aisle::Frozen, Aisle::Produce, Aisle::Grocery, Aisle::Dairy];
        aisles.sort();
        assert_eq!(
            aisles,
            [Aisle::Produce, Aisle::Dairy, Aisle::Grocery, Aisle::Frozen]
        );
    }

    #[test]
    fn aliases_resolve_case_insensitively() {
        let mut t = tomato();
        t.aliases.push("Tomates".into());
        assert!(t.matches("tomato"));
        assert!(t.matches("  TOMATES "));
        assert!(!t.matches("cherry tomato"));
    }

    #[test]
    fn free_text_resolves_to_the_canonical_ingredient() {
        let mut t = tomato();
        t.aliases.push("Tomates".into());
        let library = vec![flour(), t];

        let found = resolve(&library, "tomates").expect("alias resolves");
        assert_eq!(found.id, IngredientId::from_raw("tomato"));
        // Unknown text is reported, never minted here (Rule 1).
        assert!(resolve(&library, "saffron").is_none());
    }
}
