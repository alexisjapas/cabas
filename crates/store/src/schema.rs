//! The document layout, in one file.
//!
//! Every container and every key the document may hold is named here, so the
//! persisted shape can be read without reading the mapping code. That matters
//! more than usual: the schema is a **compatibility surface**. A phone left
//! in a pocket for three weeks must still converge with the relay, so a
//! change to anything below is a breaking change and bumps the major version
//! (Rule 15).
//!
//! # Shape
//!
//! ```text
//! meta        map     { schema: i64 }
//! ingredients map     IngredientId -> map   (container per ingredient)
//! recipes     map     RecipeId     -> map   (container per recipe)
//! list        movable list of maps          (the single shopping list)
//! overlay     map     IngredientId -> value map  (explicit actions only)
//! users       map     UserId       -> map
//! devices     map     DeviceId     -> map
//! events      list of value maps            (append-only, capped)
//! ```
//!
//! # Why containers in some places and plain value maps in others
//!
//! An entity that two people can edit *field by field* — an ingredient's
//! aisle while the other fixes its name — is a **container**, so the merge is
//! per field. A structure that is only ever rewritten as a unit is a **plain
//! value map**: a recipe's ingredient lines and steps are edited one line at
//! a time, and DECISIONS 0022 already ruled out two people co-editing a
//! single step. Making those containers too would buy a merge nobody
//! performs, at the cost of a schema you cannot read.
//!
//! The overlay is deliberately the simplest case: one register per
//! ingredient, last write wins. Concurrent check and uncheck of the same item
//! is the convergence case that matters, and "the most recent tap wins" is
//! both what converges and what a person expects.

/// Bumped whenever the layout below changes in a way an older build cannot
/// read. Written into `meta` on creation and checked on load.
pub const SCHEMA_VERSION: i64 = 1;

/// Root containers.
pub mod root {
    pub const META: &str = "meta";
    pub const INGREDIENTS: &str = "ingredients";
    pub const RECIPES: &str = "recipes";
    pub const LIST: &str = "list";
    pub const OVERLAY: &str = "overlay";
    pub const USERS: &str = "users";
    pub const DEVICES: &str = "devices";
    pub const EVENTS: &str = "events";
}

pub mod meta {
    pub const SCHEMA: &str = "schema";
}

/// A quantity, wherever one appears: `{ value: "3/2", unit: "kg" }`.
///
/// `value` is a **string**, never a number. `LoroValue` offers `f64` and
/// `i64` and nothing exact in between, and a quantity is a `Ratio<i128>` —
/// storing it as a double would put a rounding step between two devices that
/// are supposed to agree, and storing numerator and denominator as `i64`
/// would overflow on the imperial factors (Rule 4).
pub mod quantity {
    pub const VALUE: &str = "value";
    pub const UNIT: &str = "unit";
}

pub mod ingredient {
    pub const NAME: &str = "name";
    pub const ALIASES: &str = "aliases";
    pub const AISLE: &str = "aisle";
    pub const STAPLE: &str = "staple";
    pub const DENSITY: &str = "density";
    pub const UNIT_WEIGHT: &str = "unit_weight";
}

pub mod recipe {
    pub const NAME: &str = "name";
    pub const SERVINGS: &str = "servings";
    pub const YIELDS: &str = "yields";
    pub const COMPONENTS: &str = "components";
    pub const STEPS: &str = "steps";
}

/// One line of a recipe: an ingredient usage or a sub-recipe reference,
/// distinguished by `kind`.
pub mod component {
    pub const KIND: &str = "kind";
    pub const KIND_INGREDIENT: &str = "ingredient";
    pub const KIND_SUB_RECIPE: &str = "sub_recipe";

    pub const ID: &str = "id";
    pub const INGREDIENT: &str = "ingredient";
    pub const QUANTITY: &str = "quantity";
    pub const RECIPE: &str = "recipe";
    /// `factor` and `of_yield` are mutually exclusive: a sub-recipe is taken
    /// either as a multiple of itself or as an amount of what it yields
    /// (DECISIONS 0017).
    pub const FACTOR: &str = "factor";
    pub const OF_YIELD: &str = "of_yield";
}

/// A step is `{ segments: [ … ] }`; a segment is either text or a reference
/// to one of the recipe's own usages (DECISIONS 0022).
pub mod step {
    pub const SEGMENTS: &str = "segments";

    pub const KIND: &str = "kind";
    pub const KIND_TEXT: &str = "text";
    pub const KIND_INGREDIENT: &str = "ingredient";

    pub const TEXT: &str = "text";
    pub const USAGE: &str = "usage";
    pub const DISPLAY: &str = "display";
}

pub mod list_entry {
    pub const ID: &str = "id";
    pub const KIND: &str = "kind";
    pub const KIND_RECIPE: &str = "recipe";
    pub const KIND_INGREDIENT: &str = "ingredient";

    pub const RECIPE: &str = "recipe";
    pub const SERVINGS: &str = "servings";
    pub const INGREDIENT: &str = "ingredient";
    pub const QUANTITY: &str = "quantity";
    pub const ADDED_BY: &str = "added_by";
    pub const ADDED_AT: &str = "added_at";
}

/// An explicit action, and only ever an explicit one (Rule 3). An absent key
/// is not "unchecked" — it means the line falls back to its derived default,
/// which is why [`STATE_UNCHECKED`] has to be storable at all.
pub mod overlay {
    pub const STATE: &str = "state";
    pub const STATE_CHECKED: &str = "checked";
    pub const STATE_UNCHECKED: &str = "unchecked";

    pub const BY: &str = "by";
    pub const AT: &str = "at";
}

pub mod user {
    pub const NAME: &str = "name";
}

pub mod device {
    pub const OWNER: &str = "owner";
    pub const NAME: &str = "name";
    pub const PAIRED_AT: &str = "paired_at";
}

pub mod event {
    pub const AT: &str = "at";
    pub const BY: &str = "by";
    pub const ACTION: &str = "action";
    pub const ACTION_EDITED: &str = "edited";
    pub const ACTION_DELETED: &str = "deleted";

    pub const SUBJECT_KIND: &str = "subject_kind";
    pub const SUBJECT_RECIPE: &str = "recipe";
    pub const SUBJECT_INGREDIENT: &str = "ingredient";
    pub const SUBJECT_LIST_ENTRY: &str = "list_entry";

    pub const SUBJECT_ID: &str = "subject_id";
    pub const LABEL: &str = "label";
}
