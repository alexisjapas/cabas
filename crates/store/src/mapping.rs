//! Domain structs in, domain structs out.
//!
//! One pair of functions per persisted entity. Nothing here reaches for a
//! clock, a network or a random number — a mapping that needed any of those
//! would be doing something the domain should have done first.
//!
//! # Writes only touch what changed
//!
//! Every setter compares before it inserts. Two reasons, and the second is
//! the important one:
//!
//! 1. An unchanged field that is rewritten anyway is still an operation in
//!    the history, and the history is what has to stay bounded.
//! 2. It is what makes a coarse `put_recipe(&Recipe)` merge properly. If
//!    saving a recipe rewrote its whole ingredient list every time, then one
//!    person renaming a recipe while the other adds a line would lose the
//!    line: last writer wins over a sequence nobody actually edited. Writing
//!    only the fields that differ turns that into two disjoint edits, which
//!    is the case the CRDT is here for.

use cabas_domain::event::{Action, Subject};
use cabas_domain::list::{ListEntry, ListItem};
use cabas_domain::overlay::Explicit;
use cabas_domain::recipe::{
    Component, IngredientUsage, Segment, Step, SubRecipeAmount, SubRecipeUsage,
};
use cabas_domain::{
    Device, Event, Ingredient, IngredientId, ListEntryId, Recipe, RecipeId, UsageId, User, UserId,
};
use loro::{LoroMap, LoroMovableList, LoroValue, ValueOrContainer};

use crate::codec::{self, text, value_list, value_map};
use crate::error::{Result, StoreError, crdt};
use crate::schema;

// --- write primitives -------------------------------------------------------

/// Sets a register, but only if it differs from what is already there.
pub(crate) fn set(map: &LoroMap, key: &str, value: LoroValue) -> Result<()> {
    if let Some(ValueOrContainer::Value(current)) = map.get(key)
        && current == value
    {
        return Ok(());
    }
    map.insert(key, value).map_err(crdt)
}

/// Sets an optional register. `None` is written as `Null` rather than
/// deleting the key: a delete racing a concurrent set is a conflict between
/// two different kinds of operation, whereas two register writes are just
/// last-writer-wins — which is the behaviour a cleared field should have.
pub(crate) fn set_optional(map: &LoroMap, key: &str, value: Option<LoroValue>) -> Result<()> {
    set(map, key, value.unwrap_or(LoroValue::Null))
}

/// Replaces a sequence, but only if its contents differ.
fn set_sequence(list: &LoroMovableList, items: Vec<LoroValue>) -> Result<()> {
    if let Some(current) = list.get_deep_value().as_list()
        && current.as_ref() == items.as_slice()
    {
        return Ok(());
    }
    list.clear().map_err(crdt)?;
    for item in items {
        list.push(item).map_err(crdt)?;
    }
    Ok(())
}

/// A recipe's components or steps, created on first write.
///
/// Mergeable for the same reason entity containers are: two devices adding a
/// step to the same new recipe offline must end up appending to one list, not
/// to two lists that then compete for the key.
fn sequence(map: &LoroMap, key: &str) -> Result<LoroMovableList> {
    map.ensure_mergeable_movable_list(key).map_err(crdt)
}

// --- ingredients ------------------------------------------------------------

pub(crate) fn write_ingredient(entry: &LoroMap, ing: &Ingredient) -> Result<()> {
    set(entry, schema::ingredient::NAME, text(&ing.name))?;
    set(
        entry,
        schema::ingredient::ALIASES,
        value_list(ing.aliases.iter().map(|a| text(a))),
    )?;
    set(
        entry,
        schema::ingredient::AISLE,
        text(codec::aisle_tag(ing.aisle)),
    )?;
    set(entry, schema::ingredient::STAPLE, ing.staple.into())?;
    set_optional(
        entry,
        schema::ingredient::DENSITY,
        ing.density.map(codec::rational_value),
    )?;
    set_optional(
        entry,
        schema::ingredient::UNIT_WEIGHT,
        ing.unit_weight.map(codec::rational_value),
    )
}

pub(crate) fn read_ingredient(id: &str, value: &LoroValue) -> Result<Ingredient> {
    let path = format!("ingredients.{id}");
    let map = codec::map(value, &path)?;

    let aliases = match codec::optional(map, schema::ingredient::ALIASES) {
        Some(v) => codec::list(v, &path)?
            .iter()
            .map(|a| codec::string(a, &path))
            .collect::<Result<Vec<_>>>()?,
        None => Vec::new(),
    };

    Ok(Ingredient {
        id: IngredientId::from_raw(id),
        name: codec::string(codec::field(map, schema::ingredient::NAME, &path)?, &path)?,
        aliases,
        aisle: codec::aisle(codec::field(map, schema::ingredient::AISLE, &path)?, &path)?,
        staple: codec::boolean(codec::field(map, schema::ingredient::STAPLE, &path)?, &path)?,
        density: codec::optional(map, schema::ingredient::DENSITY)
            .map(|v| codec::rational(v, &path))
            .transpose()?,
        unit_weight: codec::optional(map, schema::ingredient::UNIT_WEIGHT)
            .map(|v| codec::rational(v, &path))
            .transpose()?,
    })
}

// --- recipes ----------------------------------------------------------------

pub(crate) fn write_recipe(entry: &LoroMap, recipe: &Recipe) -> Result<()> {
    set(entry, schema::recipe::NAME, text(&recipe.name))?;
    set(
        entry,
        schema::recipe::SERVINGS,
        LoroValue::I64(i64::from(recipe.servings.get())),
    )?;
    set_optional(
        entry,
        schema::recipe::YIELDS,
        recipe.yields.as_ref().map(codec::quantity_value),
    )?;

    let components = sequence(entry, schema::recipe::COMPONENTS)?;
    set_sequence(
        &components,
        recipe.components.iter().map(component_value).collect(),
    )?;

    let steps = sequence(entry, schema::recipe::STEPS)?;
    set_sequence(&steps, recipe.steps.iter().map(step_value).collect())
}

pub(crate) fn read_recipe(id: &str, value: &LoroValue) -> Result<Recipe> {
    let path = format!("recipes.{id}");
    let map = codec::map(value, &path)?;

    let components = match codec::optional(map, schema::recipe::COMPONENTS) {
        Some(v) => codec::list(v, &path)?
            .iter()
            .map(|c| read_component(c, &path))
            .collect::<Result<Vec<_>>>()?,
        None => Vec::new(),
    };
    let steps = match codec::optional(map, schema::recipe::STEPS) {
        Some(v) => codec::list(v, &path)?
            .iter()
            .map(|s| read_step(s, &path))
            .collect::<Result<Vec<_>>>()?,
        None => Vec::new(),
    };

    Ok(Recipe {
        id: RecipeId::from_raw(id),
        name: codec::string(codec::field(map, schema::recipe::NAME, &path)?, &path)?,
        servings: codec::servings(codec::field(map, schema::recipe::SERVINGS, &path)?, &path)?,
        yields: codec::optional(map, schema::recipe::YIELDS)
            .map(|v| codec::quantity(v, &path))
            .transpose()?,
        components,
        steps,
    })
}

fn component_value(component: &Component) -> LoroValue {
    use schema::component as k;
    match component {
        Component::Ingredient(usage) => value_map([
            (k::KIND, text(k::KIND_INGREDIENT)),
            (k::ID, text(usage.id.as_str())),
            (k::INGREDIENT, text(usage.ingredient.as_str())),
            (k::QUANTITY, codec::quantity_value(&usage.quantity)),
        ]),
        Component::SubRecipe(sub) => {
            let amount = match &sub.amount {
                SubRecipeAmount::Factor(f) => (k::FACTOR, codec::rational_value(*f)),
                SubRecipeAmount::OfYield(q) => (k::OF_YIELD, codec::quantity_value(q)),
            };
            value_map([
                (k::KIND, text(k::KIND_SUB_RECIPE)),
                (k::ID, text(sub.id.as_str())),
                (k::RECIPE, text(sub.recipe.as_str())),
                amount,
            ])
        }
    }
}

fn read_component(value: &LoroValue, path: &str) -> Result<Component> {
    use schema::component as k;
    let map = codec::map(value, path)?;
    let id = UsageId::from_raw(codec::string(codec::field(map, k::ID, path)?, path)?);
    let kind = codec::string(codec::field(map, k::KIND, path)?, path)?;

    match kind.as_str() {
        k::KIND_INGREDIENT => Ok(Component::Ingredient(IngredientUsage {
            id,
            ingredient: IngredientId::from_raw(codec::string(
                codec::field(map, k::INGREDIENT, path)?,
                path,
            )?),
            quantity: codec::quantity(codec::field(map, k::QUANTITY, path)?, path)?,
        })),
        k::KIND_SUB_RECIPE => {
            // Exactly one of the two amount forms is present (DECISIONS 0017).
            let amount = match (
                codec::optional(map, k::FACTOR),
                codec::optional(map, k::OF_YIELD),
            ) {
                (Some(f), None) => SubRecipeAmount::Factor(codec::rational(f, path)?),
                (None, Some(q)) => SubRecipeAmount::OfYield(codec::quantity(q, path)?),
                (Some(_), Some(_)) => {
                    return Err(StoreError::corrupt(
                        path,
                        "sub-recipe carries both a factor and a yield amount",
                    ));
                }
                (None, None) => {
                    return Err(StoreError::corrupt(path, "sub-recipe carries no amount"));
                }
            };
            Ok(Component::SubRecipe(SubRecipeUsage {
                id,
                recipe: RecipeId::from_raw(codec::string(
                    codec::field(map, k::RECIPE, path)?,
                    path,
                )?),
                amount,
            }))
        }
        other => Err(StoreError::corrupt(
            path,
            format!("unknown component kind {other:?}"),
        )),
    }
}

fn step_value(step: &Step) -> LoroValue {
    value_map([(
        schema::step::SEGMENTS,
        value_list(step.segments.iter().map(segment_value)),
    )])
}

fn read_step(value: &LoroValue, path: &str) -> Result<Step> {
    let map = codec::map(value, path)?;
    let segments = codec::list(codec::field(map, schema::step::SEGMENTS, path)?, path)?
        .iter()
        .map(|s| read_segment(s, path))
        .collect::<Result<Vec<_>>>()?;
    Ok(Step { segments })
}

fn segment_value(segment: &Segment) -> LoroValue {
    use schema::step as k;
    match segment {
        Segment::Text(t) => value_map([(k::KIND, text(k::KIND_TEXT)), (k::TEXT, text(t))]),
        Segment::Ingredient { usage, display } => value_map([
            (k::KIND, text(k::KIND_INGREDIENT)),
            (k::USAGE, text(usage.as_str())),
            (k::DISPLAY, text(codec::display_tag(*display))),
        ]),
    }
}

fn read_segment(value: &LoroValue, path: &str) -> Result<Segment> {
    use schema::step as k;
    let map = codec::map(value, path)?;
    let kind = codec::string(codec::field(map, k::KIND, path)?, path)?;
    match kind.as_str() {
        k::KIND_TEXT => Ok(Segment::Text(codec::string(
            codec::field(map, k::TEXT, path)?,
            path,
        )?)),
        k::KIND_INGREDIENT => Ok(Segment::Ingredient {
            usage: UsageId::from_raw(codec::string(codec::field(map, k::USAGE, path)?, path)?),
            display: codec::display(codec::field(map, k::DISPLAY, path)?, path)?,
        }),
        other => Err(StoreError::corrupt(
            path,
            format!("unknown segment kind {other:?}"),
        )),
    }
}

// --- the shopping list ------------------------------------------------------

pub(crate) fn list_entry_value(entry: &ListEntry) -> LoroValue {
    use schema::list_entry as k;
    let common = [
        (k::ID, text(entry.id.as_str())),
        (k::ADDED_BY, text(entry.added_by.as_str())),
        (k::ADDED_AT, codec::timestamp_value(entry.added_at)),
    ];
    let specific = match &entry.item {
        ListItem::Recipe { recipe, servings } => vec![
            (k::KIND, text(k::KIND_RECIPE)),
            (k::RECIPE, text(recipe.as_str())),
            (k::SERVINGS, LoroValue::I64(i64::from(servings.get()))),
        ],
        ListItem::Ingredient {
            ingredient,
            quantity,
        } => vec![
            (k::KIND, text(k::KIND_INGREDIENT)),
            (k::INGREDIENT, text(ingredient.as_str())),
            (k::QUANTITY, codec::quantity_value(quantity)),
        ],
    };
    value_map(common.into_iter().chain(specific))
}

pub(crate) fn read_list_entry(value: &LoroValue, path: &str) -> Result<ListEntry> {
    use schema::list_entry as k;
    let map = codec::map(value, path)?;
    let kind = codec::string(codec::field(map, k::KIND, path)?, path)?;

    let item = match kind.as_str() {
        k::KIND_RECIPE => ListItem::Recipe {
            recipe: RecipeId::from_raw(codec::string(codec::field(map, k::RECIPE, path)?, path)?),
            servings: codec::servings(codec::field(map, k::SERVINGS, path)?, path)?,
        },
        k::KIND_INGREDIENT => ListItem::Ingredient {
            ingredient: IngredientId::from_raw(codec::string(
                codec::field(map, k::INGREDIENT, path)?,
                path,
            )?),
            quantity: codec::quantity(codec::field(map, k::QUANTITY, path)?, path)?,
        },
        other => {
            return Err(StoreError::corrupt(
                path,
                format!("unknown list entry kind {other:?}"),
            ));
        }
    };

    Ok(ListEntry {
        id: ListEntryId::from_raw(codec::string(codec::field(map, k::ID, path)?, path)?),
        item,
        added_by: UserId::from_raw(codec::string(codec::field(map, k::ADDED_BY, path)?, path)?),
        added_at: codec::timestamp(codec::field(map, k::ADDED_AT, path)?, path)?,
    })
}

// --- the check overlay ------------------------------------------------------

pub(crate) fn explicit_value(explicit: &Explicit) -> LoroValue {
    use schema::overlay as k;
    match explicit {
        Explicit::Checked { by, at } => value_map([
            (k::STATE, text(k::STATE_CHECKED)),
            (k::BY, text(by.as_str())),
            (k::AT, codec::timestamp_value(*at)),
        ]),
        // Persisted, and not merely absent: an absent entry falls back to the
        // derived default, which for a staple is `AutoChecked` — so dropping
        // this would silently re-check the staple the user just unchecked
        // (Rule 3, DECISIONS 0023).
        Explicit::Unchecked => value_map([(k::STATE, text(k::STATE_UNCHECKED))]),
    }
}

pub(crate) fn read_explicit(value: &LoroValue, path: &str) -> Result<Explicit> {
    use schema::overlay as k;
    let map = codec::map(value, path)?;
    let state = codec::string(codec::field(map, k::STATE, path)?, path)?;
    match state.as_str() {
        k::STATE_CHECKED => Ok(Explicit::Checked {
            by: UserId::from_raw(codec::string(codec::field(map, k::BY, path)?, path)?),
            at: codec::timestamp(codec::field(map, k::AT, path)?, path)?,
        }),
        k::STATE_UNCHECKED => Ok(Explicit::Unchecked),
        other => Err(StoreError::corrupt(
            path,
            format!("unknown overlay state {other:?}"),
        )),
    }
}

// --- users and devices ------------------------------------------------------

pub(crate) fn write_user(entry: &LoroMap, user: &User) -> Result<()> {
    set(entry, schema::user::NAME, text(&user.name))
}

pub(crate) fn read_user(id: &str, value: &LoroValue) -> Result<User> {
    let path = format!("users.{id}");
    let map = codec::map(value, &path)?;
    Ok(User {
        id: UserId::from_raw(id),
        name: codec::string(codec::field(map, schema::user::NAME, &path)?, &path)?,
    })
}

pub(crate) fn write_device(entry: &LoroMap, device: &Device) -> Result<()> {
    set(entry, schema::device::NAME, text(&device.name))?;
    set(entry, schema::device::OWNER, text(device.owner.as_str()))?;
    set(
        entry,
        schema::device::PAIRED_AT,
        codec::timestamp_value(device.paired_at),
    )
}

pub(crate) fn read_device(id: &str, value: &LoroValue) -> Result<Device> {
    let path = format!("devices.{id}");
    let map = codec::map(value, &path)?;
    Ok(Device {
        id: cabas_domain::DeviceId::from_raw(id),
        owner: UserId::from_raw(codec::string(
            codec::field(map, schema::device::OWNER, &path)?,
            &path,
        )?),
        name: codec::string(codec::field(map, schema::device::NAME, &path)?, &path)?,
        paired_at: codec::timestamp(codec::field(map, schema::device::PAIRED_AT, &path)?, &path)?,
    })
}

// --- the event log ----------------------------------------------------------

pub(crate) fn event_value(event: &Event) -> LoroValue {
    use schema::event as k;
    let (kind, id) = match &event.subject {
        Subject::Recipe(r) => (k::SUBJECT_RECIPE, r.as_str()),
        Subject::Ingredient(i) => (k::SUBJECT_INGREDIENT, i.as_str()),
        Subject::ListEntry(e) => (k::SUBJECT_LIST_ENTRY, e.as_str()),
    };
    let action = match event.action {
        Action::Edited => k::ACTION_EDITED,
        Action::Deleted => k::ACTION_DELETED,
    };
    value_map([
        (k::AT, codec::timestamp_value(event.at)),
        (k::BY, text(event.by.as_str())),
        (k::ACTION, text(action)),
        (k::SUBJECT_KIND, text(kind)),
        (k::SUBJECT_ID, text(id)),
        (k::LABEL, text(&event.label)),
    ])
}

pub(crate) fn read_event(value: &LoroValue, path: &str) -> Result<Event> {
    use schema::event as k;
    let map = codec::map(value, path)?;

    let action = match codec::string(codec::field(map, k::ACTION, path)?, path)?.as_str() {
        k::ACTION_EDITED => Action::Edited,
        k::ACTION_DELETED => Action::Deleted,
        other => {
            return Err(StoreError::corrupt(
                path,
                format!("unknown action {other:?}"),
            ));
        }
    };

    let id = codec::string(codec::field(map, k::SUBJECT_ID, path)?, path)?;
    let subject = match codec::string(codec::field(map, k::SUBJECT_KIND, path)?, path)?.as_str() {
        k::SUBJECT_RECIPE => Subject::Recipe(RecipeId::from_raw(id)),
        k::SUBJECT_INGREDIENT => Subject::Ingredient(IngredientId::from_raw(id)),
        k::SUBJECT_LIST_ENTRY => Subject::ListEntry(ListEntryId::from_raw(id)),
        other => {
            return Err(StoreError::corrupt(
                path,
                format!("unknown subject kind {other:?}"),
            ));
        }
    };

    Ok(Event {
        at: codec::timestamp(codec::field(map, k::AT, path)?, path)?,
        by: UserId::from_raw(codec::string(codec::field(map, k::BY, path)?, path)?),
        action,
        subject,
        label: codec::string(codec::field(map, k::LABEL, path)?, path)?,
    })
}
