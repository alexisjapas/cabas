//! Intents. Everything the frontend is able to ask for, and nothing else.
//!
//! # Coarse on purpose (Rule 9)
//!
//! One command is one thing a person did — "put this recipe on the list",
//! "I have this in the trolley". Not one field write. Every call crosses an
//! FFI or IPC boundary, every generated type is maintenance, and a chatty
//! surface is how business logic ends up on the far side of it: three fine
//! commands in a row are three chances for the UI to decide what happens
//! between them.
//!
//! # Ids are strings here
//!
//! The domain's id types are opaque wrappers minted by `app`; on the wire
//! they are the strings the frontend received in the last view and hands
//! back. A string that names nothing is [`crate::AppError::NotFound`], never
//! a panic — the other device may have deleted it a second ago.
//!
//! # Inputs carry text, not numbers
//!
//! A quantity arrives as the characters typed into the field ("1,5"), and
//! `app` parses it into an exact rational (Rule 4). The UI does not
//! pre-parse, because a float that reaches the domain has already lost the
//! precision the whole design is built to keep.

use serde::{Deserialize, Serialize};

use crate::tags::{AisleTag, RefDisplayTag, UnitTag};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub enum Command {
    /// Creates the ingredient when `id` is absent, updates it otherwise.
    SaveIngredient {
        ingredient: IngredientInput,
    },

    /// Removes an ingredient from the library.
    ///
    /// Recipes that still use it are **not** rewritten: referential
    /// integrity is not enforceable under a CRDT, so the dangling reference
    /// is reported by the domain and rendered as a warning (DECISIONS 0022).
    DeleteIngredient {
        ingredient: String,
    },

    SaveRecipe {
        recipe: RecipeInput,
    },
    DeleteRecipe {
        recipe: String,
    },

    AddRecipeToList {
        recipe: String,
        servings: u32,
    },

    /// Adds a bare ingredient. This also purges the ingredient's overlay
    /// entry — putting something on the list by hand means "I need this",
    /// so it must come back into view even if it is a staple or was checked
    /// off earlier in the same trip (Rule 3).
    AddIngredientToList {
        ingredient: String,
        quantity: QuantityInput,
    },

    /// "We are six tonight." Rescales one list entry in place.
    SetEntryServings {
        entry: String,
        servings: u32,
    },

    RemoveListEntry {
        entry: String,
    },

    /// One tap on a cart line. Toggling reads the *derived* state, so
    /// unchecking a staple that was never explicitly checked stores an
    /// explicit `Unchecked` — without which the next derivation would
    /// silently re-check it (Rule 3).
    ToggleCartItem {
        ingredient: String,
    },

    /// Ends the trip: completed entries leave the list and their overlay
    /// entries are pruned — selectively, so a partly bought entry keeps its
    /// progress (DECISIONS 0028).
    FinishShopping,

    /// Opens a recipe, optionally at a different number of servings.
    /// Absent servings means the recipe as written.
    ///
    /// Which recipe is open is device-local state and is never synced: two
    /// people reading different recipes is not a conflict.
    OpenRecipe {
        recipe: String,
        servings: Option<u32>,
    },
    CloseRecipe,

    /// Renames the person this device belongs to. Attribution is declarative
    /// (Rule 7), so this changes a label and nothing else.
    RenameUser {
        name: String,
    },
}

/// A quantity as typed: `{ amount: "1,5", unit: "kg" }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct QuantityInput {
    pub amount: String,
    pub unit: UnitTag,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct IngredientInput {
    /// Absent on creation. Present — and unchanged — on every edit.
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub aisle: AisleTag,
    #[serde(default)]
    pub staple: bool,
    /// Grams per millilitre, as text. What makes mass ↔ volume possible for
    /// this ingredient; absent, the cart keeps the two on separate lines
    /// rather than guessing (Rule 5).
    #[serde(default)]
    pub density: Option<String>,
    /// Grams per piece, as text. Same bargain, for count ↔ mass.
    #[serde(default)]
    pub unit_weight: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct RecipeInput {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    /// How many people the quantities as written serve. Must be at least one.
    pub servings: u32,
    /// What it produces — "makes 500 g". Required before another recipe can
    /// take an amount *of* it (DECISIONS 0017).
    #[serde(default)]
    pub yields: Option<QuantityInput>,
    #[serde(default)]
    pub components: Vec<ComponentInput>,
    #[serde(default)]
    pub steps: Vec<StepInput>,
}

/// One line of a recipe's ingredient list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub enum ComponentInput {
    Ingredient {
        /// The *usage* id — absent on a new line. Steps reference this, not
        /// the ingredient, so that a recipe using flour twice can render the
        /// right amount in each step (DECISIONS 0022).
        #[serde(default)]
        id: Option<String>,
        ingredient: String,
        quantity: QuantityInput,
    },
    SubRecipe {
        #[serde(default)]
        id: Option<String>,
        recipe: String,
        amount: SubRecipeAmountInput,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub enum SubRecipeAmountInput {
    /// A multiple of the sub-recipe as written: "half of it".
    Factor { factor: String },
    /// An absolute amount of what it yields: "200 g of that pastry".
    OfYield { quantity: QuantityInput },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct StepInput {
    pub segments: Vec<SegmentInput>,
}

/// A step is a run of segments rather than a string with markers in it, so
/// that editing the prose cannot break a reference (DECISIONS 0022).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub enum SegmentInput {
    Text {
        text: String,
    },
    Ingredient {
        usage: String,
        display: RefDisplayTag,
    },
}
