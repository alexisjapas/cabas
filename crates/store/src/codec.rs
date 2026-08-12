//! Primitive conversions between domain values and `LoroValue`.
//!
//! # Reading and writing are not symmetric, on purpose
//!
//! Writes go through *containers*, because that is what produces a mergeable
//! operation. Reads go through `get_deep_value()`, which hands back the whole
//! subtree as plain `LoroValue`s — so every reader in this crate walks one
//! kind of thing instead of branching on container-or-value at every level.
//! The document is a few hundred kilobytes (DECISIONS 0008), so materialising
//! it is not a cost worth optimising against clarity.

use std::num::NonZeroU32;

use cabas_domain::units::{MassUnit, Unit, VolumeUnit};
use cabas_domain::{Aisle, Quantity, Rational, RefDisplay, Timestamp};
use loro::{LoroMapValue, LoroValue};

use crate::error::{Result, StoreError};
use crate::schema;

// --- construction -----------------------------------------------------------

/// Builds a plain value map — the representation used for structures that are
/// rewritten as a unit rather than merged field by field (see `schema`).
pub(crate) fn value_map<I>(pairs: I) -> LoroValue
where
    I: IntoIterator<Item = (&'static str, LoroValue)>,
{
    LoroValue::Map(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

pub(crate) fn value_list<I>(items: I) -> LoroValue
where
    I: IntoIterator<Item = LoroValue>,
{
    LoroValue::List(items.into_iter().collect::<Vec<_>>().into())
}

pub(crate) fn text(s: &str) -> LoroValue {
    LoroValue::String(s.into())
}

// --- exact numbers (Rule 4) -------------------------------------------------

/// Encodes a rational as `"numerator/denominator"`.
///
/// A string, because `LoroValue` has no exact numeric type: `Double` would
/// introduce a rounding step between two devices that must agree, and a pair
/// of `i64`s overflows on the imperial factors that motivated `Ratio<i128>`
/// in the first place. The form is also readable in a raw document dump,
/// which is worth something the first time a sync bug has to be diagnosed.
pub(crate) fn rational_value(r: Rational) -> LoroValue {
    LoroValue::String(format!("{}/{}", r.numer(), r.denom()).into())
}

pub(crate) fn rational(value: &LoroValue, path: &str) -> Result<Rational> {
    let raw = string(value, path)?;
    let (numer, denom) = raw
        .split_once('/')
        .ok_or_else(|| StoreError::corrupt(path, format!("rational {raw:?} has no `/`")))?;
    let numer: i128 = numer
        .parse()
        .map_err(|_| StoreError::corrupt(path, format!("numerator {numer:?} is not an integer")))?;
    let denom: i128 = denom.parse().map_err(|_| {
        StoreError::corrupt(path, format!("denominator {denom:?} is not an integer"))
    })?;
    if denom == 0 {
        return Err(StoreError::corrupt(
            path,
            "rational with a zero denominator",
        ));
    }
    // `Ratio::new`, never `new_raw`: equality compares numerator and
    // denominator directly, so an unreduced value read back from an older
    // writer would silently fail to equal its own reduced form.
    Ok(Rational::new(numer, denom))
}

pub(crate) fn timestamp_value(t: Timestamp) -> LoroValue {
    LoroValue::I64(t.0)
}

pub(crate) fn timestamp(value: &LoroValue, path: &str) -> Result<Timestamp> {
    Ok(Timestamp(int(value, path)?))
}

pub(crate) fn servings(value: &LoroValue, path: &str) -> Result<NonZeroU32> {
    let raw = int(value, path)?;
    u32::try_from(raw)
        .ok()
        .and_then(NonZeroU32::new)
        .ok_or_else(|| StoreError::corrupt(path, format!("{raw} is not a positive serving count")))
}

// --- quantities -------------------------------------------------------------

pub(crate) fn quantity_value(q: &Quantity) -> LoroValue {
    value_map([
        (schema::quantity::VALUE, rational_value(q.value)),
        (
            schema::quantity::UNIT,
            LoroValue::String(unit_tag(q.unit).into()),
        ),
    ])
}

pub(crate) fn quantity(value: &LoroValue, path: &str) -> Result<Quantity> {
    let map = map(value, path)?;
    let value_path = format!("{path}.{}", schema::quantity::VALUE);
    let unit_path = format!("{path}.{}", schema::quantity::UNIT);
    Ok(Quantity::new(
        rational(field(map, schema::quantity::VALUE, path)?, &value_path)?,
        unit(field(map, schema::quantity::UNIT, path)?, &unit_path)?,
    ))
}

// --- enum tags --------------------------------------------------------------
//
// Tags are strings rather than integer discriminants: a discriminant silently
// changes meaning when a variant is inserted in the middle of an enum, and
// this document outlives the build that wrote it.

pub(crate) fn unit_tag(unit: Unit) -> &'static str {
    match unit {
        Unit::Mass(MassUnit::Milligram) => "mg",
        Unit::Mass(MassUnit::Gram) => "g",
        Unit::Mass(MassUnit::Kilogram) => "kg",
        Unit::Mass(MassUnit::Ounce) => "oz",
        Unit::Mass(MassUnit::Pound) => "lb",
        Unit::Volume(VolumeUnit::Milliliter) => "ml",
        Unit::Volume(VolumeUnit::Centiliter) => "cl",
        Unit::Volume(VolumeUnit::Deciliter) => "dl",
        Unit::Volume(VolumeUnit::Liter) => "l",
        Unit::Volume(VolumeUnit::TeaspoonFr) => "tsp_fr",
        Unit::Volume(VolumeUnit::TablespoonFr) => "tbsp_fr",
        Unit::Volume(VolumeUnit::CupMetric) => "cup_metric",
        Unit::Volume(VolumeUnit::TeaspoonUs) => "tsp_us",
        Unit::Volume(VolumeUnit::TablespoonUs) => "tbsp_us",
        Unit::Volume(VolumeUnit::CupUs) => "cup_us",
        Unit::Volume(VolumeUnit::FluidOunceUs) => "floz_us",
        Unit::Piece => "piece",
        Unit::Pinch => "pinch",
        Unit::ToTaste => "to_taste",
    }
}

pub(crate) fn unit(value: &LoroValue, path: &str) -> Result<Unit> {
    let tag = string(value, path)?;
    Ok(match tag.as_str() {
        "mg" => Unit::Mass(MassUnit::Milligram),
        "g" => Unit::Mass(MassUnit::Gram),
        "kg" => Unit::Mass(MassUnit::Kilogram),
        "oz" => Unit::Mass(MassUnit::Ounce),
        "lb" => Unit::Mass(MassUnit::Pound),
        "ml" => Unit::Volume(VolumeUnit::Milliliter),
        "cl" => Unit::Volume(VolumeUnit::Centiliter),
        "dl" => Unit::Volume(VolumeUnit::Deciliter),
        "l" => Unit::Volume(VolumeUnit::Liter),
        "tsp_fr" => Unit::Volume(VolumeUnit::TeaspoonFr),
        "tbsp_fr" => Unit::Volume(VolumeUnit::TablespoonFr),
        "cup_metric" => Unit::Volume(VolumeUnit::CupMetric),
        "tsp_us" => Unit::Volume(VolumeUnit::TeaspoonUs),
        "tbsp_us" => Unit::Volume(VolumeUnit::TablespoonUs),
        "cup_us" => Unit::Volume(VolumeUnit::CupUs),
        "floz_us" => Unit::Volume(VolumeUnit::FluidOunceUs),
        "piece" => Unit::Piece,
        "pinch" => Unit::Pinch,
        "to_taste" => Unit::ToTaste,
        other => return Err(StoreError::corrupt(path, format!("unknown unit {other:?}"))),
    })
}

pub(crate) fn aisle_tag(aisle: Aisle) -> &'static str {
    match aisle {
        Aisle::Produce => "produce",
        Aisle::Butcher => "butcher",
        Aisle::Fish => "fish",
        Aisle::Deli => "deli",
        Aisle::Dairy => "dairy",
        Aisle::Bakery => "bakery",
        Aisle::Grocery => "grocery",
        Aisle::Frozen => "frozen",
        Aisle::Beverages => "beverages",
        Aisle::Household => "household",
        Aisle::Items => "items",
        Aisle::Other => "other",
    }
}

pub(crate) fn aisle(value: &LoroValue, path: &str) -> Result<Aisle> {
    let tag = string(value, path)?;
    Ok(match tag.as_str() {
        "produce" => Aisle::Produce,
        "butcher" => Aisle::Butcher,
        "fish" => Aisle::Fish,
        "deli" => Aisle::Deli,
        "dairy" => Aisle::Dairy,
        "bakery" => Aisle::Bakery,
        "grocery" => Aisle::Grocery,
        "frozen" => Aisle::Frozen,
        "beverages" => Aisle::Beverages,
        "household" => Aisle::Household,
        "items" => Aisle::Items,
        // An aisle only decides sort order, so an unknown one is survivable
        // where an unknown unit is not: falling back to `Other` puts the item
        // at the end of the walk instead of refusing to open the document.
        //
        // It is also what makes adding one a non-breaking change: a phone
        // three weeks out of date reads "items" as `Other` and shows the line
        // at the end of the cart, rather than failing to open the document
        // its family just synced to it (DECISIONS 0057).
        _ => Aisle::Other,
    })
}

pub(crate) fn display_tag(display: RefDisplay) -> &'static str {
    match display {
        RefDisplay::Full => "full",
        RefDisplay::NameOnly => "name_only",
        RefDisplay::QuantityOnly => "quantity_only",
    }
}

pub(crate) fn display(value: &LoroValue, path: &str) -> Result<RefDisplay> {
    let tag = string(value, path)?;
    Ok(match tag.as_str() {
        "full" => RefDisplay::Full,
        "name_only" => RefDisplay::NameOnly,
        "quantity_only" => RefDisplay::QuantityOnly,
        other => {
            return Err(StoreError::corrupt(
                path,
                format!("unknown reference display {other:?}"),
            ));
        }
    })
}

// --- LoroValue accessors ----------------------------------------------------

pub(crate) fn map<'a>(value: &'a LoroValue, path: &str) -> Result<&'a LoroMapValue> {
    value
        .as_map()
        .ok_or_else(|| StoreError::corrupt(path, "expected a map"))
}

pub(crate) fn list<'a>(value: &'a LoroValue, path: &str) -> Result<&'a [LoroValue]> {
    value
        .as_list()
        .map(|l| l.as_ref())
        .ok_or_else(|| StoreError::corrupt(path, "expected a list"))
}

pub(crate) fn string(value: &LoroValue, path: &str) -> Result<String> {
    value
        .as_string()
        .map(|s| s.to_string())
        .ok_or_else(|| StoreError::corrupt(path, "expected a string"))
}

pub(crate) fn int(value: &LoroValue, path: &str) -> Result<i64> {
    value
        .as_i64()
        .copied()
        .ok_or_else(|| StoreError::corrupt(path, "expected an integer"))
}

pub(crate) fn boolean(value: &LoroValue, path: &str) -> Result<bool> {
    value
        .as_bool()
        .copied()
        .ok_or_else(|| StoreError::corrupt(path, "expected a boolean"))
}

/// A required field.
pub(crate) fn field<'a>(map: &'a LoroMapValue, key: &str, path: &str) -> Result<&'a LoroValue> {
    optional(map, key).ok_or_else(|| StoreError::corrupt(path, format!("missing key {key:?}")))
}

/// An optional field. `Null` counts as absent — a key that was deleted and a
/// key that was never written must read the same way, or a cleared yield
/// would come back as a corrupt quantity.
pub(crate) fn optional<'a>(map: &'a LoroMapValue, key: &str) -> Option<&'a LoroValue> {
    match map.get(key) {
        None | Some(LoroValue::Null) => None,
        Some(value) => Some(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every unit the domain defines. Listed by hand so that adding a variant
    /// without giving it a tag fails here rather than at a user's first sync.
    const ALL_UNITS: [Unit; 19] = [
        Unit::Mass(MassUnit::Milligram),
        Unit::Mass(MassUnit::Gram),
        Unit::Mass(MassUnit::Kilogram),
        Unit::Mass(MassUnit::Ounce),
        Unit::Mass(MassUnit::Pound),
        Unit::Volume(VolumeUnit::Milliliter),
        Unit::Volume(VolumeUnit::Centiliter),
        Unit::Volume(VolumeUnit::Deciliter),
        Unit::Volume(VolumeUnit::Liter),
        Unit::Volume(VolumeUnit::TeaspoonFr),
        Unit::Volume(VolumeUnit::TablespoonFr),
        Unit::Volume(VolumeUnit::CupMetric),
        Unit::Volume(VolumeUnit::TeaspoonUs),
        Unit::Volume(VolumeUnit::TablespoonUs),
        Unit::Volume(VolumeUnit::CupUs),
        Unit::Volume(VolumeUnit::FluidOunceUs),
        Unit::Piece,
        Unit::Pinch,
        Unit::ToTaste,
    ];

    const ALL_AISLES: [Aisle; 12] = [
        Aisle::Produce,
        Aisle::Butcher,
        Aisle::Fish,
        Aisle::Deli,
        Aisle::Dairy,
        Aisle::Bakery,
        Aisle::Grocery,
        Aisle::Frozen,
        Aisle::Beverages,
        Aisle::Household,
        Aisle::Items,
        Aisle::Other,
    ];

    #[test]
    fn every_unit_round_trips_through_its_tag() {
        let mut tags = Vec::new();
        for u in ALL_UNITS {
            let tag = unit_tag(u);
            let value = LoroValue::String(tag.into());
            assert_eq!(unit(&value, "test").expect("known tag"), u);
            tags.push(tag);
        }
        // Two units sharing a tag would silently rewrite one as the other.
        tags.sort_unstable();
        let before = tags.len();
        tags.dedup();
        assert_eq!(tags.len(), before, "unit tags must be unique");
    }

    #[test]
    fn every_aisle_round_trips_through_its_tag() {
        for a in ALL_AISLES {
            let value = LoroValue::String(aisle_tag(a).into());
            assert_eq!(aisle(&value, "test").expect("known tag"), a);
        }
    }

    #[test]
    fn an_unknown_aisle_degrades_but_an_unknown_unit_refuses() {
        // Sort order can be wrong; a quantity cannot.
        let unknown = LoroValue::String("chocolate_fountain".into());
        assert_eq!(aisle(&unknown, "test").expect("degrades"), Aisle::Other);
        assert!(unit(&unknown, "test").is_err());
    }

    #[test]
    fn rationals_survive_the_round_trip_exactly() {
        // The imperial factor that motivates i128 (Rule 4).
        let ounce = Rational::new(28_349_523_125, 1_000_000_000);
        let encoded = rational_value(ounce);
        assert_eq!(rational(&encoded, "test").expect("valid"), ounce);

        // A third stays a third — the property Rule 4 exists for.
        let third = Rational::new(1, 3);
        assert_eq!(
            rational(&rational_value(third), "test").expect("valid"),
            third
        );
    }

    #[test]
    fn an_unreduced_rational_is_read_back_reduced() {
        // An older writer could have emitted "2/4". `Ratio::new` normalises,
        // so it must compare equal to a freshly built 1/2.
        let raw = LoroValue::String("2/4".into());
        assert_eq!(rational(&raw, "test").expect("valid"), Rational::new(1, 2));
    }

    #[test]
    fn malformed_rationals_are_rejected_rather_than_guessed() {
        for bad in ["", "3", "3/0", "a/2", "3/b", "1.5"] {
            let value = LoroValue::String(bad.into());
            assert!(
                rational(&value, "test").is_err(),
                "{bad:?} should not parse"
            );
        }
    }

    #[test]
    fn quantities_round_trip() {
        let q = Quantity::new(Rational::new(3, 2), Unit::Mass(MassUnit::Kilogram));
        assert_eq!(quantity(&quantity_value(&q), "test").expect("valid"), q);

        let taste = Quantity::to_taste();
        assert_eq!(
            quantity(&quantity_value(&taste), "test").expect("valid"),
            taste
        );
    }

    #[test]
    fn a_null_field_reads_as_absent() {
        let map_value = value_map([("yields", LoroValue::Null)]);
        let m = map(&map_value, "test").expect("a map");
        assert!(optional(m, "yields").is_none());
        assert!(field(m, "yields", "test").is_err());
    }

    #[test]
    fn servings_must_be_a_positive_count() {
        assert_eq!(
            servings(&LoroValue::I64(4), "test").expect("valid").get(),
            4
        );
        assert!(servings(&LoroValue::I64(0), "test").is_err());
        assert!(servings(&LoroValue::I64(-2), "test").is_err());
    }
}
