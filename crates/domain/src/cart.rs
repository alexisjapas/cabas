//! Deriving the cart from the list.
//!
//! The cart is a pure function of the sources plus the overlay, recomputed on
//! every change and never stored (Rule 3). The pipeline is:
//!
//! ```text
//! list -> expand recipes -> group by ingredient -> merge units -> resolve
//!         check state -> sort by aisle
//! ```

use std::collections::{BTreeMap, BTreeSet};

use crate::expand::{ExpandError, RecipeIndex, expand};
use crate::ingredient::{Aisle, Ingredient};
use crate::list::{ListItem, ShoppingList};
use crate::overlay::{CheckState, Overlay, resolve};
use crate::units::{Dimension, Unit};
use crate::{IngredientId, ListEntryId, Quantity, Rational, RecipeId};

/// The ingredient library, keyed by id.
pub type IngredientIndex = BTreeMap<IngredientId, Ingredient>;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CartError {
    #[error(transparent)]
    Expand(#[from] ExpandError),

    #[error("ingredient `{0}` is not in the library")]
    UnknownIngredient(IngredientId),

    #[error("recipe `{0}` is not in the library")]
    UnknownRecipe(RecipeId),
}

/// One row of the cart: everything the list asks for of one ingredient.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartLine {
    pub ingredient: IngredientId,
    /// Denormalised for sorting and display.
    pub name: String,
    pub aisle: Aisle,
    pub staple: bool,
    /// Usually one amount. More than one when the contributions could not all
    /// be merged — "300 g + 2 tbsp" is the honest rendering when no density is
    /// known (Rule 5).
    pub amounts: Vec<Quantity>,
    pub state: CheckState,
    /// Which list entries asked for this. Checking one line advances every one
    /// of them at once.
    pub sources: BTreeSet<ListEntryId>,
}

impl CartLine {
    pub fn is_settled(&self) -> bool {
        self.state.is_settled()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cart {
    /// Sorted by aisle, then name — the walking order through the shop.
    pub lines: Vec<CartLine>,
}

/// How far through a list entry the shopping has got.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryProgress {
    pub entry: ListEntryId,
    pub settled: usize,
    pub total: usize,
}

impl EntryProgress {
    /// An entry that contributes nothing is deliberately never complete: an
    /// empty recipe is a data error, and it should stay visible rather than
    /// silently vanish from the list.
    pub fn is_complete(&self) -> bool {
        self.total > 0 && self.settled == self.total
    }
}

impl Cart {
    pub fn line(&self, ingredient: &IngredientId) -> Option<&CartLine> {
        self.lines.iter().find(|l| &l.ingredient == ingredient)
    }

    /// What is still to pick up — the working view in the shop.
    pub fn to_buy(&self) -> impl Iterator<Item = &CartLine> {
        self.lines.iter().filter(|l| !l.is_settled())
    }

    /// The "Bought" section: someone put it in the trolley.
    pub fn bought(&self) -> impl Iterator<Item = &CartLine> {
        self.lines
            .iter()
            .filter(|l| matches!(l.state, CheckState::Checked { .. }))
    }

    /// The "Already at home" section — kept apart from [`Cart::bought`]
    /// because the two do not mean the same thing, and merging them makes
    /// unchecking a staple hard to discover (DECISIONS 0023).
    pub fn already_at_home(&self) -> impl Iterator<Item = &CartLine> {
        self.lines
            .iter()
            .filter(|l| matches!(l.state, CheckState::AutoChecked))
    }

    /// Per-entry progress, in list order. A recipe shows "5/7" until every
    /// ingredient it contributed is settled (DECISIONS 0020).
    pub fn progress(&self, list: &ShoppingList) -> Vec<EntryProgress> {
        let mut counts: BTreeMap<ListEntryId, (usize, usize)> = list
            .entries
            .iter()
            .map(|e| (e.id.clone(), (0, 0)))
            .collect();

        for line in &self.lines {
            for source in &line.sources {
                if let Some(count) = counts.get_mut(source) {
                    count.1 += 1;
                    if line.is_settled() {
                        count.0 += 1;
                    }
                }
            }
        }

        list.entries
            .iter()
            .map(|e| {
                let (settled, total) = counts[&e.id];
                EntryProgress {
                    entry: e.id.clone(),
                    settled,
                    total,
                }
            })
            .collect()
    }
}

/// Derives the cart. See the module docs for the pipeline.
pub fn derive(
    list: &ShoppingList,
    recipes: &RecipeIndex,
    ingredients: &IngredientIndex,
    overlay: &Overlay,
) -> Result<Cart, CartError> {
    #[derive(Default)]
    struct Accumulator {
        quantities: Vec<Quantity>,
        sources: BTreeSet<ListEntryId>,
        from_manual_entry: bool,
    }

    let mut accumulated: BTreeMap<IngredientId, Accumulator> = BTreeMap::new();

    for entry in &list.entries {
        match &entry.item {
            ListItem::Recipe { recipe, servings } => {
                let target = recipes
                    .get(recipe)
                    .ok_or_else(|| CartError::UnknownRecipe(recipe.clone()))?;
                let factor = target.factor_for_servings(*servings);
                for contribution in expand(recipe, factor, recipes)? {
                    let slot = accumulated.entry(contribution.ingredient).or_default();
                    slot.quantities.push(contribution.quantity);
                    slot.sources.insert(entry.id.clone());
                }
            }
            ListItem::Ingredient {
                ingredient,
                quantity,
            } => {
                let slot = accumulated.entry(ingredient.clone()).or_default();
                slot.quantities.push(quantity.clone());
                slot.sources.insert(entry.id.clone());
                slot.from_manual_entry = true;
            }
        }
    }

    let mut lines = Vec::with_capacity(accumulated.len());
    for (id, slot) in accumulated {
        let ingredient = ingredients
            .get(&id)
            .ok_or_else(|| CartError::UnknownIngredient(id.clone()))?;
        let state = resolve(overlay.get(&id), ingredient.staple, slot.from_manual_entry);
        lines.push(CartLine {
            name: ingredient.name.clone(),
            aisle: ingredient.aisle,
            staple: ingredient.staple,
            amounts: merge(ingredient, &slot.quantities),
            state,
            sources: slot.sources,
            ingredient: id,
        });
    }

    lines.sort_by(|a, b| {
        a.aisle
            .cmp(&b.aisle)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.ingredient.cmp(&b.ingredient))
    });

    Ok(Cart { lines })
}

/// Combines every contribution of one ingredient into as few amounts as its
/// coefficients allow.
///
/// Greedy over [`Dimension::MERGE_PREFERENCE`]: take the most preferred
/// dimension present, absorb everything convertible into it, repeat on what is
/// left. Anything that never becomes convertible ends up as its own amount —
/// which is Rule 5 in action.
fn merge(ingredient: &Ingredient, quantities: &[Quantity]) -> Vec<Quantity> {
    let mut by_dimension: BTreeMap<Dimension, Rational> = BTreeMap::new();
    let mut unmeasured: BTreeMap<Unit, Rational> = BTreeMap::new();

    for quantity in quantities {
        match quantity.in_base() {
            Some(base) => {
                *by_dimension
                    .entry(quantity.dimension())
                    .or_insert_with(zero) += base
            }
            // Presence, not magnitude: two recipes saying "to taste" do not
            // add up to two of anything.
            None if quantity.unit == Unit::ToTaste => {
                unmeasured.insert(Unit::ToTaste, Rational::from_integer(1));
            }
            None => *unmeasured.entry(quantity.unit).or_insert_with(zero) += quantity.value,
        }
    }

    let mut amounts = Vec::new();
    let mut remaining: Vec<Dimension> = by_dimension.keys().copied().collect();

    while let Some(&target) = Dimension::MERGE_PREFERENCE
        .iter()
        .find(|d| remaining.contains(*d))
    {
        let mut total = zero();
        let mut absorbed = Vec::new();
        for &dimension in &remaining {
            if dimension != target && !ingredient.can_convert(dimension, target) {
                continue;
            }
            let base_unit = dimension.base_unit().expect("measurable dimension");
            let quantity = Quantity::new(by_dimension[&dimension], base_unit);
            let converted = if dimension == target {
                quantity
            } else {
                ingredient
                    .convert(&quantity, target)
                    .expect("can_convert already agreed")
            };
            total += converted.value;
            absorbed.push(dimension);
        }
        remaining.retain(|d| !absorbed.contains(d));

        let unit = target.base_unit().expect("measurable dimension");
        let mut amount = Quantity::new(total, unit);
        if target == Dimension::Count {
            // You cannot buy 4.33 tomatoes (DECISIONS 0016).
            amount = amount.ceil_to_whole();
        }
        amounts.push(amount.humanized());
    }

    amounts.extend(
        unmeasured
            .into_iter()
            .map(|(unit, value)| Quantity::new(value, unit)),
    );
    amounts
}

fn zero() -> Rational {
    Rational::from_integer(0)
}

/// Ends the trip: removes every completed entry and drops the overlay entries
/// that went with them.
///
/// The overlay is *not* cleared wholesale. An entry you could not finish —
/// the shop was out of one item — stays on the list, and clearing everything
/// would reset the five things you did buy back to unchecked. So an overlay
/// entry is dropped only when every list entry that asked for that ingredient
/// is going away, which is exactly when its cart line disappears too.
pub fn finish_shopping(list: &mut ShoppingList, cart: &Cart, overlay: &mut Overlay) {
    let completed: BTreeSet<ListEntryId> = cart
        .progress(list)
        .into_iter()
        .filter(EntryProgress::is_complete)
        .map(|p| p.entry)
        .collect();

    for line in &cart.lines {
        if line.sources.iter().all(|s| completed.contains(s)) {
            overlay.remove(&line.ingredient);
        }
    }
    list.entries.retain(|e| !completed.contains(&e.id));
}
