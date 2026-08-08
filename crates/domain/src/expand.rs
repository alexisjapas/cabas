//! Flattening the recipe graph into ingredient contributions.
//!
//! Recipes form a **DAG**, not a tree: the same sub-recipe may legitimately
//! appear twice under one parent. What must not happen is a cycle, so
//! expansion carries the current path and refuses to re-enter it.

use std::collections::BTreeMap;

use crate::recipe::{Component, Recipe, SubRecipeAmount};
use crate::units::Dimension;
use crate::{IngredientId, Quantity, Rational, RecipeId, UsageId};

/// The recipe library, keyed by id.
pub type RecipeIndex = BTreeMap<RecipeId, Recipe>;

/// How deep sub-recipes may nest. A pastry inside a tart inside a menu is
/// three; anything approaching this is a modelling mistake, and the bound
/// exists so it surfaces as an error rather than as a stack overflow.
pub const MAX_DEPTH: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExpandError {
    #[error("recipe `{0}` is not in the index")]
    UnknownRecipe(RecipeId),

    #[error("sub-recipe cycle: {}", .0.iter().map(RecipeId::as_str).collect::<Vec<_>>().join(" -> "))]
    Cycle(Vec<RecipeId>),

    #[error("sub-recipes nested deeper than {MAX_DEPTH} at `{0}`")]
    DepthExceeded(RecipeId),

    #[error("recipe `{0}` is referenced by yield but declares none")]
    MissingYield(RecipeId),

    #[error("recipe `{0}` declares an unmeasurable yield, which cannot be divided")]
    UnmeasurableYield(RecipeId),

    #[error("recipe `{0}` declares a zero yield")]
    ZeroYield(RecipeId),

    #[error("recipe `{recipe}` yields a {declared:?} but is referenced as a {requested:?}")]
    YieldDimensionMismatch {
        recipe: RecipeId,
        declared: Dimension,
        requested: Dimension,
    },
}

/// One leaf ingredient requirement, already scaled by every factor on the
/// path from the list entry down to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contribution {
    pub ingredient: IngredientId,
    pub quantity: Quantity,
    /// The usage this came from, for tracing a cart line back to the line of
    /// the recipe that asked for it.
    pub usage: UsageId,
}

/// Flattens `root` scaled by `factor` into leaf ingredient contributions.
///
/// Contributions are **not** merged here — the same ingredient may appear
/// several times. Merging is the cart's job, because only it knows the
/// ingredient coefficients needed to decide what may combine with what.
pub fn expand(
    root: &RecipeId,
    factor: Rational,
    recipes: &RecipeIndex,
) -> Result<Vec<Contribution>, ExpandError> {
    let mut out = Vec::new();
    let mut path = Vec::new();
    expand_into(root, factor, recipes, &mut path, &mut out)?;
    Ok(out)
}

fn expand_into(
    id: &RecipeId,
    factor: Rational,
    recipes: &RecipeIndex,
    path: &mut Vec<RecipeId>,
    out: &mut Vec<Contribution>,
) -> Result<(), ExpandError> {
    if path.contains(id) {
        let mut cycle = path.clone();
        cycle.push(id.clone());
        return Err(ExpandError::Cycle(cycle));
    }
    if path.len() >= MAX_DEPTH {
        return Err(ExpandError::DepthExceeded(id.clone()));
    }

    let recipe = recipes
        .get(id)
        .ok_or_else(|| ExpandError::UnknownRecipe(id.clone()))?;

    path.push(id.clone());
    for component in &recipe.components {
        match component {
            Component::Ingredient(usage) => out.push(Contribution {
                ingredient: usage.ingredient.clone(),
                quantity: usage.quantity.scaled(factor),
                usage: usage.id.clone(),
            }),
            Component::SubRecipe(sub) => {
                let target = recipes
                    .get(&sub.recipe)
                    .ok_or_else(|| ExpandError::UnknownRecipe(sub.recipe.clone()))?;
                let sub_factor = sub_recipe_factor(target, &sub.amount)?;
                expand_into(&sub.recipe, factor * sub_factor, recipes, path, out)?;
            }
        }
    }
    path.pop();
    Ok(())
}

/// How much of `sub`, as written, the parent is asking for.
fn sub_recipe_factor(sub: &Recipe, amount: &SubRecipeAmount) -> Result<Rational, ExpandError> {
    match amount {
        SubRecipeAmount::Factor(f) => Ok(*f),
        SubRecipeAmount::OfYield(requested) => {
            let declared = sub
                .yields
                .as_ref()
                .ok_or_else(|| ExpandError::MissingYield(sub.id.clone()))?;
            if declared.dimension() != requested.dimension() {
                return Err(ExpandError::YieldDimensionMismatch {
                    recipe: sub.id.clone(),
                    declared: declared.dimension(),
                    requested: requested.dimension(),
                });
            }
            let (declared, requested) = (
                declared
                    .in_base()
                    .ok_or_else(|| ExpandError::UnmeasurableYield(sub.id.clone()))?,
                requested
                    .in_base()
                    .ok_or_else(|| ExpandError::UnmeasurableYield(sub.id.clone()))?,
            );
            if *declared.numer() == 0 {
                return Err(ExpandError::ZeroYield(sub.id.clone()));
            }
            Ok(requested / declared)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe::{IngredientUsage, SubRecipeUsage};
    use crate::units::{MassUnit, Unit};
    use std::num::NonZeroU32;

    const G: Unit = Unit::Mass(MassUnit::Gram);

    fn nz(n: u32) -> NonZeroU32 {
        NonZeroU32::new(n).expect("non-zero")
    }

    fn rid(s: &str) -> RecipeId {
        RecipeId::from_raw(s)
    }

    fn ing_component(usage: &str, ingredient: &str, grams: i128) -> Component {
        Component::Ingredient(IngredientUsage {
            id: UsageId::from_raw(usage),
            ingredient: IngredientId::from_raw(ingredient),
            quantity: Quantity::whole(grams, G),
        })
    }

    fn sub_component(usage: &str, recipe: &str, amount: SubRecipeAmount) -> Component {
        Component::SubRecipe(SubRecipeUsage {
            id: UsageId::from_raw(usage),
            recipe: rid(recipe),
            amount,
        })
    }

    /// Pastry: serves 4, yields 500 g, needs 300 g of flour.
    fn pastry() -> Recipe {
        Recipe::new(rid("pastry"), "Shortcrust", nz(4))
            .with_yield(Quantity::whole(500, G))
            .with_component(ing_component("u_p_flour", "flour", 300))
    }

    fn index(recipes: Vec<Recipe>) -> RecipeIndex {
        recipes.into_iter().map(|r| (r.id.clone(), r)).collect()
    }

    #[test]
    fn a_flat_recipe_expands_to_its_own_usages() {
        let idx = index(vec![pastry()]);
        let out = expand(&rid("pastry"), Rational::from_integer(1), &idx).expect("expands");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].quantity, Quantity::whole(300, G));
        assert_eq!(out[0].usage, UsageId::from_raw("u_p_flour"));
    }

    #[test]
    fn factors_multiply_down_the_path() {
        // Tart uses half a pastry; asking for double the tart wants a whole one.
        let tart = Recipe::new(rid("tart"), "Tart", nz(4)).with_component(sub_component(
            "u_t_pastry",
            "pastry",
            SubRecipeAmount::Factor(Rational::new(1, 2)),
        ));
        let idx = index(vec![tart, pastry()]);

        let once = expand(&rid("tart"), Rational::from_integer(1), &idx).expect("expands");
        assert_eq!(once[0].quantity, Quantity::whole(150, G));

        let twice = expand(&rid("tart"), Rational::from_integer(2), &idx).expect("expands");
        assert_eq!(twice[0].quantity, Quantity::whole(300, G));
    }

    #[test]
    fn a_yield_reference_derives_the_factor_from_the_declared_yield() {
        // 200 g of a pastry that yields 500 g = 2/5 of it = 120 g of flour.
        let tart = Recipe::new(rid("tart"), "Tart", nz(4)).with_component(sub_component(
            "u_t_pastry",
            "pastry",
            SubRecipeAmount::OfYield(Quantity::whole(200, G)),
        ));
        let idx = index(vec![tart, pastry()]);
        let out = expand(&rid("tart"), Rational::from_integer(1), &idx).expect("expands");
        assert_eq!(out[0].quantity, Quantity::whole(120, G));
    }

    #[test]
    fn a_yield_reference_to_a_recipe_without_a_yield_is_an_error() {
        let no_yield = Recipe::new(rid("plain"), "Plain", nz(4))
            .with_component(ing_component("u_x", "flour", 10));
        let parent = Recipe::new(rid("parent"), "Parent", nz(4)).with_component(sub_component(
            "u_sub",
            "plain",
            SubRecipeAmount::OfYield(Quantity::whole(100, G)),
        ));
        let idx = index(vec![parent, no_yield]);
        assert_eq!(
            expand(&rid("parent"), Rational::from_integer(1), &idx),
            Err(ExpandError::MissingYield(rid("plain")))
        );
    }

    #[test]
    fn a_yield_reference_in_the_wrong_dimension_is_an_error() {
        let parent = Recipe::new(rid("parent"), "Parent", nz(4)).with_component(sub_component(
            "u_sub",
            "pastry",
            SubRecipeAmount::OfYield(Quantity::whole(2, Unit::Piece)),
        ));
        let idx = index(vec![parent, pastry()]);
        assert_eq!(
            expand(&rid("parent"), Rational::from_integer(1), &idx),
            Err(ExpandError::YieldDimensionMismatch {
                recipe: rid("pastry"),
                declared: Dimension::Mass,
                requested: Dimension::Count,
            })
        );
    }

    #[test]
    fn a_direct_cycle_is_detected() {
        let a = Recipe::new(rid("a"), "A", nz(1)).with_component(sub_component(
            "u",
            "a",
            SubRecipeAmount::Factor(Rational::from_integer(1)),
        ));
        let idx = index(vec![a]);
        assert_eq!(
            expand(&rid("a"), Rational::from_integer(1), &idx),
            Err(ExpandError::Cycle(vec![rid("a"), rid("a")]))
        );
    }

    #[test]
    fn an_indirect_cycle_is_detected() {
        let one = Rational::from_integer(1);
        let a = Recipe::new(rid("a"), "A", nz(1)).with_component(sub_component(
            "ua",
            "b",
            SubRecipeAmount::Factor(one),
        ));
        let b = Recipe::new(rid("b"), "B", nz(1)).with_component(sub_component(
            "ub",
            "c",
            SubRecipeAmount::Factor(one),
        ));
        let c = Recipe::new(rid("c"), "C", nz(1)).with_component(sub_component(
            "uc",
            "a",
            SubRecipeAmount::Factor(one),
        ));
        let idx = index(vec![a, b, c]);
        assert_eq!(
            expand(&rid("a"), one, &idx),
            Err(ExpandError::Cycle(vec![
                rid("a"),
                rid("b"),
                rid("c"),
                rid("a")
            ]))
        );
    }

    #[test]
    fn the_same_sub_recipe_twice_is_a_dag_not_a_cycle() {
        let one = Rational::from_integer(1);
        let menu = Recipe::new(rid("menu"), "Menu", nz(4))
            .with_component(sub_component("u1", "pastry", SubRecipeAmount::Factor(one)))
            .with_component(sub_component("u2", "pastry", SubRecipeAmount::Factor(one)));
        let idx = index(vec![menu, pastry()]);
        let out = expand(&rid("menu"), one, &idx).expect("a DAG expands fine");
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].quantity, Quantity::whole(300, G));
        assert_eq!(out[1].quantity, Quantity::whole(300, G));
    }

    #[test]
    fn an_unknown_recipe_is_reported() {
        let idx = index(vec![]);
        assert_eq!(
            expand(&rid("ghost"), Rational::from_integer(1), &idx),
            Err(ExpandError::UnknownRecipe(rid("ghost")))
        );
    }

    #[test]
    fn nesting_beyond_the_bound_is_reported_not_overflowed() {
        let one = Rational::from_integer(1);
        // A chain r0 -> r1 -> ... -> r20, deeper than MAX_DEPTH.
        let mut recipes = Vec::new();
        for i in 0..20 {
            let name = format!("r{i}");
            let next = format!("r{}", i + 1);
            recipes.push(
                Recipe::new(rid(&name), name.clone(), nz(1)).with_component(sub_component(
                    "u",
                    &next,
                    SubRecipeAmount::Factor(one),
                )),
            );
        }
        recipes.push(Recipe::new(rid("r20"), "r20", nz(1)));
        let idx = index(recipes);
        assert_eq!(
            expand(&rid("r0"), one, &idx),
            Err(ExpandError::DepthExceeded(rid(&format!("r{MAX_DEPTH}"))))
        );
    }

    #[test]
    fn unmeasured_ingredients_survive_scaling_unchanged() {
        let r = Recipe::new(rid("soup"), "Soup", nz(2)).with_component(Component::Ingredient(
            IngredientUsage {
                id: UsageId::from_raw("u_salt"),
                ingredient: IngredientId::from_raw("salt"),
                quantity: Quantity::to_taste(),
            },
        ));
        let idx = index(vec![r]);
        let out = expand(&rid("soup"), Rational::from_integer(10), &idx).expect("expands");
        assert_eq!(out[0].quantity, Quantity::to_taste());
    }
}
