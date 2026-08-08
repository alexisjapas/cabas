//! A magnitude paired with a unit, and the arithmetic allowed on it.

use crate::Rational;
use crate::units::{Dimension, MassUnit, Unit, VolumeUnit, convert};

/// An amount of something: `200 g`, `1/3 cup`, `3 pieces`, `to taste`.
///
/// The value of a [`Unit::ToTaste`] quantity is meaningless and ignored by
/// every operation here.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Quantity {
    pub value: Rational,
    pub unit: Unit,
}

impl Quantity {
    pub fn new(value: Rational, unit: Unit) -> Self {
        Self { value, unit }
    }

    /// Convenience for whole amounts: `Quantity::whole(200, Unit::Mass(Gram))`.
    pub fn whole(value: i128, unit: Unit) -> Self {
        Self::new(Rational::from_integer(value), unit)
    }

    pub fn to_taste() -> Self {
        Self::new(Rational::from_integer(1), Unit::ToTaste)
    }

    pub fn dimension(&self) -> Dimension {
        self.unit.dimension()
    }

    /// The magnitude expressed in the dimension's base unit (g, ml, piece).
    /// `None` for unmeasured quantities.
    pub fn in_base(&self) -> Option<Rational> {
        Some(self.value * self.unit.base_factor()?)
    }

    /// Same dimension only; see [`convert`].
    pub fn convert_to(&self, unit: Unit) -> Option<Quantity> {
        convert(self.value, self.unit, unit).map(|v| Quantity::new(v, unit))
    }

    /// Multiplies the magnitude. Unmeasured quantities are returned unchanged:
    /// doubling a recipe does not make "to taste" twice as much, and it does
    /// not make a pinch 2.5 pinches either — the cook rounds, not the model.
    pub fn scaled(&self, factor: Rational) -> Quantity {
        if self.dimension() == Dimension::Unmeasured {
            return self.clone();
        }
        Quantity::new(self.value * factor, self.unit)
    }

    /// Adds `other`, expressing the result in `self`'s unit.
    ///
    /// `None` unless both sides share a dimension, and — for unmeasured
    /// quantities — the very same unit: two pinches plus one pinch is three
    /// pinches, but a pinch plus "to taste" is two separate lines.
    pub fn try_add(&self, other: &Quantity) -> Option<Quantity> {
        if self.dimension() == Dimension::Unmeasured {
            return (self.unit == other.unit)
                .then(|| Quantity::new(self.value + other.value, self.unit));
        }
        let converted = other.convert_to(self.unit)?;
        Some(Quantity::new(self.value + converted.value, self.unit))
    }

    /// Rounds up to a whole number of units — what a cart does to a countable
    /// line, because scaling a recipe by 3/2 asks for 1.5 eggs and the shop
    /// sells 2 (DECISIONS 0016). The recipe itself keeps the exact value.
    pub fn ceil_to_whole(&self) -> Quantity {
        Quantity::new(self.value.ceil(), self.unit)
    }

    /// Re-expresses the quantity in the unit a human would read it in, without
    /// changing what it denotes: 1500 g becomes 1.5 kg, 0.5 g becomes 500 mg,
    /// 2000 ml becomes 2 L.
    ///
    /// Only ever moves within the metric ladder — a quantity entered in cups
    /// or ounces stays there, because rewriting a cook's own unit is not this
    /// function's business.
    pub fn humanized(&self) -> Quantity {
        let base = match self.in_base() {
            Some(b) => b,
            None => return self.clone(),
        };
        if base.is_zero_magnitude() {
            return self.clone();
        }
        let target = match self.unit {
            Unit::Mass(MassUnit::Milligram | MassUnit::Gram | MassUnit::Kilogram) => {
                let grams = base;
                if grams >= Rational::from_integer(1000) {
                    Unit::Mass(MassUnit::Kilogram)
                } else if grams < Rational::from_integer(1) {
                    Unit::Mass(MassUnit::Milligram)
                } else {
                    Unit::Mass(MassUnit::Gram)
                }
            }
            Unit::Volume(
                VolumeUnit::Milliliter
                | VolumeUnit::Centiliter
                | VolumeUnit::Deciliter
                | VolumeUnit::Liter,
            ) => {
                if base >= Rational::from_integer(1000) {
                    Unit::Volume(VolumeUnit::Liter)
                } else {
                    Unit::Volume(VolumeUnit::Milliliter)
                }
            }
            other => other,
        };
        self.convert_to(target).unwrap_or_else(|| self.clone())
    }
}

/// Small helper kept private to the module: `Ratio` has no `is_zero` without
/// pulling `num_traits` into the public surface.
trait ZeroMagnitude {
    fn is_zero_magnitude(&self) -> bool;
}

impl ZeroMagnitude for Rational {
    fn is_zero_magnitude(&self) -> bool {
        *self.numer() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(n: i128, d: i128, unit: Unit) -> Quantity {
        Quantity::new(Rational::new(n, d), unit)
    }

    const G: Unit = Unit::Mass(MassUnit::Gram);
    const KG: Unit = Unit::Mass(MassUnit::Kilogram);
    const ML: Unit = Unit::Volume(VolumeUnit::Milliliter);
    const L: Unit = Unit::Volume(VolumeUnit::Liter);

    #[test]
    fn scaling_is_exact_and_reversible() {
        let flour = q(200, 1, G);
        let scaled = flour.scaled(Rational::new(3, 2));
        assert_eq!(scaled, q(300, 1, G));
        assert_eq!(scaled.scaled(Rational::new(2, 3)), flour);
    }

    #[test]
    fn scaling_by_thirds_does_not_drift() {
        let q1 = q(1, 1, G);
        let there_and_back = q1.scaled(Rational::new(1, 3)).scaled(Rational::new(3, 1));
        assert_eq!(there_and_back, q1);
    }

    #[test]
    fn addition_crosses_units_but_not_dimensions() {
        let a = q(500, 1, G);
        let b = q(1, 2, KG);
        assert_eq!(a.try_add(&b), Some(q(1000, 1, G)));
        // Result is expressed in the left-hand unit.
        assert_eq!(b.try_add(&a), Some(q(1, 1, KG)));
        assert_eq!(a.try_add(&q(100, 1, ML)), None);
    }

    #[test]
    fn unmeasured_adds_only_to_the_identical_unit() {
        let pinch = q(2, 1, Unit::Pinch);
        assert_eq!(
            pinch.try_add(&q(1, 1, Unit::Pinch)),
            Some(q(3, 1, Unit::Pinch))
        );
        assert_eq!(pinch.try_add(&Quantity::to_taste()), None);
    }

    #[test]
    fn scaling_leaves_unmeasured_alone() {
        let salt = Quantity::to_taste();
        assert_eq!(salt.scaled(Rational::from_integer(4)), salt);
        let pinch = q(2, 1, Unit::Pinch);
        assert_eq!(pinch.scaled(Rational::new(3, 2)), pinch);
    }

    #[test]
    fn count_rounds_up_for_the_cart() {
        // 1 egg scaled by 3/2 → the shop sells 2.
        let eggs = q(1, 1, Unit::Piece).scaled(Rational::new(3, 2));
        assert_eq!(eggs.value, Rational::new(3, 2));
        assert_eq!(eggs.ceil_to_whole(), q(2, 1, Unit::Piece));
        // Already whole stays put.
        assert_eq!(q(4, 1, Unit::Piece).ceil_to_whole(), q(4, 1, Unit::Piece));
    }

    #[test]
    fn humanized_climbs_the_metric_ladder() {
        assert_eq!(q(1500, 1, G).humanized(), q(3, 2, KG));
        assert_eq!(q(2000, 1, ML).humanized(), q(2, 1, L));
        assert_eq!(
            q(1, 2, G).humanized(),
            q(500, 1, Unit::Mass(MassUnit::Milligram))
        );
        assert_eq!(q(250, 1, G).humanized(), q(250, 1, G));
    }

    #[test]
    fn humanized_leaves_non_metric_and_unmeasured_units_alone() {
        let cups = q(2, 1, Unit::Volume(VolumeUnit::CupUs));
        assert_eq!(cups.humanized(), cups);
        assert_eq!(Quantity::to_taste().humanized(), Quantity::to_taste());
    }
}
