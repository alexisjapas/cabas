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

use crate::tags::{AisleTag, KeepingTag, RefDisplayTag, UnitTag};

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

    /// Creates the shop when `id` is absent, renames it otherwise
    /// (DECISIONS 0071).
    ///
    /// Sent by the field where a shop is typed onto an ingredient: a name
    /// that matches nothing offers to become one, and the ingredient is saved
    /// referring to the id this minted. Two commands rather than a shop name
    /// travelling on [`IngredientInput`], because creating a shop is a thing
    /// a person did and not a side effect of saving something else.
    SaveShop {
        shop: ShopInput,
    },

    /// Forgets a shop.
    ///
    /// Ingredients still naming it are **not** rewritten, like every other
    /// delete here (DECISIONS 0022): the dangling id matches no shop, so it
    /// filters nothing and the ingredient keeps its other shops.
    DeleteShop {
        shop: String,
    },

    SaveRecipe {
        recipe: RecipeInput,
    },
    DeleteRecipe {
        recipe: String,
    },

    AddRecipeToList {
        recipe: String,
        /// Absent means "as the recipe is written". That is how a swiped row
        /// asks (DECISIONS 0067): a gesture has nowhere to put a number, and
        /// what to put there instead is the recipe's own business rather than
        /// something the frontend should read off a view and hand back
        /// (Rule 9). Cooking it for a different number tonight is what the
        /// list's own "− 4 pers. +" is for.
        servings: Option<u32>,
    },

    /// Adds a bare ingredient. This also purges the ingredient's overlay
    /// entry — putting something on the list by hand means "I need this",
    /// so it must come back into view even if it is a staple or was checked
    /// off earlier in the same trip (Rule 3).
    AddIngredientToList {
        ingredient: String,
        /// Absent means "as much of it as one usually buys": the
        /// ingredient's own default quantity, or one piece if it has none
        /// (DECISIONS 0066). That is how a swiped row asks — a gesture has
        /// nowhere to put an amount, and the rule for choosing one is
        /// business logic rather than something the frontend should hold
        /// (Rule 9).
        #[serde(default)]
        quantity: Option<QuantityInput>,
    },

    /// "We are six tonight." Rescales one list entry in place.
    SetEntryServings {
        entry: String,
        servings: u32,
    },

    /// Sets what a bare ingredient on the list asks for, exactly
    /// (DECISIONS 0072). What the long press on a shelf row opens, and the
    /// only way an amount and a unit are both chosen at once.
    ///
    /// Refused on a recipe entry, which is measured in people and has
    /// [`Command::SetEntryServings`] for that.
    SetEntryQuantity {
        entry: String,
        quantity: QuantityInput,
    },

    /// One more of this, or one less — the gesture continued (DECISIONS
    /// 0072).
    ///
    /// `steps` is a count of notches and never an amount: what a notch is
    /// worth is the core's rule and differs by what is on the line — the
    /// ingredient's usual shopping quantity, or one whole recipe as written
    /// (Rule 9). Nudging the last one down takes the entry off the list,
    /// which is why this is not [`Command::SetEntryQuantity`] with the
    /// arithmetic done in the frontend.
    NudgeListEntry {
        entry: String,
        steps: i32,
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

    /// Says which member of the group is carrying this device — one of the
    /// people already in the document (DECISIONS 0068).
    ///
    /// Used twice: by a phone that has just typed the twelve words and been
    /// shown the roster, and by one being handed to somebody else. The two
    /// are the same act, so they are the same command. What changes is a
    /// label on future edits and the owner of this device's record; nothing
    /// already written is re-attributed, because it was not written by this
    /// person (Rule 7).
    ChooseUser {
        user: String,
    },

    /// Adds a person to the group and says this device is them.
    ///
    /// The way out of a roster that does not have you on it — including the
    /// empty roster of a group that was created a moment ago.
    CreateUser {
        name: String,
    },

    /// What this device is called in the roster — "l'iPhone d'Alexis".
    ///
    /// Sent before [`Command::ChooseUser`] or [`Command::CreateUser`] on a
    /// first launch, which is what lets the device record be written once,
    /// with its name already on it: the record cannot exist before there is
    /// an owner, and the name is answered after the owner is (DECISIONS
    /// 0068).
    NameDevice {
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

/// A shop, as the field that creates one sends it: a name and nothing else
/// (DECISIONS 0071).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct ShopInput {
    /// Absent on creation. Minted by the frontend in practice, for the reason
    /// [`IngredientInput::id`] is: the field that creates a shop has to
    /// *select* it the moment it exists, and a command hands back a whole
    /// state rather than an id (DECISIONS 0056).
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
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
    /// The ids of the shops this can be bought at, in the order they are to
    /// be read (DECISIONS 0071). Ids and never names: a shop is created by
    /// [`Command::SaveShop`] before it is referred to here, so that "Biocoop"
    /// typed twice is one shop and not two.
    ///
    /// Empty means "nobody has said", which the cart reads as every shop.
    #[serde(default)]
    pub shops: Vec<String>,
    /// Fridge, freezer or neither (DECISIONS 0070). Defaulted rather than
    /// optional: every ingredient is kept somewhere, and "a cupboard" is the
    /// honest answer for one nobody has thought about.
    #[serde(default)]
    pub keeping: KeepingTag,
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
    /// How much of this one buys when nobody says how much — a kilo of
    /// flour, six eggs (DECISIONS 0066). Absent means one piece, decided by
    /// [`cabas_domain::Ingredient::shopping_quantity`] rather than here.
    ///
    /// A shopping quantity and not a cooking one: no recipe reads it.
    #[serde(default)]
    pub default_quantity: Option<QuantityInput>,
    /// The id of a photo already stored by [`crate::photos::Photos::put`],
    /// or `null` to detach the one that is there.
    ///
    /// An id and never bytes: attaching a photo is this ordinary save, while
    /// the bytes went to a store of their own on a call that could be awaited
    /// (DECISIONS 0062). Sending it whole on every save is what makes an edit
    /// that does not mention the photo keep it.
    #[serde(default)]
    pub photo: Option<String>,
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
    /// The id of the dish's photo, or `null` to detach it. See
    /// [`IngredientInput::photo`].
    #[serde(default)]
    pub photo: Option<String>,
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
