//! Turning the library into what is on screen.
//!
//! Two steps, and the first one is the interesting one.
//!
//! # Deriving the cart cannot be allowed to fail
//!
//! [`cabas_domain::cart::derive`] refuses a list that names a recipe or an
//! ingredient it cannot find, which is exactly right for a pure function and
//! exactly wrong for a screen: under a CRDT, one device deleting a recipe
//! while the other has it on the list is not an error state, it is Tuesday.
//! An `Err` here would blank the whole cart because of one row.
//!
//! So the list is triaged first. An entry whose recipe is gone is set aside
//! with a [`ProblemView`]; an ingredient that is gone is replaced by a
//! placeholder carrying its own id as a name, so the line stays in the cart
//! and the rest of the trip still works. What is left cannot fail to derive.
//! The user sees a warning and eight ingredients, rather than a warning and
//! nothing.

use std::num::NonZeroU32;

use cabas_domain::cart::{self, Cart};
use cabas_domain::expand::{ExpandError, expand};
use cabas_domain::list::ListItem;
use cabas_domain::recipe::{Component, SubRecipeAmount};
use cabas_domain::{
    Aisle, CheckState, Ingredient, IngredientId, IngredientIndex, Quantity, Rational, Recipe,
    RefDisplay, Segment, ShoppingList,
};

use crate::command::{
    ComponentInput, QuantityInput, RecipeInput, SegmentInput, StepInput, SubRecipeAmountInput,
};
use crate::library::Library;
use crate::number;
use crate::platform::Identity;
use crate::view::*;

/// Which recipe this device has open, and at how many servings. Device-local
/// (Rule 3 territory: it is not a source, and two people reading different
/// recipes is not a conflict).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Focus {
    pub recipe: cabas_domain::RecipeId,
    pub servings: Option<NonZeroU32>,
}

/// The cart, plus everything the list could not make sense of.
pub(crate) struct Projection {
    pub cart: Cart,
    pub problems: Vec<ProblemView>,
}

/// Triages the list, then derives the cart from what survives.
pub(crate) fn derive(library: &Library) -> Projection {
    let mut problems = Vec::new();
    let mut usable = ShoppingList::default();
    let mut ingredients = library.ingredients.clone();

    for entry in &library.list.entries {
        let id = entry.id.to_string();
        match &entry.item {
            ListItem::Recipe { recipe, servings } => {
                let Some(target) = library.recipes.get(recipe) else {
                    problems.push(problem(
                        Some(&id),
                        Some(recipe.as_str()),
                        ProblemKind::MissingRecipe,
                        format!("recipe `{recipe}` is not in the library"),
                    ));
                    continue;
                };
                // Expanded here to find out whether it *can* be, and again
                // inside `derive`. Twice over a handful of list entries is
                // not a cost; a cart that cannot be shown is.
                match expand(
                    recipe,
                    target.factor_for_servings(*servings),
                    &library.recipes,
                ) {
                    Ok(contributions) => {
                        for contribution in contributions {
                            fill_missing(
                                &mut ingredients,
                                &contribution.ingredient,
                                &id,
                                &mut problems,
                            );
                        }
                        usable.entries.push(entry.clone());
                    }
                    Err(error) => problems.push(expansion_problem(&id, &error)),
                }
            }
            ListItem::Ingredient { ingredient, .. } => {
                fill_missing(&mut ingredients, ingredient, &id, &mut problems);
                usable.entries.push(entry.clone());
            }
        }
    }

    let cart = match cart::derive(&usable, &library.recipes, &ingredients, &library.overlay) {
        Ok(cart) => cart,
        // Unreachable: the triage above removed every cause. Reported rather
        // than unwrapped, because "unreachable" is a claim about today's
        // domain code and this is a screen either way.
        Err(error) => {
            problems.push(problem(
                None,
                None,
                ProblemKind::BrokenGraph,
                error.to_string(),
            ));
            Cart::default()
        }
    };

    Projection { cart, problems }
}

/// Keeps a deleted ingredient visible instead of taking the whole entry down
/// with it. The placeholder carries the id as its name — unhelpful to read,
/// but it is the only thing left to say about it, and it makes the row
/// identifiable next to the warning that accompanies it.
fn fill_missing(
    ingredients: &mut IngredientIndex,
    id: &IngredientId,
    entry: &str,
    problems: &mut Vec<ProblemView>,
) {
    if ingredients.contains_key(id) {
        return;
    }
    problems.push(problem(
        Some(entry),
        Some(id.as_str()),
        ProblemKind::MissingIngredient,
        format!("ingredient `{id}` is not in the library"),
    ));
    ingredients.insert(
        id.clone(),
        Ingredient::new(id.clone(), id.as_str(), Aisle::Other),
    );
}

fn expansion_problem(entry: &str, error: &ExpandError) -> ProblemView {
    let (kind, subject) = match error {
        ExpandError::UnknownRecipe(recipe) => (ProblemKind::MissingRecipe, Some(recipe.as_str())),
        ExpandError::Cycle(path) => (ProblemKind::BrokenGraph, path.last().map(|r| r.as_str())),
        ExpandError::DepthExceeded(recipe) => (ProblemKind::BrokenGraph, Some(recipe.as_str())),
        ExpandError::MissingYield(recipe)
        | ExpandError::UnmeasurableYield(recipe)
        | ExpandError::ZeroYield(recipe) => (ProblemKind::BrokenYield, Some(recipe.as_str())),
        ExpandError::YieldDimensionMismatch { recipe, .. } => {
            (ProblemKind::BrokenYield, Some(recipe.as_str()))
        }
    };
    problem(Some(entry), subject, kind, error.to_string())
}

fn problem(
    entry: Option<&str>,
    subject: Option<&str>,
    kind: ProblemKind,
    detail: String,
) -> ProblemView {
    ProblemView {
        entry: entry.map(str::to_owned),
        subject: subject.map(str::to_owned),
        kind,
        detail,
    }
}

// --- the state -------------------------------------------------------------

pub(crate) fn state(
    library: &Library,
    projection: &Projection,
    focus: Option<&Focus>,
    identity: &Identity,
    revision: u64,
) -> StateView {
    let me = identity.user_id();
    StateView {
        revision,
        me: UserView {
            id: identity.user.clone(),
            // The document wins over what the host remembers: the name may
            // have been changed from another device since this one launched.
            name: library
                .user_name(&me)
                .unwrap_or(&identity.user_name)
                .to_owned(),
        },
        people: people_views(library, identity),
        cart: cart_view(library, &projection.cart),
        list: list_view(library, &projection.cart),
        recipes: recipe_summaries(library),
        ingredients: ingredient_views(library),
        focus: focus.and_then(|focus| focus_view(library, focus)),
        problems: projection.problems.clone(),
    }
}

/// The family roster: everyone in the document, each with what they carry.
///
/// A device whose owner is not in the roster is invisible here, which is what
/// [`cabas_domain::devices_of`] decides and why: under a CRDT one replica can
/// delete a user while another pairs a device to them, and a dangling owner
/// must not be able to break the screen (DECISIONS 0022's reasoning, applied
/// to people). No command deletes a user today, so the case cannot arise from
/// this app — it is handled because the document is shared, not because the
/// UI can produce it.
fn people_views(library: &Library, identity: &Identity) -> Vec<PersonView> {
    let me = identity.user_id();
    let this_device = identity.device_id();
    library
        .users
        .iter()
        .map(|user| PersonView {
            id: user.id.to_string(),
            name: user.name.clone(),
            is_me: user.id == me,
            devices: cabas_domain::devices_of(&library.devices, &user.id)
                .map(|device| DeviceView {
                    id: device.id.to_string(),
                    name: device.name.clone(),
                    is_this_one: device.id == this_device,
                    paired_at: device.paired_at.0,
                })
                .collect(),
        })
        .collect()
}

fn cart_view(library: &Library, cart: &Cart) -> CartView {
    let line = |line: &cabas_domain::CartLine| CartLineView {
        ingredient: line.ingredient.to_string(),
        name: line.name.clone(),
        aisle: line.aisle.into(),
        staple: line.staple,
        amounts: line.amounts.iter().map(QuantityView::of).collect(),
        state: (&line.state).into(),
        checked_by: match &line.state {
            CheckState::Checked { by, .. } => library.user_name(by).map(str::to_owned),
            _ => None,
        },
        checked_at: match &line.state {
            CheckState::Checked { at, .. } => Some(at.0),
            _ => None,
        },
        sources: line.sources.iter().map(ToString::to_string).collect(),
    };

    CartView {
        to_buy: cart.to_buy().map(line).collect(),
        bought: cart.bought().map(line).collect(),
        at_home: cart.already_at_home().map(line).collect(),
        remaining: cart.to_buy().count(),
        total: cart.lines.len(),
    }
}

fn list_view(library: &Library, cart: &Cart) -> Vec<ListEntryView> {
    let progress = cart.progress(&library.list);
    library
        .list
        .entries
        .iter()
        .zip(progress)
        .map(|(entry, progress)| ListEntryView {
            id: entry.id.to_string(),
            item: match &entry.item {
                ListItem::Recipe { recipe, servings } => ListItemView::Recipe {
                    recipe: recipe.to_string(),
                    name: library
                        .recipes
                        .get(recipe)
                        .map(|r| r.name.clone())
                        .unwrap_or_default(),
                    servings: servings.get(),
                    written_for: library
                        .recipes
                        .get(recipe)
                        .map_or(servings.get(), |r| r.servings.get()),
                },
                ListItem::Ingredient {
                    ingredient,
                    quantity,
                } => ListItemView::Ingredient {
                    ingredient: ingredient.to_string(),
                    name: library
                        .ingredient_name(ingredient)
                        .unwrap_or_default()
                        .to_owned(),
                    quantity: QuantityView::of(quantity),
                },
            },
            progress: ProgressView {
                settled: progress.settled,
                total: progress.total,
                complete: progress.is_complete(),
            },
            added_by: library.user_name(&entry.added_by).map(str::to_owned),
            added_at: entry.added_at.0,
        })
        .collect()
}

fn recipe_summaries(library: &Library) -> Vec<RecipeSummaryView> {
    let mut summaries: Vec<_> = library
        .recipes
        .values()
        .map(|recipe| RecipeSummaryView {
            id: recipe.id.to_string(),
            name: recipe.name.clone(),
            servings: recipe.servings.get(),
            yields: recipe.yields.as_ref().map(QuantityView::of),
            ingredients: recipe.ingredient_usages().count(),
            sub_recipes: recipe.sub_recipes().count(),
        })
        .collect();
    summaries.sort_by(|a, b| by_name(&a.name, &a.id, &b.name, &b.id));
    summaries
}

fn ingredient_views(library: &Library) -> Vec<IngredientView> {
    let mut views: Vec<_> = library
        .ingredients
        .values()
        .map(|ingredient| IngredientView {
            id: ingredient.id.to_string(),
            name: ingredient.name.clone(),
            aliases: ingredient.aliases.clone(),
            aisle: ingredient.aisle.into(),
            staple: ingredient.staple,
            density: ingredient.density.map(number::render_lossless),
            unit_weight: ingredient.unit_weight.map(number::render_lossless),
        })
        .collect();
    views.sort_by(|a, b| by_name(&a.name, &a.id, &b.name, &b.id));
    views
}

/// Alphabetical, case-insensitively, with the id breaking ties so two devices
/// never disagree about the order of two identically named things.
///
/// Not a locale-aware collation: "é" sorts after "z" here. Fixing that needs
/// a collation table, and it is worth exactly as much as i18n is — which is
/// nothing until the app has more than one language.
fn by_name(a: &str, a_id: &str, b: &str, b_id: &str) -> std::cmp::Ordering {
    a.to_lowercase()
        .cmp(&b.to_lowercase())
        .then_with(|| a_id.cmp(b_id))
}

fn focus_view(library: &Library, focus: &Focus) -> Option<FocusView> {
    let recipe = library.recipes.get(&focus.recipe)?;
    let servings = focus.servings.unwrap_or(recipe.servings);
    let factor = recipe.factor_for_servings(servings);

    Some(FocusView {
        recipe: RecipeView {
            id: recipe.id.to_string(),
            name: recipe.name.clone(),
            written_for: recipe.servings.get(),
            servings: servings.get(),
            yields: recipe
                .yields
                .as_ref()
                .map(|q| QuantityView::of(&q.scaled(factor))),
            components: components_view(library, recipe, factor),
            steps: steps_view(library, recipe, factor),
        },
        edit: edit_input(recipe),
    })
}

fn components_view(library: &Library, recipe: &Recipe, factor: Rational) -> Vec<ComponentView> {
    recipe
        .components
        .iter()
        .map(|component| match component {
            Component::Ingredient(usage) => ComponentView::Ingredient {
                usage: usage.id.to_string(),
                ingredient: usage.ingredient.to_string(),
                name: library
                    .ingredient_name(&usage.ingredient)
                    .map(str::to_owned),
                quantity: QuantityView::of(&usage.quantity.scaled(factor)),
            },
            Component::SubRecipe(sub) => ComponentView::SubRecipe {
                usage: sub.id.to_string(),
                recipe: sub.recipe.to_string(),
                name: library.recipes.get(&sub.recipe).map(|r| r.name.clone()),
                amount: match &sub.amount {
                    SubRecipeAmount::Factor(value) => SubRecipeAmountView::Factor {
                        factor: number::render(*value * factor).text,
                    },
                    SubRecipeAmount::OfYield(quantity) => SubRecipeAmountView::OfYield {
                        quantity: QuantityView::of(&quantity.scaled(factor)),
                    },
                },
            },
        })
        .collect()
}

/// Resolves each step's references at the servings being read.
///
/// The display rule is applied *here*: `name` and `quantity` are present
/// exactly when they are to be shown, so the frontend renders what it is
/// given instead of deciding what a "name only" mention means (Rule 9).
fn steps_view(library: &Library, recipe: &Recipe, factor: Rational) -> Vec<StepView> {
    recipe
        .steps
        .iter()
        .map(|step| StepView {
            segments: step
                .segments
                .iter()
                .map(|segment| match segment {
                    Segment::Text(text) => SegmentView::Text { text: text.clone() },
                    Segment::Ingredient { usage, display } => match recipe.usage(usage) {
                        Some(line) => {
                            let quantity = QuantityView::of(&line.quantity.scaled(factor));
                            let name = library.ingredient_name(&line.ingredient).map(str::to_owned);
                            SegmentView::Ingredient {
                                usage: usage.to_string(),
                                name: match display {
                                    RefDisplay::Full | RefDisplay::NameOnly => name,
                                    RefDisplay::QuantityOnly => None,
                                },
                                quantity: match display {
                                    RefDisplay::Full | RefDisplay::QuantityOnly => Some(quantity),
                                    RefDisplay::NameOnly => None,
                                },
                                display: (*display).into(),
                            }
                        }
                        // The usage was deleted while the step still points at
                        // it. Rendered as a warning in place; never a panic,
                        // and never a reason to have blocked the deletion.
                        None => SegmentView::Missing {
                            usage: usage.to_string(),
                        },
                    },
                })
                .collect(),
        })
        .collect()
}

/// The recipe as written, in the shape `SaveRecipe` accepts.
///
/// Quantities go through [`number::render_lossless`] rather than the pretty
/// rendering: this is a form that will be sent back, and a form that rounds
/// what it displays is a form that silently rewrites what it was given.
fn edit_input(recipe: &Recipe) -> RecipeInput {
    RecipeInput {
        id: Some(recipe.id.to_string()),
        name: recipe.name.clone(),
        servings: recipe.servings.get(),
        yields: recipe.yields.as_ref().map(quantity_input),
        components: recipe
            .components
            .iter()
            .map(|component| match component {
                Component::Ingredient(usage) => ComponentInput::Ingredient {
                    id: Some(usage.id.to_string()),
                    ingredient: usage.ingredient.to_string(),
                    quantity: quantity_input(&usage.quantity),
                },
                Component::SubRecipe(sub) => ComponentInput::SubRecipe {
                    id: Some(sub.id.to_string()),
                    recipe: sub.recipe.to_string(),
                    amount: match &sub.amount {
                        SubRecipeAmount::Factor(value) => SubRecipeAmountInput::Factor {
                            factor: number::render_lossless(*value),
                        },
                        SubRecipeAmount::OfYield(quantity) => SubRecipeAmountInput::OfYield {
                            quantity: quantity_input(quantity),
                        },
                    },
                },
            })
            .collect(),
        steps: recipe
            .steps
            .iter()
            .map(|step| StepInput {
                segments: step
                    .segments
                    .iter()
                    .map(|segment| match segment {
                        Segment::Text(text) => SegmentInput::Text { text: text.clone() },
                        Segment::Ingredient { usage, display } => SegmentInput::Ingredient {
                            usage: usage.to_string(),
                            display: (*display).into(),
                        },
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn quantity_input(quantity: &Quantity) -> QuantityInput {
    QuantityInput {
        amount: number::render_lossless(quantity.value),
        unit: quantity.unit.into(),
    }
}
