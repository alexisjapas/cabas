//! Dimensions, units and exact conversion.
//!
//! Three measurable dimensions convert internally through a canonical base
//! unit (gram, millilitre, piece); a fourth, [`Dimension::Unmeasured`], has no
//! base at all and only ever merges with a unit identical to itself.
//!
//! Every factor here is an exact rational (Rule 4). The imperial ones are
//! exact decimals by definition — one ounce is 28.349523125 g, not an
//! approximation — which is why they can be written as ratios and why
//! `Ratio<i128>` rather than `Ratio<i64>` is required to multiply them
//! without overflow.
//!
//! Locale variants are carried from the start even though the UI exposes
//! metric only: a French tablespoon is 15 ml and a US one 14.79, and
//! retrofitting that distinction when recipe import arrives would be a data
//! migration (DECISIONS 0016).

use crate::Rational;

/// Shorthand for an exact factor.
///
/// Deliberately `Ratio::new` and not the `const fn` `Ratio::new_raw`:
/// `Ratio`'s equality compares numerator and denominator directly and assumes
/// normalised form, so an unreduced constant (28349523125/1000000000) would
/// silently fail to equal its own reduced form.
fn r(numer: i128, denom: i128) -> Rational {
    Rational::new(numer, denom)
}

/// What a quantity measures. Quantities only ever sum within one dimension;
/// crossing between them requires the ingredient's own coefficient
/// (Rule 5, see [`crate::ingredient::Ingredient`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Dimension {
    /// Countable pieces: 3 tomatoes, 2 eggs.
    Count,
    Mass,
    Volume,
    /// "To taste", "a pinch" — no meaningful magnitude to convert.
    Unmeasured,
}

impl Dimension {
    /// Dimensions a cart line prefers to be expressed in, most preferred
    /// first, when an ingredient's coefficients allow a choice.
    ///
    /// Count wins because it is the most actionable while shopping — you pick
    /// up items, so "5 tomatoes" beats "680 g of tomatoes". Mass beats volume
    /// for the same reason: scales exist in shops, measuring jugs do not.
    /// [`Dimension::Unmeasured`] is absent on purpose; it never merges.
    pub const MERGE_PREFERENCE: [Dimension; 3] =
        [Dimension::Count, Dimension::Mass, Dimension::Volume];

    /// The unit every quantity of this dimension is normalised to before
    /// summing. `None` for [`Dimension::Unmeasured`], which is exactly what
    /// makes it un-summable across differing units.
    pub fn base_unit(self) -> Option<Unit> {
        match self {
            Dimension::Count => Some(Unit::Piece),
            Dimension::Mass => Some(Unit::Mass(MassUnit::Gram)),
            Dimension::Volume => Some(Unit::Volume(VolumeUnit::Milliliter)),
            Dimension::Unmeasured => None,
        }
    }

    /// Whether quantities of this dimension can be added at all.
    pub fn is_measurable(self) -> bool {
        self.base_unit().is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MassUnit {
    Milligram,
    Gram,
    Kilogram,
    Ounce,
    Pound,
}

impl MassUnit {
    /// Grams per unit, exact.
    pub fn grams(self) -> Rational {
        match self {
            MassUnit::Milligram => r(1, 1000),
            MassUnit::Gram => r(1, 1),
            MassUnit::Kilogram => r(1000, 1),
            // Exact by international definition since 1959.
            MassUnit::Ounce => r(28_349_523_125, 1_000_000_000),
            MassUnit::Pound => r(45_359_237, 100_000),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VolumeUnit {
    Milliliter,
    Centiliter,
    Deciliter,
    Liter,
    /// French teaspoon — 5 ml by convention.
    TeaspoonFr,
    /// French tablespoon — 15 ml by convention.
    TablespoonFr,
    /// Metric cup — 250 ml.
    CupMetric,
    /// US teaspoon — 4.92892159375 ml.
    TeaspoonUs,
    /// US tablespoon — 14.78676478125 ml.
    TablespoonUs,
    /// US legal cup as used in recipes — 236.5882365 ml.
    CupUs,
    /// US fluid ounce — 29.5735295625 ml.
    FluidOunceUs,
}

impl VolumeUnit {
    /// Millilitres per unit, exact.
    pub fn milliliters(self) -> Rational {
        match self {
            VolumeUnit::Milliliter => r(1, 1),
            VolumeUnit::Centiliter => r(10, 1),
            VolumeUnit::Deciliter => r(100, 1),
            VolumeUnit::Liter => r(1000, 1),
            VolumeUnit::TeaspoonFr => r(5, 1),
            VolumeUnit::TablespoonFr => r(15, 1),
            VolumeUnit::CupMetric => r(250, 1),
            VolumeUnit::TeaspoonUs => r(492_892_159_375, 100_000_000_000),
            VolumeUnit::TablespoonUs => r(1_478_676_478_125, 100_000_000_000),
            VolumeUnit::CupUs => r(2_365_882_365, 10_000_000),
            VolumeUnit::FluidOunceUs => r(295_735_295_625, 10_000_000_000),
        }
    }
}

/// A unit a quantity can be expressed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Unit {
    Mass(MassUnit),
    Volume(VolumeUnit),
    /// Countable pieces.
    Piece,
    /// A pinch. Unmeasured: it keeps a count so an instruction can read "2
    /// pinches of salt", but it never converts to a volume — rendering a
    /// pinch as 0.6 ml would be precision the cook never asked for.
    Pinch,
    /// "To taste". The numeric value is meaningless and ignored.
    ToTaste,
}

impl Unit {
    pub fn dimension(self) -> Dimension {
        match self {
            Unit::Mass(_) => Dimension::Mass,
            Unit::Volume(_) => Dimension::Volume,
            Unit::Piece => Dimension::Count,
            Unit::Pinch | Unit::ToTaste => Dimension::Unmeasured,
        }
    }

    /// How many base units of its dimension one of this unit is worth.
    /// `None` for unmeasured units.
    pub fn base_factor(self) -> Option<Rational> {
        match self {
            Unit::Mass(m) => Some(m.grams()),
            Unit::Volume(v) => Some(v.milliliters()),
            Unit::Piece => Some(r(1, 1)),
            Unit::Pinch | Unit::ToTaste => None,
        }
    }
}

/// Converts `value` from one unit to another **within a single dimension**.
///
/// Returns `None` across dimensions and for unmeasured units — deliberately,
/// so a caller has to reach for the ingredient's own coefficients (Rule 5)
/// rather than get a silently wrong number.
pub fn convert(value: Rational, from: Unit, to: Unit) -> Option<Rational> {
    if from == to {
        return Some(value);
    }
    if from.dimension() != to.dimension() {
        return None;
    }
    let (f, t) = (from.base_factor()?, to.base_factor()?);
    Some(value * f / t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(n: i128, d: i128) -> Rational {
        Rational::new(n, d)
    }

    #[test]
    fn metric_mass_conversions_are_exact() {
        let one_kg = q(1, 1);
        assert_eq!(
            convert(
                one_kg,
                Unit::Mass(MassUnit::Kilogram),
                Unit::Mass(MassUnit::Gram)
            ),
            Some(q(1000, 1))
        );
        assert_eq!(
            convert(
                q(500, 1),
                Unit::Mass(MassUnit::Gram),
                Unit::Mass(MassUnit::Kilogram)
            ),
            Some(q(1, 2))
        );
    }

    #[test]
    fn a_third_of_a_cup_stays_a_third() {
        // The property that motivates Rule 4: no decimal drift anywhere.
        let third = q(1, 3);
        let ml = convert(
            third,
            Unit::Volume(VolumeUnit::CupMetric),
            Unit::Volume(VolumeUnit::Milliliter),
        )
        .expect("same dimension");
        assert_eq!(ml, q(250, 3));
        let back = convert(
            ml,
            Unit::Volume(VolumeUnit::Milliliter),
            Unit::Volume(VolumeUnit::CupMetric),
        )
        .expect("same dimension");
        assert_eq!(back, third);
    }

    #[test]
    fn locale_variants_differ() {
        let fr = VolumeUnit::TablespoonFr.milliliters();
        let us = VolumeUnit::TablespoonUs.milliliters();
        assert_ne!(fr, us);
        assert_eq!(fr, q(15, 1));
        // 14.78676478125 ml
        assert_eq!(us, q(1_478_676_478_125, 100_000_000_000));
    }

    #[test]
    fn imperial_factors_do_not_overflow_i128() {
        // One pound expressed in ounces must come back exactly 16.
        let ounces = convert(
            q(1, 1),
            Unit::Mass(MassUnit::Pound),
            Unit::Mass(MassUnit::Ounce),
        )
        .expect("same dimension");
        assert_eq!(ounces, q(16, 1));
    }

    #[test]
    fn cross_dimension_conversion_is_refused() {
        assert_eq!(
            convert(
                q(1, 1),
                Unit::Mass(MassUnit::Gram),
                Unit::Volume(VolumeUnit::Milliliter)
            ),
            None
        );
        assert_eq!(
            convert(q(1, 1), Unit::Piece, Unit::Mass(MassUnit::Gram)),
            None
        );
    }

    #[test]
    fn unmeasured_units_never_convert_even_to_themselves_by_factor() {
        // Identity still works — a pinch is a pinch…
        assert_eq!(convert(q(2, 1), Unit::Pinch, Unit::Pinch), Some(q(2, 1)));
        // …but there is no factor to reach any other unit, including ToTaste.
        assert_eq!(convert(q(2, 1), Unit::Pinch, Unit::ToTaste), None);
        assert_eq!(Unit::Pinch.base_factor(), None);
        assert!(!Dimension::Unmeasured.is_measurable());
    }

    #[test]
    fn every_measurable_dimension_has_a_base_unit_of_factor_one() {
        for d in Dimension::MERGE_PREFERENCE {
            let base = d.base_unit().expect("measurable");
            assert_eq!(base.dimension(), d);
            assert_eq!(base.base_factor(), Some(q(1, 1)));
        }
    }
}
