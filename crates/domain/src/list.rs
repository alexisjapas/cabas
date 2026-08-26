//! The shopping list — recipes and bare ingredients, in one single list.

use std::num::NonZeroU32;

use crate::ingredient::Ingredient;
use crate::overlay::Overlay;
use crate::units::Dimension;
use crate::{IngredientId, ListEntryId, Quantity, Rational, RecipeId, Timestamp, UserId};

/// What a list entry asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListItem {
    /// A recipe, for a possibly different number of people than it is
    /// written for.
    Recipe {
        recipe: RecipeId,
        servings: NonZeroU32,
    },
    /// A bare ingredient, added by hand.
    Ingredient {
        ingredient: IngredientId,
        quantity: Quantity,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListEntry {
    pub id: ListEntryId,
    pub item: ListItem,
    /// Declarative attribution — a convenience, never access control
    /// (DECISIONS 0024).
    pub added_by: UserId,
    pub added_at: Timestamp,
}

/// The one shopping list (DECISIONS 0018). It has no name and no id because
/// there is never a second one to tell it apart from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShoppingList {
    pub entries: Vec<ListEntry>,
}

impl ShoppingList {
    pub fn entry(&self, id: &ListEntryId) -> Option<&ListEntry> {
        self.entries.iter().find(|e| &e.id == id)
    }

    pub fn remove(&mut self, id: &ListEntryId) {
        self.entries.retain(|e| &e.id != id);
    }

    /// Adds an entry, purging the overlay entry of a bare ingredient.
    ///
    /// The purge is the load-bearing half (Rule 3): putting an ingredient on
    /// the list by hand means "I need this", so it must return to its derived
    /// default and become visible — whether it was auto-checked as a staple or
    /// checked off earlier in the same trip. Adding a *recipe* purges nothing,
    /// since it makes no statement about any single ingredient.
    pub fn add(&mut self, entry: ListEntry, overlay: &mut Overlay) {
        if let ListItem::Ingredient { ingredient, .. } = &entry.item {
            overlay.remove(ingredient);
        }
        self.entries.push(entry);
    }
}

/// What nudging one list entry leaves behind (DECISIONS 0072).
///
/// A nudge is one notch of the gesture that put the line there in the first
/// place — drag right for one more, left for one less — so what a notch is
/// worth is the amount that gesture would have added, and the arithmetic
/// belongs here rather than in the frontend (Rule 9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Nudged {
    /// The line stays, asking for this instead.
    To(Quantity),
    /// The line comes off the list. One less than the last one is not zero of
    /// something — it is nothing to buy, and a list that keeps a row asking
    /// for none of a thing is a list you have to read twice.
    Off,
    /// Nothing happens. The notch and what is already on the line do not
    /// share a dimension and the ingredient carries no coefficient to cross
    /// it, so there is no honest sum to write down (Rule 5) — and an amount
    /// that is not measured at all ("au goût") has no notches to count.
    Refused,
}

/// Adds `steps` notches of what one usually buys to what the line already
/// asks for, in the line's own unit.
///
/// The notch is [`Ingredient::shopping_quantity`] — the same amount a swipe
/// puts on the list from nothing (DECISIONS 0066) — so a kilo of flour goes
/// 1 kg → 2 kg → 3 kg, and an ingredient nobody has sized counts in pieces.
///
/// The notch is converted into the line's unit rather than the line into the
/// notch's, because the line is what a person reads in a shop: typing "500 g"
/// and then nudging must not turn the row into kilos.
pub fn nudge_quantity(current: &Quantity, ingredient: &Ingredient, steps: i32) -> Nudged {
    if steps == 0 {
        return Nudged::To(current.clone());
    }
    // Pinches and "au goût" are excluded before any arithmetic: `scaled`
    // deliberately ignores a factor on an unmeasured quantity, so a negative
    // notch would otherwise *add* one.
    if current.dimension() == Dimension::Unmeasured {
        return Nudged::Refused;
    }

    let notch = ingredient.shopping_quantity();
    let notch = if notch.dimension() == current.dimension() {
        notch
    } else {
        match ingredient.convert(&notch, current.dimension()) {
            Some(converted) => converted,
            None => return Nudged::Refused,
        }
    };

    let steps = Rational::from_integer(i128::from(steps));
    match current.try_add(&notch.scaled(steps)) {
        Some(total) if total.value > Rational::from_integer(0) => Nudged::To(total),
        Some(_) => Nudged::Off,
        None => Nudged::Refused,
    }
}

/// The same notch for a recipe on the list: one whole recipe, as written.
///
/// A recipe for four goes 4 → 8 → 12 rather than 4 → 5, because what a notch
/// adds is *another one of these*, and a recipe's own serving count is the
/// size it comes in. `None` takes the line off the list, for the reason
/// [`Nudged::Off`] gives.
pub fn nudge_servings(
    current: NonZeroU32,
    written_for: NonZeroU32,
    steps: i32,
) -> Option<NonZeroU32> {
    let total = i64::from(current.get()) + i64::from(written_for.get()) * i64::from(steps);
    u32::try_from(total).ok().and_then(NonZeroU32::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingredient::Aisle;
    use crate::overlay::Explicit;
    use crate::units::{MassUnit, Unit, VolumeUnit};

    const G: Unit = Unit::Mass(MassUnit::Gram);

    fn ingredient_entry(id: &str, ingredient: &str) -> ListEntry {
        ListEntry {
            id: ListEntryId::from_raw(id),
            item: ListItem::Ingredient {
                ingredient: IngredientId::from_raw(ingredient),
                quantity: Quantity::whole(100, G),
            },
            added_by: UserId::from_raw("alice"),
            added_at: Timestamp(0),
        }
    }

    #[test]
    fn adding_an_ingredient_purges_its_overlay_entry() {
        let mut overlay: Overlay = [(
            IngredientId::from_raw("salt"),
            Explicit::Checked {
                by: UserId::from_raw("bob"),
                at: Timestamp(5),
            },
        )]
        .into_iter()
        .collect();

        let mut list = ShoppingList::default();
        list.add(ingredient_entry("e1", "salt"), &mut overlay);

        assert!(overlay.is_empty(), "the explicit action must be cleared");
        assert_eq!(list.entries.len(), 1);
    }

    #[test]
    fn adding_a_recipe_purges_nothing() {
        let mut overlay: Overlay = [(IngredientId::from_raw("salt"), Explicit::Unchecked)]
            .into_iter()
            .collect();
        let mut list = ShoppingList::default();
        list.add(
            ListEntry {
                id: ListEntryId::from_raw("e1"),
                item: ListItem::Recipe {
                    recipe: RecipeId::from_raw("tart"),
                    servings: NonZeroU32::new(4).expect("non-zero"),
                },
                added_by: UserId::from_raw("alice"),
                added_at: Timestamp(0),
            },
            &mut overlay,
        );
        assert_eq!(overlay.len(), 1);
    }

    #[test]
    fn entries_can_be_looked_up_and_removed() {
        let mut overlay = Overlay::new();
        let mut list = ShoppingList::default();
        list.add(ingredient_entry("e1", "salt"), &mut overlay);
        list.add(ingredient_entry("e2", "flour"), &mut overlay);

        assert!(list.entry(&ListEntryId::from_raw("e1")).is_some());
        list.remove(&ListEntryId::from_raw("e1"));
        assert!(list.entry(&ListEntryId::from_raw("e1")).is_none());
        assert_eq!(list.entries.len(), 1);
    }

    // --- nudging (DECISIONS 0072) -----------------------------------------

    const ML: Unit = Unit::Volume(VolumeUnit::Milliliter);
    const KG: Unit = Unit::Mass(MassUnit::Kilogram);

    /// A kilo at a time.
    fn flour() -> Ingredient {
        Ingredient::new(IngredientId::from_raw("flour"), "Farine", Aisle::Pantry)
            .with_default_quantity(Quantity::whole(1, KG))
    }

    #[test]
    fn a_notch_is_what_one_usually_buys() {
        let one_kilo = Quantity::whole(1, KG);
        assert_eq!(
            nudge_quantity(&one_kilo, &flour(), 1),
            Nudged::To(Quantity::whole(2, KG))
        );
        assert_eq!(
            nudge_quantity(&one_kilo, &flour(), 2),
            Nudged::To(Quantity::whole(3, KG))
        );
    }

    #[test]
    fn a_notch_lands_in_the_unit_the_line_is_written_in() {
        // Typed "500 g" by hand, then nudged: the row must stay in grams,
        // because that is what a person is reading in the shop.
        let typed = Quantity::whole(500, G);
        assert_eq!(
            nudge_quantity(&typed, &flour(), 1),
            Nudged::To(Quantity::whole(1500, G))
        );
    }

    #[test]
    fn an_ingredient_nobody_has_sized_counts_in_pieces() {
        let plain = Ingredient::new(IngredientId::from_raw("tomato"), "Tomate", Aisle::Produce);
        assert_eq!(
            nudge_quantity(&Quantity::whole(1, Unit::Piece), &plain, 1),
            Nudged::To(Quantity::whole(2, Unit::Piece))
        );
    }

    #[test]
    fn nudging_the_last_one_down_takes_the_line_off_the_list() {
        let one_kilo = Quantity::whole(1, KG);
        assert_eq!(nudge_quantity(&one_kilo, &flour(), -1), Nudged::Off);
        // And past it, rather than going negative.
        assert_eq!(nudge_quantity(&one_kilo, &flour(), -3), Nudged::Off);
    }

    #[test]
    fn a_notch_crosses_dimensions_only_with_the_ingredients_own_coefficient() {
        // 0.55 g/ml, and a kilo at a time: 1 kg of flour is 1000/0.55 ml.
        let dense = flour().with_density(Rational::new(55, 100));
        let Nudged::To(total) = nudge_quantity(&Quantity::whole(1000, ML), &dense, 1) else {
            panic!("a density makes the sum honest");
        };
        assert_eq!(total.unit, ML);
        assert_eq!(
            total.value,
            Rational::from_integer(1000) + Rational::new(100_000, 55)
        );

        // Without one, two honest lines beat one invented number (Rule 5) —
        // which here means the gesture does nothing at all.
        assert_eq!(
            nudge_quantity(&Quantity::whole(1000, ML), &flour(), 1),
            Nudged::Refused
        );
    }

    #[test]
    fn an_unmeasured_line_has_no_notches() {
        // `scaled` ignores a factor on an unmeasured quantity by design, so
        // this is the guard that stops "one less" from adding one.
        assert_eq!(
            nudge_quantity(&Quantity::to_taste(), &flour(), -1),
            Nudged::Refused
        );
        assert_eq!(
            nudge_quantity(&Quantity::whole(1, Unit::Pinch), &flour(), 1),
            Nudged::Refused
        );
    }

    #[test]
    fn a_recipe_is_nudged_by_a_whole_recipe() {
        let four = NonZeroU32::new(4).expect("non-zero");
        assert_eq!(nudge_servings(four, four, 1), NonZeroU32::new(8));
        assert_eq!(nudge_servings(four, four, 2), NonZeroU32::new(12));
        // Down to nothing takes it off the list, rather than to zero people.
        assert_eq!(nudge_servings(four, four, -1), None);

        // A line rescaled by hand keeps counting in the recipe's own size.
        let six = NonZeroU32::new(6).expect("non-zero");
        assert_eq!(nudge_servings(six, four, -1), NonZeroU32::new(2));
    }
}
