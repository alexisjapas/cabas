//! The enum spellings the frontend sees.
//!
//! # Why these are not the store's tags
//!
//! `store` already encodes units and aisles as strings, and the spellings
//! here are deliberately the same ones — but they are a **different
//! contract**. The store's tags are a persistence format: a phone left in a
//! pocket for three weeks decodes a document written by a build that is three
//! weeks old, so changing one is a breaking change (Rule 15). These are an
//! API between two halves of one artifact that always ship together, so
//! changing one costs a rebuild of the frontend and nothing more.
//!
//! Sharing one table would tie the two lifetimes together, and the first time
//! the UI wanted a new spelling it would become a schema migration. The cost
//! of keeping them apart is this file, plus a test on each side that no
//! variant goes untagged.
//!
//! Every conversion below is **total**. An unknown tag cannot arrive here:
//! it fails at deserialisation, before any command runs.

use cabas_domain::units::{MassUnit, Unit, VolumeUnit};
use cabas_domain::{Aisle, CheckState, RefDisplay};
use serde::{Deserialize, Serialize};

/// A unit, as the frontend names it. The UI owns the *label* ("c. à s.") —
/// this is the identity underneath it, and it never changes with the
/// language (see the crate docs on where French lives).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub enum UnitTag {
    Mg,
    G,
    Kg,
    Oz,
    Lb,
    Ml,
    Cl,
    Dl,
    L,
    TspFr,
    TbspFr,
    CupMetric,
    TspUs,
    TbspUs,
    CupUs,
    FlozUs,
    Piece,
    Pinch,
    ToTaste,
}

impl From<Unit> for UnitTag {
    fn from(unit: Unit) -> Self {
        match unit {
            Unit::Mass(MassUnit::Milligram) => UnitTag::Mg,
            Unit::Mass(MassUnit::Gram) => UnitTag::G,
            Unit::Mass(MassUnit::Kilogram) => UnitTag::Kg,
            Unit::Mass(MassUnit::Ounce) => UnitTag::Oz,
            Unit::Mass(MassUnit::Pound) => UnitTag::Lb,
            Unit::Volume(VolumeUnit::Milliliter) => UnitTag::Ml,
            Unit::Volume(VolumeUnit::Centiliter) => UnitTag::Cl,
            Unit::Volume(VolumeUnit::Deciliter) => UnitTag::Dl,
            Unit::Volume(VolumeUnit::Liter) => UnitTag::L,
            Unit::Volume(VolumeUnit::TeaspoonFr) => UnitTag::TspFr,
            Unit::Volume(VolumeUnit::TablespoonFr) => UnitTag::TbspFr,
            Unit::Volume(VolumeUnit::CupMetric) => UnitTag::CupMetric,
            Unit::Volume(VolumeUnit::TeaspoonUs) => UnitTag::TspUs,
            Unit::Volume(VolumeUnit::TablespoonUs) => UnitTag::TbspUs,
            Unit::Volume(VolumeUnit::CupUs) => UnitTag::CupUs,
            Unit::Volume(VolumeUnit::FluidOunceUs) => UnitTag::FlozUs,
            Unit::Piece => UnitTag::Piece,
            Unit::Pinch => UnitTag::Pinch,
            Unit::ToTaste => UnitTag::ToTaste,
        }
    }
}

impl From<UnitTag> for Unit {
    fn from(tag: UnitTag) -> Self {
        match tag {
            UnitTag::Mg => Unit::Mass(MassUnit::Milligram),
            UnitTag::G => Unit::Mass(MassUnit::Gram),
            UnitTag::Kg => Unit::Mass(MassUnit::Kilogram),
            UnitTag::Oz => Unit::Mass(MassUnit::Ounce),
            UnitTag::Lb => Unit::Mass(MassUnit::Pound),
            UnitTag::Ml => Unit::Volume(VolumeUnit::Milliliter),
            UnitTag::Cl => Unit::Volume(VolumeUnit::Centiliter),
            UnitTag::Dl => Unit::Volume(VolumeUnit::Deciliter),
            UnitTag::L => Unit::Volume(VolumeUnit::Liter),
            UnitTag::TspFr => Unit::Volume(VolumeUnit::TeaspoonFr),
            UnitTag::TbspFr => Unit::Volume(VolumeUnit::TablespoonFr),
            UnitTag::CupMetric => Unit::Volume(VolumeUnit::CupMetric),
            UnitTag::TspUs => Unit::Volume(VolumeUnit::TeaspoonUs),
            UnitTag::TbspUs => Unit::Volume(VolumeUnit::TablespoonUs),
            UnitTag::CupUs => Unit::Volume(VolumeUnit::CupUs),
            UnitTag::FlozUs => Unit::Volume(VolumeUnit::FluidOunceUs),
            UnitTag::Piece => Unit::Piece,
            UnitTag::Pinch => Unit::Pinch,
            UnitTag::ToTaste => Unit::ToTaste,
        }
    }
}

/// Where the item is found in the shop. Declaration order is the walking
/// order the cart sorts by — that ordering lives in `domain`, and this is
/// only its name on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub enum AisleTag {
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

impl From<Aisle> for AisleTag {
    fn from(aisle: Aisle) -> Self {
        match aisle {
            Aisle::Produce => AisleTag::Produce,
            Aisle::Butcher => AisleTag::Butcher,
            Aisle::Fish => AisleTag::Fish,
            Aisle::Deli => AisleTag::Deli,
            Aisle::Dairy => AisleTag::Dairy,
            Aisle::Bakery => AisleTag::Bakery,
            Aisle::Grocery => AisleTag::Grocery,
            Aisle::Frozen => AisleTag::Frozen,
            Aisle::Beverages => AisleTag::Beverages,
            Aisle::Household => AisleTag::Household,
            Aisle::Other => AisleTag::Other,
        }
    }
}

impl From<AisleTag> for Aisle {
    fn from(tag: AisleTag) -> Self {
        match tag {
            AisleTag::Produce => Aisle::Produce,
            AisleTag::Butcher => Aisle::Butcher,
            AisleTag::Fish => Aisle::Fish,
            AisleTag::Deli => Aisle::Deli,
            AisleTag::Dairy => Aisle::Dairy,
            AisleTag::Bakery => Aisle::Bakery,
            AisleTag::Grocery => Aisle::Grocery,
            AisleTag::Frozen => Aisle::Frozen,
            AisleTag::Beverages => Aisle::Beverages,
            AisleTag::Household => Aisle::Household,
            AisleTag::Other => Aisle::Other,
        }
    }
}

/// What a cart line shows. `Checked` and `AutoChecked` are both settled but
/// they do not mean the same thing, and the UI keeps them in two separate
/// sections for that reason (DECISIONS 0023).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub enum CheckStateTag {
    ToBuy,
    Checked,
    AutoChecked,
}

impl From<&CheckState> for CheckStateTag {
    fn from(state: &CheckState) -> Self {
        match state {
            CheckState::ToBuy => CheckStateTag::ToBuy,
            CheckState::Checked { .. } => CheckStateTag::Checked,
            CheckState::AutoChecked => CheckStateTag::AutoChecked,
        }
    }
}

/// How an ingredient reference renders inside an instruction step. The app
/// applies the rule and sends the pieces to show; this tag travels with them
/// so the UI can style a first mention differently from a later one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub enum RefDisplayTag {
    Full,
    NameOnly,
    QuantityOnly,
}

impl From<RefDisplay> for RefDisplayTag {
    fn from(display: RefDisplay) -> Self {
        match display {
            RefDisplay::Full => RefDisplayTag::Full,
            RefDisplay::NameOnly => RefDisplayTag::NameOnly,
            RefDisplay::QuantityOnly => RefDisplayTag::QuantityOnly,
        }
    }
}

impl From<RefDisplayTag> for RefDisplay {
    fn from(tag: RefDisplayTag) -> Self {
        match tag {
            RefDisplayTag::Full => RefDisplay::Full,
            RefDisplayTag::NameOnly => RefDisplay::NameOnly,
            RefDisplayTag::QuantityOnly => RefDisplay::QuantityOnly,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every unit the domain defines, listed by hand — so a new variant fails
    /// here instead of reaching the frontend as a serialisation panic.
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

    const ALL_AISLES: [Aisle; 11] = [
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
        Aisle::Other,
    ];

    #[test]
    fn every_unit_survives_the_round_trip_to_the_frontend_and_back() {
        for unit in ALL_UNITS {
            assert_eq!(Unit::from(UnitTag::from(unit)), unit);
        }
    }

    #[test]
    fn every_aisle_survives_the_round_trip() {
        for aisle in ALL_AISLES {
            assert_eq!(Aisle::from(AisleTag::from(aisle)), aisle);
        }
    }

    #[test]
    fn a_reference_display_survives_the_round_trip() {
        for display in [
            RefDisplay::Full,
            RefDisplay::NameOnly,
            RefDisplay::QuantityOnly,
        ] {
            assert_eq!(RefDisplay::from(RefDisplayTag::from(display)), display);
        }
    }
}
