//! View-models: everything on screen, computed here and pushed whole.
//!
//! # One state, not a getter surface (Rule 9)
//!
//! There is a single type — [`StateView`] — and every state change hands back
//! a complete new one. The frontend never asks a follow-up question, so it
//! never has a chance to answer one itself: no quantity is computed there, no
//! unit resolved, no check state decided.
//!
//! Sending the whole state on every change is affordable because the whole
//! library is 154 kB of CRDT and rather less as view-models (M2's
//! measurement), and it buys the property that matters — the screen cannot
//! show two things that disagree, because it only ever received one thing.
//!
//! # What is a string here, and what is not
//!
//! Amounts are **rendered text** ("1 1/2"), because turning an exact rational
//! into something readable is arithmetic and arithmetic stays in Rust.
//! Everything nameable is a **tag** (`"kg"`, `"produce"`, `"to_buy"`),
//! because the label a person reads is French and the frontend is where the
//! French lives — the app writes no user-facing prose, which is also what
//! keeps a later translation a UI change rather than a core change.

use serde::{Deserialize, Serialize};

use crate::command::RecipeInput;
use crate::number;
use crate::tags::{ActionTag, AisleTag, CheckStateTag, RefDisplayTag, SubjectTag, UnitTag};
use cabas_domain::Quantity;

/// Everything on screen, after the last thing that happened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct StateView {
    /// Counts state pushes, so a frontend can tell a new state from a
    /// re-render without comparing the whole tree. Resets when the app is
    /// reopened; it orders nothing across devices.
    pub revision: u64,
    /// Who this device says it is — the name attribution will use.
    pub me: UserView,
    /// Everyone in the family and the devices they carry, in document order.
    /// Sorting is the screen's business, like every other list here.
    pub people: Vec<PersonView>,
    /// What has been edited and deleted, newest first and capped
    /// (DECISIONS 0024).
    pub events: Vec<EventView>,
    pub cart: CartView,
    pub list: Vec<ListEntryView>,
    pub recipes: Vec<RecipeSummaryView>,
    pub ingredients: Vec<IngredientView>,
    /// The open recipe, if any. Device-local, never synced.
    pub focus: Option<FocusView>,
    /// What could not be made sense of, usually because another device
    /// deleted something this one still refers to. Reported, never fatal
    /// (DECISIONS 0022).
    pub problems: Vec<ProblemView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct UserView {
    pub id: String,
    pub name: String,
}

/// One person in the family, and the devices they carry.
///
/// **Names, not permissions** (Rule 7, DECISIONS 0024). One shared key
/// decrypts the whole document, so every one of these says who most likely
/// did something — not who was allowed to. The screen that lists them is the
/// one place the UI is required to say so out loud, because it is the screen
/// that looks most like an access control panel and is not one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct PersonView {
    pub id: String,
    pub name: String,
    /// The person this device belongs to.
    pub is_me: bool,
    pub devices: Vec<DeviceView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct DeviceView {
    pub id: String,
    pub name: String,
    /// The device this state was rendered on.
    pub is_this_one: bool,
    /// When it joined the family, in milliseconds since the epoch.
    ///
    /// A number rather than text: a millisecond count is nowhere near where a
    /// double stops being exact (DECISIONS 0046), and the words around a date
    /// belong to the frontend (0035) — "il y a deux mois" is a sentence, and
    /// this crate writes none.
    pub paired_at: i64,
}

/// One line of the event log: what the data itself cannot remember.
///
/// Deletions and edits leave no field behind to hold "and Alexis did this",
/// so they are recorded (DECISIONS 0024). A courtesy, capped, and never an
/// audit trail — with one shared key any device can write any of these under
/// any name (Rule 7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct EventView {
    /// Milliseconds since the epoch, from *the clock of the device that did
    /// it*. Two devices' clocks disagree, so this is worth showing as "two
    /// days ago" and not worth sorting a timeline by — which is why the log
    /// arrives in merge order and stays that way.
    pub at: i64,
    /// The name of whoever did it, or `None` if that person is no longer in
    /// the document. The frontend has a word for that case; this crate does
    /// not write one (DECISIONS 0035).
    pub by: Option<String>,
    /// Whether that was this device's own person.
    pub by_me: bool,
    pub action: ActionTag,
    pub subject: SubjectTag,
    /// What the thing was called **when it happened** — copied at the time,
    /// because the entries that matter most are the ones whose subject no
    /// longer exists to be looked up.
    pub label: String,
}

/// An amount, ready to display.
///
/// `amount` is text and never a number: the value behind it is an exact
/// rational, and any numeric type the frontend could hold it in would be a
/// float (Rule 4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct QuantityView {
    pub amount: String,
    pub unit: UnitTag,
    /// The rendering is rounded — 28.35 g where the exact value is
    /// 28.349523125. The UI may show a "≈"; the cart still adds up the exact
    /// value underneath.
    pub approximate: bool,
}

impl QuantityView {
    pub(crate) fn of(quantity: &Quantity) -> Self {
        let rendered = number::render(quantity.value);
        Self {
            amount: rendered.text,
            unit: quantity.unit.into(),
            approximate: rendered.approximate,
        }
    }
}

/// The shop screen. Three sections, because "bought" and "already at home"
/// are not the same statement and merging them makes unchecking a staple
/// hard to discover (DECISIONS 0023).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct CartView {
    pub to_buy: Vec<CartLineView>,
    pub bought: Vec<CartLineView>,
    pub at_home: Vec<CartLineView>,
    /// Lines still to pick up, and lines in total — the "12 / 20" a person
    /// glances at while walking.
    pub remaining: usize,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct CartLineView {
    pub ingredient: String,
    pub name: String,
    pub aisle: AisleTag,
    pub staple: bool,
    /// Usually one. More than one when the contributions could not be merged
    /// — "300 g + 2 tbsp" is the honest rendering when no density is known
    /// (Rule 5).
    pub amounts: Vec<QuantityView>,
    pub state: CheckStateTag,
    /// Who ticked it, when it is known. A name, not an id: attribution is a
    /// label (Rule 7).
    pub checked_by: Option<String>,
    pub checked_at: Option<i64>,
    /// Which list entries asked for this. One tap advances every one of them.
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct ListEntryView {
    pub id: String,
    pub item: ListItemView,
    pub progress: ProgressView,
    pub added_by: Option<String>,
    /// Milliseconds since the Unix epoch. The frontend renders "il y a 2 h" —
    /// relative time is presentation, and the clock it is relative to is the
    /// reader's.
    pub added_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub enum ListItemView {
    Recipe {
        recipe: String,
        name: String,
        /// What this entry is cooked for.
        servings: u32,
        /// What the recipe itself is written for, so the UI can show that the
        /// two differ.
        written_for: u32,
    },
    Ingredient {
        ingredient: String,
        name: String,
        quantity: QuantityView,
    },
}

/// How far through an entry the shopping has got — "5 / 7" against a recipe
/// until every ingredient it contributed is settled (DECISIONS 0020).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct ProgressView {
    pub settled: usize,
    pub total: usize,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct RecipeSummaryView {
    pub id: String,
    pub name: String,
    pub servings: u32,
    pub yields: Option<QuantityView>,
    pub ingredients: usize,
    pub sub_recipes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct IngredientView {
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub aisle: AisleTag,
    pub staple: bool,
    pub density: Option<String>,
    pub unit_weight: Option<String>,
}

/// The open recipe: what to read, and what to edit.
///
/// Both, because they are not the same shape. `recipe` is rendered at the
/// current servings — quantities scaled, step references resolved. `edit` is
/// the recipe as written, in exactly the form [`crate::Command::SaveRecipe`]
/// takes, so the editor is a form the frontend fills and hands back rather
/// than a structure it has to rebuild from rendered text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct FocusView {
    pub recipe: RecipeView,
    pub edit: RecipeInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct RecipeView {
    pub id: String,
    pub name: String,
    /// What the recipe is written for, and what it is being read at. Every
    /// quantity below is already scaled by the ratio of the two.
    pub written_for: u32,
    pub servings: u32,
    pub yields: Option<QuantityView>,
    pub components: Vec<ComponentView>,
    pub steps: Vec<StepView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub enum ComponentView {
    Ingredient {
        usage: String,
        ingredient: String,
        /// The library name, or `None` if the ingredient is gone — the line
        /// still renders, with a warning (DECISIONS 0022).
        name: Option<String>,
        quantity: QuantityView,
    },
    SubRecipe {
        usage: String,
        recipe: String,
        name: Option<String>,
        amount: SubRecipeAmountView,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub enum SubRecipeAmountView {
    /// "×1 1/2" — a multiple of the sub-recipe as written.
    Factor {
        factor: String,
    },
    OfYield {
        quantity: QuantityView,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct StepView {
    pub segments: Vec<SegmentView>,
}

/// A step, resolved. The app has already applied the reference's display
/// rule, so `name` and `quantity` are present exactly when they should be
/// shown; the tag travels along so a first mention can be styled differently
/// from a later one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub enum SegmentView {
    Text {
        text: String,
    },
    Ingredient {
        usage: String,
        name: Option<String>,
        quantity: Option<QuantityView>,
        display: RefDisplayTag,
    },
    /// A reference to a usage that no longer exists. Rendered as a warning in
    /// place, never a panic and never a reason to refuse the deletion that
    /// caused it (DECISIONS 0022).
    Missing {
        usage: String,
    },
}

/// Something the data says that cannot be honoured.
///
/// Always the shape of a concurrent edit: one device deleted a recipe the
/// other still had on its list, or a sub-recipe chain became a cycle when two
/// halves of it merged. The entry stays visible and the rest of the cart
/// still derives — an empty screen would be the worst possible response to
/// one broken row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub struct ProblemView {
    /// The list entry that could not be used, when the problem has one.
    pub entry: Option<String>,
    /// What is missing or broken — a recipe or an ingredient id — so the UI
    /// can mark the row it belongs to rather than only the list.
    pub subject: Option<String>,
    pub kind: ProblemKind,
    /// The domain's own message. English, and a diagnostic rather than UI
    /// copy: the frontend shows its own sentence per `kind` and keeps this
    /// for the details view.
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(export))]
pub enum ProblemKind {
    /// The entry names a recipe the library no longer holds.
    MissingRecipe,
    /// A recipe asks for an ingredient the library no longer holds.
    MissingIngredient,
    /// Sub-recipes reference each other in a loop, or nest too deep.
    BrokenGraph,
    /// A sub-recipe is taken as an amount of a yield it does not declare, or
    /// declares in another dimension (DECISIONS 0017).
    BrokenYield,
}
