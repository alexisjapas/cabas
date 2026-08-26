//! The library as a file: exported, read by a person, imported back.
//!
//! # The file is the app's own inputs
//!
//! Every entry in it is an [`IngredientInput`], a [`RecipeInput`] or a
//! [`ShopInput`] — the very shapes [`crate::Command::SaveIngredient`] and its
//! two siblings already take. There is no second vocabulary for the same
//! things, and therefore no second set of rules to keep in step: an import is
//! the ordinary save, run over what the file says, and a field added to a
//! form appears in the file the day it is added (DECISIONS 0076).
//!
//! It also means the file is honest about what an amount is. Quantities are
//! text there, exactly as they are in a form — `"1,5"`, `"1/3"` — because a
//! JSON number is a float and Rule 4 does not allow one anywhere near a
//! quantity. Rendering uses [`crate::number::render_lossless`] for the reason
//! an edit form does: a rounded amount that gets written back is a quantity
//! that quietly changed.
//!
//! # Ids travel; names are the fallback
//!
//! An entity carries the id it has here. Re-importing this group's own file
//! therefore updates in place, which is what makes the file a backup. A file
//! from *another* group carries ids this document has never seen — so a
//! reference that resolves to nothing by id is resolved by **name**, through
//! the domain's own matchers ([`cabas_domain::Ingredient::matches`] and the
//! two beside it). That is what keeps somebody else's "Farine" from landing
//! as a second flour, and it is what lets a file be written by hand: name
//! things and leave the ids out.
//!
//! # An import merges, and never deletes
//!
//! What the file holds wins over what is already here; what it does not
//! mention is left alone. Nothing is removed, ever. The reason is not
//! caution: a delete under a CRDT is a group-wide fact, so an import that
//! pruned would reach through the relay and take a recipe off the other
//! person's phone — a file opened on one device would silently be a decision
//! made for two.

use std::collections::BTreeMap;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use cabas_domain::recipe::{Component, SubRecipeAmount};
use cabas_domain::{Ingredient, PhotoId, Quantity, Recipe, Shop, ShopId, Timestamp};
use serde::{Deserialize, Serialize};

use crate::command::{
    ComponentInput, IngredientInput, QuantityInput, RecipeInput, SegmentInput, ShopInput,
    SubRecipeAmountInput,
};
use crate::error::{AppError, Result};
use crate::id;
use crate::library::Library;
use crate::number;

/// What a cabas file says it is. Checked on the way in, because the one thing
/// worse than refusing a file is merging something else's into a library.
pub const FORMAT: &str = "cabas.library";

/// The shape of the file, versioned on its own.
///
/// Separate from the app's version (Rule 15) because they answer different
/// questions: the app's says which build wrote the file, this one says
/// whether a build can read it. A file from the future is refused; a file
/// from the past is read, which is the whole point of having the number.
pub const FORMAT_VERSION: u32 = 1;

/// The library, in one file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryFile {
    /// Always [`FORMAT`]. First field on purpose: it is what somebody opening
    /// the file in a text editor reads first.
    pub format: String,
    pub format_version: u32,
    /// The build that wrote it — a note for a human, never a gate.
    #[serde(default)]
    pub app_version: String,
    /// Milliseconds since the epoch, from the writing device's clock. Also a
    /// note: two devices' clocks disagree and nothing here depends on it.
    #[serde(default)]
    pub exported_at: i64,
    #[serde(default)]
    pub shops: Vec<ShopInput>,
    #[serde(default)]
    pub ingredients: Vec<IngredientInput>,
    #[serde(default)]
    pub recipes: Vec<RecipeInput>,
    /// Photo id → the JPEG, base64. Absent when the export was asked for
    /// without them, which is the form to hand-edit: a library of pictures
    /// turns a readable file into a wall of base64 (DECISIONS 0062 has the
    /// arithmetic).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub photos: BTreeMap<String, String>,
}

impl LibraryFile {
    /// Reads a file, refusing anything that is not one.
    ///
    /// Three refusals, and each is a library that would otherwise be quietly
    /// wrong: something that is not JSON at all, something that is JSON and
    /// not a cabas file, and a cabas file written by a build that knows
    /// fields this one would drop on the floor.
    pub fn from_json(text: &str) -> Result<Self> {
        let file: Self = serde_json::from_str(text)
            .map_err(|error| AppError::invalid("file", format!("not readable as JSON: {error}")))?;
        if file.format != FORMAT {
            return Err(AppError::invalid(
                "file",
                format!("not a cabas library: it says it is {:?}", file.format),
            ));
        }
        if file.format_version > FORMAT_VERSION {
            return Err(AppError::invalid(
                "file",
                format!(
                    "written by a newer cabas (file format {}, this build reads {FORMAT_VERSION}) \
                     — update this device first",
                    file.format_version
                ),
            ));
        }
        Ok(file)
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|error| AppError::invalid("file", format!("could not be written: {error}")))
    }

    /// Every photo the entities in this file name, deduplicated and in order.
    ///
    /// What an export has to go and fetch from the photo store, and — read
    /// off a file that arrived — what it *should* have carried. A file whose
    /// entities name photos it does not carry is not an error: the ingredient
    /// arrives without its picture, exactly as it does when the other phone
    /// has taken one and the bytes have not landed yet (Rule 6).
    pub fn photo_ids(&self) -> Vec<PhotoId> {
        let named = self
            .ingredients
            .iter()
            .filter_map(|ingredient| ingredient.photo.as_deref())
            .chain(
                self.recipes
                    .iter()
                    .filter_map(|recipe| recipe.photo.as_deref()),
            );

        let mut ids: Vec<PhotoId> = Vec::new();
        for id in named {
            let id = PhotoId::from_raw(id);
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        ids
    }

    /// Puts one photo's bytes in the file.
    pub fn attach_photo(&mut self, id: &PhotoId, bytes: &[u8]) {
        self.photos.insert(id.to_string(), BASE64.encode(bytes));
    }

    /// The photos this file carries, decoded.
    ///
    /// The one place base64 is read back, and the one place a file that came
    /// from somewhere is allowed to fail late: a photo that does not decode
    /// stops the import before a single entity is written, rather than
    /// leaving half a library behind a picture nobody can open.
    pub fn carried_photos(&self) -> Result<Vec<(PhotoId, Vec<u8>)>> {
        self.photos
            .iter()
            .map(|(id, encoded)| {
                let bytes = BASE64.decode(encoded).map_err(|error| {
                    AppError::invalid("photo", format!("{id} is not readable base64: {error}"))
                })?;
                Ok((PhotoId::from_raw(id.clone()), bytes))
            })
            .collect()
    }
}

/// What one import did.
///
/// Shown, not logged. An import writes as much as a month of ordinary use and
/// the event log is capped at 200 entries (`EventLog::CAP`), so recording it
/// there would push out the deletions the log exists for — and "imported"
/// has no subject to be recorded against without widening the persisted
/// `subject_kind`, which `save_shop` already declined to buy for a row nobody
/// reads. This is the receipt instead, handed straight back to the screen
/// that asked.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct ImportReport {
    pub shops_added: u32,
    pub shops_updated: u32,
    pub ingredients_added: u32,
    pub ingredients_updated: u32,
    pub recipes_added: u32,
    pub recipes_updated: u32,
    /// Photos stored. Counted by the host, which is what awaits the store.
    pub photos_added: u32,
    /// Recipes that arrived naming an ingredient or a sub-recipe this library
    /// does not have — because the file was trimmed to one recipe, or written
    /// by hand with a name nothing answers to.
    ///
    /// Not a failure. The reference is kept and renders as a warning, which
    /// is the defined rendering for a reference that points at nothing
    /// (DECISIONS 0034); naming the recipes here is what turns "there is a
    /// warning somewhere" into a place to look.
    pub incomplete: Vec<String>,
}

/// What an import hands back: the receipt, and the state it produced.
///
/// The state comes with it for the reason every mutation returns one
/// (DECISIONS 0033) — an import changes more of the screen than anything else
/// the app does, and asking for it separately would be a second crossing of
/// the boundary and a window in which the two disagree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct Imported {
    pub report: ImportReport,
    pub state: crate::view::StateView,
}

// --- export -----------------------------------------------------------------

/// The whole library, projected back into the inputs that would have created
/// it.
///
/// Sorted by name rather than left in document order: a file that reorders
/// itself between two exports of an unchanged library is a file nobody can
/// diff, and diffing two exports is how somebody checks what a month changed.
pub(crate) fn export(library: &Library, app_version: &str, at: Timestamp) -> LibraryFile {
    let mut shops: Vec<&Shop> = library.shops.iter().collect();
    shops.sort_by(|a, b| (&a.name, a.id.as_str()).cmp(&(&b.name, b.id.as_str())));

    let mut ingredients: Vec<&Ingredient> = library.ingredients.values().collect();
    ingredients.sort_by(|a, b| (&a.name, a.id.as_str()).cmp(&(&b.name, b.id.as_str())));

    let mut recipes: Vec<&Recipe> = library.recipes.values().collect();
    recipes.sort_by(|a, b| (&a.name, a.id.as_str()).cmp(&(&b.name, b.id.as_str())));

    LibraryFile {
        format: FORMAT.to_owned(),
        format_version: FORMAT_VERSION,
        app_version: app_version.to_owned(),
        exported_at: at.0,
        shops: shops.into_iter().map(shop_input).collect(),
        ingredients: ingredients.into_iter().map(ingredient_input).collect(),
        recipes: recipes.into_iter().map(recipe_input).collect(),
        photos: BTreeMap::new(),
    }
}

fn shop_input(shop: &Shop) -> ShopInput {
    ShopInput {
        id: Some(shop.id.to_string()),
        name: shop.name.clone(),
    }
}

fn ingredient_input(ingredient: &Ingredient) -> IngredientInput {
    IngredientInput {
        id: Some(ingredient.id.to_string()),
        name: ingredient.name.clone(),
        aliases: ingredient.aliases.clone(),
        aisle: ingredient.aisle.into(),
        shops: ingredient.shops.iter().map(ShopId::to_string).collect(),
        keeping: ingredient.keeping.into(),
        staple: ingredient.staple,
        density: ingredient.density.map(number::render_lossless),
        unit_weight: ingredient.unit_weight.map(number::render_lossless),
        default_quantity: ingredient.default_quantity.as_ref().map(quantity_input),
        photo: ingredient.photo.as_ref().map(PhotoId::to_string),
    }
}

fn recipe_input(recipe: &Recipe) -> RecipeInput {
    RecipeInput {
        id: Some(recipe.id.to_string()),
        name: recipe.name.clone(),
        servings: recipe.servings.get(),
        yields: recipe.yields.as_ref().map(quantity_input),
        components: recipe.components.iter().map(component_input).collect(),
        steps: recipe
            .steps
            .iter()
            .map(|step| crate::command::StepInput {
                segments: step
                    .segments
                    .iter()
                    .map(|segment| match segment {
                        cabas_domain::recipe::Segment::Text(text) => {
                            SegmentInput::Text { text: text.clone() }
                        }
                        cabas_domain::recipe::Segment::Ingredient { usage, display } => {
                            SegmentInput::Ingredient {
                                usage: usage.to_string(),
                                display: (*display).into(),
                            }
                        }
                    })
                    .collect(),
            })
            .collect(),
        photo: recipe.photo.as_ref().map(PhotoId::to_string),
    }
}

fn component_input(component: &Component) -> ComponentInput {
    match component {
        Component::Ingredient(usage) => ComponentInput::Ingredient {
            id: Some(usage.id.to_string()),
            ingredient: usage.ingredient.to_string(),
            quantity: quantity_input(&usage.quantity),
        },
        Component::SubRecipe(usage) => ComponentInput::SubRecipe {
            id: Some(usage.id.to_string()),
            recipe: usage.recipe.to_string(),
            amount: match &usage.amount {
                SubRecipeAmount::Factor(factor) => SubRecipeAmountInput::Factor {
                    factor: number::render_lossless(*factor),
                },
                SubRecipeAmount::OfYield(quantity) => SubRecipeAmountInput::OfYield {
                    quantity: quantity_input(quantity),
                },
            },
        },
    }
}

fn quantity_input(quantity: &Quantity) -> QuantityInput {
    QuantityInput {
        amount: number::render_lossless(quantity.value),
        unit: quantity.unit.into(),
    }
}

// --- import -----------------------------------------------------------------

/// The file with every id it names replaced by the id it means *here*, ready
/// to be saved by the ordinary path.
pub(crate) struct Resolved {
    pub shops: Vec<ShopInput>,
    pub ingredients: Vec<IngredientInput>,
    pub recipes: Vec<RecipeInput>,
    pub report: ImportReport,
}

/// Decides what each entry in the file is, here.
///
/// Three passes in dependency order — shops, then ingredients, then recipes —
/// because an ingredient names shops and a recipe names ingredients, and a
/// reference can only be rewritten once its target has an id. Within a pass
/// each entry is matched against the document *and against what earlier
/// entries of the same pass decided*, so a file that names the same shop
/// twice creates it once.
///
/// Nothing is written here. That is deliberate: an import either lands whole
/// or not at all, and the only way to promise that is to do every lookup and
/// every parse before the first `put`.
pub(crate) fn resolve(
    file: &LibraryFile,
    library: &Library,
    mint: &mut dyn FnMut(&'static str) -> Result<String>,
) -> Result<Resolved> {
    let mut report = ImportReport::default();

    // Each pass keeps the document's own entities plus a stand-in for every
    // entity the file has just claimed, so the domain's matchers answer for
    // both. The stand-ins carry the *target* id — what the entity will be
    // called here — which is what makes the reference rewriting below a plain
    // lookup.
    let mut known_shops: Vec<Shop> = library.shops.clone();
    let mut shop_targets: Vec<(String, String)> = Vec::new();
    let mut shops = Vec::with_capacity(file.shops.len());

    for input in &file.shops {
        let held = input
            .id
            .as_deref()
            .and_then(|id| known_shops.iter().find(|shop| shop.id.as_str() == id))
            .or_else(|| cabas_domain::shop::resolve(&known_shops, &input.name));

        let target = match held {
            Some(shop) => {
                report.shops_updated += 1;
                shop.id.to_string()
            }
            None => {
                report.shops_added += 1;
                claim(input.id.as_deref(), id::SHOP, mint)?
            }
        };

        if let Some(named) = &input.id {
            shop_targets.push((named.clone(), target.clone()));
        }
        known_shops.push(Shop::new(ShopId::from_raw(&target), input.name.trim()));
        shops.push(ShopInput {
            id: Some(target),
            name: input.name.clone(),
        });
    }

    let mut known_ingredients: Vec<Ingredient> = library.ingredients.values().cloned().collect();
    let mut ingredient_targets: Vec<(String, String)> = Vec::new();
    let mut ingredients = Vec::with_capacity(file.ingredients.len());

    for input in &file.ingredients {
        let held = input
            .id
            .as_deref()
            .and_then(|id| {
                known_ingredients
                    .iter()
                    .find(|ingredient| ingredient.id.as_str() == id)
            })
            .or_else(|| cabas_domain::ingredient::resolve(&known_ingredients, &input.name));

        let target = match held {
            Some(ingredient) => {
                report.ingredients_updated += 1;
                ingredient.id.to_string()
            }
            None => {
                report.ingredients_added += 1;
                claim(input.id.as_deref(), id::INGREDIENT, mint)?
            }
        };

        if let Some(named) = &input.id {
            ingredient_targets.push((named.clone(), target.clone()));
        }

        let mut stand_in = Ingredient::new(
            cabas_domain::IngredientId::from_raw(&target),
            input.name.trim(),
            input.aisle.into(),
        );
        stand_in.aliases = input.aliases.clone();
        known_ingredients.push(stand_in);

        // A shop reference that answers to nothing is **dropped**, where an
        // ingredient reference below is kept. The asymmetry is the difference
        // between a filter and a content: an unknown shop id would sit on the
        // ingredient forever saying nothing (DECISIONS 0071 reads it as "no
        // shop named"), whereas an unknown ingredient is a line of the recipe
        // and losing it loses the recipe.
        let placed = input
            .shops
            .iter()
            .filter_map(|reference| {
                target_of(reference, &shop_targets)
                    .or_else(|| {
                        known_shops
                            .iter()
                            .find(|shop| shop.id.as_str() == reference)
                            .map(|shop| shop.id.to_string())
                    })
                    .or_else(|| {
                        cabas_domain::shop::resolve(&known_shops, reference)
                            .map(|shop| shop.id.to_string())
                    })
            })
            .collect();

        ingredients.push(IngredientInput {
            id: Some(target),
            shops: placed,
            ..input.clone()
        });
    }

    let mut known_recipes: Vec<Recipe> = library.recipes.values().cloned().collect();
    let mut recipe_targets: Vec<(String, String)> = Vec::new();
    let mut planned = Vec::with_capacity(file.recipes.len());

    // Recipes are claimed before any of them is rewritten, because one recipe
    // may use another that appears later in the file.
    for input in &file.recipes {
        let held = input
            .id
            .as_deref()
            .and_then(|id| known_recipes.iter().find(|recipe| recipe.id.as_str() == id))
            .or_else(|| cabas_domain::recipe::resolve(&known_recipes, &input.name));

        let target = match held {
            Some(recipe) => {
                report.recipes_updated += 1;
                recipe.id.to_string()
            }
            None => {
                report.recipes_added += 1;
                claim(input.id.as_deref(), id::RECIPE, mint)?
            }
        };

        if let Some(named) = &input.id {
            recipe_targets.push((named.clone(), target.clone()));
        }
        known_recipes.push(Recipe::new(
            cabas_domain::RecipeId::from_raw(&target),
            input.name.trim(),
            std::num::NonZeroU32::MIN,
        ));
        planned.push((input, target));
    }

    let mut recipes = Vec::with_capacity(planned.len());
    for (input, target) in planned {
        let mut complete = true;
        let components = input
            .components
            .iter()
            .map(|component| match component {
                ComponentInput::Ingredient {
                    id,
                    ingredient,
                    quantity,
                } => {
                    let resolved = target_of(ingredient, &ingredient_targets)
                        .or_else(|| {
                            known_ingredients
                                .iter()
                                .find(|held| held.id.as_str() == ingredient)
                                .map(|held| held.id.to_string())
                        })
                        .or_else(|| {
                            cabas_domain::ingredient::resolve(&known_ingredients, ingredient)
                                .map(|held| held.id.to_string())
                        });
                    ComponentInput::Ingredient {
                        id: id.clone(),
                        ingredient: resolved.unwrap_or_else(|| {
                            complete = false;
                            ingredient.clone()
                        }),
                        quantity: quantity.clone(),
                    }
                }
                ComponentInput::SubRecipe { id, recipe, amount } => {
                    let resolved = target_of(recipe, &recipe_targets)
                        .or_else(|| {
                            known_recipes
                                .iter()
                                .find(|held| held.id.as_str() == recipe)
                                .map(|held| held.id.to_string())
                        })
                        .or_else(|| {
                            cabas_domain::recipe::resolve(&known_recipes, recipe)
                                .map(|held| held.id.to_string())
                        });
                    ComponentInput::SubRecipe {
                        id: id.clone(),
                        recipe: resolved.unwrap_or_else(|| {
                            complete = false;
                            recipe.clone()
                        }),
                        amount: amount.clone(),
                    }
                }
            })
            .collect();

        if !complete {
            report.incomplete.push(input.name.trim().to_owned());
        }

        recipes.push(RecipeInput {
            id: Some(target),
            components,
            ..input.clone()
        });
    }

    Ok(Resolved {
        shops,
        ingredients,
        recipes,
        report,
    })
}

/// The id a new entity takes: the one the file gave it, or a fresh one.
///
/// Keeping the file's id is what makes re-importing the same file twice
/// idempotent on a device that has never seen it — the second run matches by
/// id and updates. It is safe because an id is 64 random bits minted by
/// [`crate::id`], never anything derived: two libraries cannot have agreed on
/// one by accident.
fn claim(
    named: Option<&str>,
    prefix: &'static str,
    mint: &mut dyn FnMut(&'static str) -> Result<String>,
) -> Result<String> {
    match named {
        Some(id) if !id.trim().is_empty() => Ok(id.to_owned()),
        _ => mint(prefix),
    }
}

/// What the file called something, translated to what it is called here.
fn target_of(reference: &str, targets: &[(String, String)]) -> Option<String> {
    targets
        .iter()
        .find(|(named, _)| named == reference)
        .map(|(_, target)| target.clone())
}
