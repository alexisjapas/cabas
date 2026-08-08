//! The shopping list — recipes and bare ingredients, in one single list.

use std::num::NonZeroU32;

use crate::overlay::Overlay;
use crate::{IngredientId, ListEntryId, Quantity, RecipeId, Timestamp, UserId};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlay::Explicit;
    use crate::units::{MassUnit, Unit};

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
}
