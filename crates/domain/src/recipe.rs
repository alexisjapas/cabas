//! Recipes: ingredient usages, sub-recipe references, and instruction steps
//! whose quantities re-render at the scaled amount.

use std::num::NonZeroU32;

use crate::{IngredientId, Quantity, Rational, RecipeId, UsageId};

/// One line of a recipe's ingredient list.
///
/// Steps reference a *usage*, never an ingredient: a recipe may use flour
/// twice, in different amounts, in different steps (DECISIONS 0022).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngredientUsage {
    pub id: UsageId,
    pub ingredient: IngredientId,
    pub quantity: Quantity,
}

/// How much of a sub-recipe a parent needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubRecipeAmount {
    /// A multiple of the sub-recipe exactly as written: `1/2` of it.
    Factor(Rational),
    /// An absolute amount of what the sub-recipe yields: 200 g of a pastry
    /// that yields 500 g. Requires the sub-recipe to declare a yield — which
    /// is precisely why `servings` alone was not enough (DECISIONS 0017).
    OfYield(Quantity),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubRecipeUsage {
    pub id: UsageId,
    pub recipe: RecipeId,
    pub amount: SubRecipeAmount,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Component {
    Ingredient(IngredientUsage),
    SubRecipe(SubRecipeUsage),
}

/// How an ingredient reference renders inside an instruction step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefDisplay {
    /// "200 g of flour" — the first mention.
    Full,
    /// "the flour" — a later mention, no quantity repeated.
    NameOnly,
    /// "200 g" — the ingredient is already obvious from the sentence.
    QuantityOnly,
}

/// A run of instruction text, or a reference that renders as a scaled
/// quantity. Storing steps as segments rather than a string with embedded
/// markers is what keeps a user edit from breaking a reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Text(String),
    Ingredient { usage: UsageId, display: RefDisplay },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Step {
    pub segments: Vec<Segment>,
}

impl Step {
    pub fn text(s: impl Into<String>) -> Self {
        Self {
            segments: vec![Segment::Text(s.into())],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipe {
    pub id: RecipeId,
    pub name: String,
    /// How many people the quantities as written serve.
    pub servings: NonZeroU32,
    /// What the recipe produces, when it is meaningful — "makes 500 g".
    /// Required to reference this recipe by [`SubRecipeAmount::OfYield`].
    pub yields: Option<Quantity>,
    pub components: Vec<Component>,
    pub steps: Vec<Step>,
}

impl Recipe {
    pub fn new(id: RecipeId, name: impl Into<String>, servings: NonZeroU32) -> Self {
        Self {
            id,
            name: name.into(),
            servings,
            yields: None,
            components: Vec::new(),
            steps: Vec::new(),
        }
    }

    pub fn with_yield(mut self, quantity: Quantity) -> Self {
        self.yields = Some(quantity);
        self
    }

    pub fn with_component(mut self, component: Component) -> Self {
        self.components.push(component);
        self
    }

    pub fn with_step(mut self, step: Step) -> Self {
        self.steps.push(step);
        self
    }

    pub fn usage(&self, id: &UsageId) -> Option<&IngredientUsage> {
        self.components.iter().find_map(|c| match c {
            Component::Ingredient(u) if &u.id == id => Some(u),
            _ => None,
        })
    }

    pub fn ingredient_usages(&self) -> impl Iterator<Item = &IngredientUsage> {
        self.components.iter().filter_map(|c| match c {
            Component::Ingredient(u) => Some(u),
            Component::SubRecipe(_) => None,
        })
    }

    pub fn sub_recipes(&self) -> impl Iterator<Item = &SubRecipeUsage> {
        self.components.iter().filter_map(|c| match c {
            Component::SubRecipe(s) => Some(s),
            Component::Ingredient(_) => None,
        })
    }

    /// Step references that point at no existing usage.
    ///
    /// Strict referential integrity is impossible under a CRDT — one device
    /// deletes an ingredient while another references it — so this reports
    /// rather than prevents: the UI renders the orphan with a warning so it
    /// gets fixed, and never blocks the deletion (DECISIONS 0022).
    pub fn dangling_refs(&self) -> Vec<UsageId> {
        self.steps
            .iter()
            .flat_map(|s| &s.segments)
            .filter_map(|seg| match seg {
                Segment::Ingredient { usage, .. } if self.usage(usage).is_none() => {
                    Some(usage.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// The factor that takes this recipe from its written servings to
    /// `wanted`.
    pub fn factor_for_servings(&self, wanted: NonZeroU32) -> Rational {
        Rational::new(i128::from(wanted.get()), i128::from(self.servings.get()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::{MassUnit, Unit};

    fn nz(n: u32) -> NonZeroU32 {
        NonZeroU32::new(n).expect("non-zero")
    }

    const G: Unit = Unit::Mass(MassUnit::Gram);

    fn tart() -> Recipe {
        Recipe::new(RecipeId::from_raw("tart"), "Apple tart", nz(4))
            .with_component(Component::Ingredient(IngredientUsage {
                id: UsageId::from_raw("u_flour"),
                ingredient: IngredientId::from_raw("flour"),
                quantity: Quantity::whole(200, G),
            }))
            .with_step(Step {
                segments: vec![
                    Segment::Text("Add ".into()),
                    Segment::Ingredient {
                        usage: UsageId::from_raw("u_flour"),
                        display: RefDisplay::Full,
                    },
                    Segment::Text(" and mix.".into()),
                ],
            })
    }

    #[test]
    fn servings_factor_is_exact() {
        let r = tart();
        assert_eq!(r.factor_for_servings(nz(6)), Rational::new(3, 2));
        assert_eq!(r.factor_for_servings(nz(4)), Rational::from_integer(1));
        assert_eq!(r.factor_for_servings(nz(1)), Rational::new(1, 4));
    }

    #[test]
    fn a_step_reference_resolves_to_its_usage() {
        let r = tart();
        let usage = r.usage(&UsageId::from_raw("u_flour")).expect("present");
        assert_eq!(usage.quantity, Quantity::whole(200, G));
        assert!(r.dangling_refs().is_empty());
    }

    #[test]
    fn deleting_a_referenced_usage_is_reported_not_prevented() {
        let mut r = tart();
        r.components.clear();
        assert_eq!(r.dangling_refs(), vec![UsageId::from_raw("u_flour")]);
        // The step itself is untouched and still renderable.
        assert_eq!(r.steps.len(), 1);
    }

    #[test]
    fn components_split_into_ingredients_and_sub_recipes() {
        let r = tart().with_component(Component::SubRecipe(SubRecipeUsage {
            id: UsageId::from_raw("u_pastry"),
            recipe: RecipeId::from_raw("pastry"),
            amount: SubRecipeAmount::Factor(Rational::new(1, 2)),
        }));
        assert_eq!(r.ingredient_usages().count(), 1);
        assert_eq!(r.sub_recipes().count(), 1);
        // A sub-recipe usage is not addressable as an ingredient usage.
        assert!(r.usage(&UsageId::from_raw("u_pastry")).is_none());
    }
}
