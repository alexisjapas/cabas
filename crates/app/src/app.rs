//! The app: one replica, a command in, a whole state out.
//!
//! # Applying and saving are two steps, deliberately
//!
//! [`App::apply`] is **synchronous**. It mutates the in-memory replica and
//! hands back the new state; nothing in it awaits. Saving is
//! [`App::persist`], which is where the storage backend — and on the PWA, a
//! browser transaction — actually happens.
//!
//! Rule 6 says no user action waits on the network, and the same reasoning
//! applies one layer down: a tick in the shop should render at the speed of a
//! tap, not at the speed of IndexedDB on a five-year-old iPhone. Splitting
//! the two also removes the trap that would otherwise sit at the wasm
//! boundary, where an `&mut self` held across an `await` turns a second tap
//! into a panic.
//!
//! # Commands compose the domain and the store; neither thinks for the other
//!
//! A command asks `domain` for the rule and `store` to persist what came out.
//! `add_to_list` is the worked example: the domain's
//! [`cabas_domain::ShoppingList::add`] both appends the entry *and* purges the
//! ingredient's overlay entry (Rule 3), and this layer writes down both
//! effects. Neither of the other two crates re-derives the other's half —
//! that is how two implementations of one rule appear, and then drift.

use std::num::NonZeroU32;

use cabas_domain::event::{Action, Subject};
use cabas_domain::list::{ListEntry, ListItem};
use cabas_domain::recipe::{Component, IngredientUsage, Segment, Step, SubRecipeUsage};
use cabas_domain::{
    Device, Event, Explicit, Ingredient, IngredientId, ListEntryId, Quantity, Rational, Recipe,
    RecipeId, SubRecipeAmount, Timestamp, UsageId, User, finish_shopping,
};
use cabas_store::{Document, Storage};

use crate::command::{
    Command, ComponentInput, IngredientInput, QuantityInput, RecipeInput, SegmentInput,
    SubRecipeAmountInput,
};
use crate::error::{AppError, Result};
use crate::library::Library;
use crate::platform::{Identity, Platform};
use crate::project::{self, Focus};
use crate::view::StateView;
use crate::{id, number};

pub struct App<S: Storage, P: Platform> {
    storage: S,
    platform: P,
    identity: Identity,
    document: Document,
    /// Which recipe is open. Device-local: not a source, never synced.
    focus: Option<Focus>,
    /// Incremented on every state push, so a frontend can tell a new state
    /// from a re-render.
    revision: u64,
    /// The revision at which the *document* last changed, and the last one
    /// written to storage. Two counters rather than a dirty flag: opening a
    /// recipe pushes a new state without touching the document, and a phone
    /// should not write 154 kB because somebody looked at a recipe.
    changed_at: u64,
    saved_at: u64,
    /// Whether this replica was created from nothing at launch, rather than
    /// loaded from storage. A sync cursor that outlived the replica it
    /// belongs to points at frames this device no longer holds, so the host
    /// asks this before resuming one (see [`crate::sync`]).
    fresh: bool,
}

impl<S: Storage, P: Platform> App<S, P> {
    /// Loads the replica, or starts a fresh one, and makes sure this device
    /// and its owner exist in the document.
    ///
    /// A missing stored document is a first run, not a failure — `Storage`
    /// says so by returning `None`.
    pub async fn open(storage: S, platform: P, identity: Identity) -> Result<Self> {
        let stored = storage.load().await?;
        let fresh = stored.is_none();
        let document = match stored {
            Some(bytes) => Document::load(&bytes)?,
            None => Document::new(),
        };
        document.set_peer(identity.peer())?;

        let mut app = Self {
            storage,
            platform,
            identity,
            document,
            focus: None,
            revision: 0,
            changed_at: 0,
            saved_at: 0,
            fresh,
        };
        app.enrol()?;
        // A first run has just minted a user and a device; they are worth
        // nothing until they survive a restart.
        app.persist().await?;
        Ok(app)
    }

    /// Writes this device and its owner into the family document, if they are
    /// not already there.
    ///
    /// Only ever *adds*. The name in the document wins over the one the host
    /// passed in, because the other device may have renamed the person since
    /// this one last launched, and a launch is not a rename.
    fn enrol(&mut self) -> Result<()> {
        let user = self.identity.user_id();
        if !self.document.users()?.iter().any(|u| u.id == user) {
            self.document
                .put_user(&User::new(user.clone(), &self.identity.user_name))?;
            self.mark_changed();
        }

        let device = self.identity.device_id();
        if !self.document.devices()?.iter().any(|d| d.id == device) {
            self.document.put_device(&Device::new(
                device,
                user,
                &self.identity.device_name,
                self.platform.now(),
            ))?;
            self.mark_changed();
        }
        Ok(())
    }

    /// The current state, without changing anything.
    ///
    /// The one read the frontend performs: it calls this once to paint the
    /// first screen, and after that every state arrives as the return value
    /// of a command (Rule 9).
    pub fn state(&self) -> Result<StateView> {
        let library = Library::read(&self.document)?;
        Ok(self.view(&library))
    }

    /// Applies one command and returns the whole new state. Synchronous:
    /// nothing here waits on storage.
    pub fn apply(&mut self, command: Command) -> Result<StateView> {
        let library = Library::read(&self.document)?;
        match self.run(command, &library) {
            Ok(true) => {
                self.mark_changed();
                // The document moved under it; the library just read is stale.
                self.state()
            }
            Ok(false) => {
                self.revision += 1;
                Ok(self.view(&library))
            }
            // A command that failed partway may still have written something
            // — the writes are separate CRDT operations. Mark the replica
            // changed anyway, so whatever landed is saved rather than sitting
            // in memory until the process dies.
            Err(error) => {
                self.mark_changed();
                Err(error)
            }
        }
    }

    /// [`App::apply`] followed by [`App::persist`] — the convenient form for
    /// native hosts and tests. The PWA calls the two halves separately so
    /// that rendering never waits on a browser transaction.
    pub async fn dispatch(&mut self, command: Command) -> Result<StateView> {
        let state = self.apply(command)?;
        self.persist().await?;
        Ok(state)
    }

    /// Saves, if there is anything to save. Returns whether it wrote.
    pub async fn persist(&mut self) -> Result<bool> {
        let Some((snapshot, revision)) = self.pending_snapshot()? else {
            return Ok(false);
        };
        self.storage.save(&snapshot).await?;
        self.mark_saved(revision);
        Ok(true)
    }

    /// The bytes a save would write, and the revision they represent.
    ///
    /// Split out of [`App::persist`] for hosts that cannot hold a borrow
    /// across an await — which is every host with one thread and an event
    /// loop. Pair it with [`App::mark_saved`], and pass back the revision it
    /// gave you: anything applied while the write was in flight then stays
    /// pending instead of being quietly counted as saved.
    pub fn pending_snapshot(&mut self) -> Result<Option<(Vec<u8>, u64)>> {
        if self.changed_at == self.saved_at {
            return Ok(None);
        }
        Ok(Some((self.document.snapshot()?, self.changed_at)))
    }

    pub fn mark_saved(&mut self, revision: u64) {
        self.saved_at = self.saved_at.max(revision);
    }

    /// The document changed: push a new state, and remember at which revision
    /// it happened so a save knows what it is writing.
    fn mark_changed(&mut self) {
        self.revision += 1;
        self.changed_at = self.revision;
    }

    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    // --- the sync seam (M5 drives these) ------------------------------------
    //
    // Bytes in, bytes out, no protocol. `sync` will seal and unseal them and
    // carry them over a WebSocket; nothing about that belongs here, and
    // nothing here needs to know it happened.

    /// This replica's version, opaque. A peer holding it can compute exactly
    /// what this one is missing.
    pub fn version(&self) -> Vec<u8> {
        self.document.version()
    }

    /// Whether this replica started from nothing at launch — no stored
    /// snapshot, so nothing merged into it in a previous life.
    ///
    /// The one question a host must ask before resuming a persisted sync
    /// cursor. The cursor and the replica are two files on a device and can
    /// be lost separately; a cursor that survived says "I already have
    /// everything up to frame N" on behalf of a replica that has nothing, and
    /// the relay then honestly replays nothing at all. The library would come
    /// back only when somebody else pushed something (DECISIONS 0042).
    pub fn opened_fresh(&self) -> bool {
        self.fresh
    }

    pub fn changes_since(&self, version: &[u8]) -> Result<Vec<u8>> {
        Ok(self.document.changes_since(version)?)
    }

    /// Applies a peer's changes and returns the state they produced.
    pub fn merge(&mut self, updates: &[u8]) -> Result<StateView> {
        self.document.merge(updates)?;
        self.mark_changed();
        self.state()
    }

    // --- internals ----------------------------------------------------------

    fn view(&self, library: &Library) -> StateView {
        let projection = project::derive(library);
        project::state(
            library,
            &projection,
            self.focus.as_ref(),
            &self.identity,
            self.revision,
        )
    }

    fn now(&self) -> Timestamp {
        self.platform.now()
    }

    fn mint(&self, prefix: &str) -> Result<String> {
        id::mint(&self.platform, prefix)
    }

    /// Runs one command. `Ok(true)` means the document changed.
    fn run(&mut self, command: Command, library: &Library) -> Result<bool> {
        match command {
            Command::SaveIngredient { ingredient } => self.save_ingredient(ingredient, library),
            Command::DeleteIngredient { ingredient } => {
                self.delete_ingredient(&ingredient, library)
            }
            Command::SaveRecipe { recipe } => self.save_recipe(recipe, library),
            Command::DeleteRecipe { recipe } => self.delete_recipe(&recipe, library),
            Command::AddRecipeToList { recipe, servings } => {
                self.add_recipe_to_list(&recipe, servings, library)
            }
            Command::AddIngredientToList {
                ingredient,
                quantity,
            } => self.add_ingredient_to_list(&ingredient, &quantity, library),
            Command::SetEntryServings { entry, servings } => {
                self.set_entry_servings(&entry, servings, library)
            }
            Command::RemoveListEntry { entry } => self.remove_list_entry(&entry, library),
            Command::ToggleCartItem { ingredient } => self.toggle_cart_item(&ingredient, library),
            Command::FinishShopping => self.finish_shopping(library),
            Command::OpenRecipe { recipe, servings } => {
                let recipe = RecipeId::from_raw(recipe);
                if !library.recipes.contains_key(&recipe) {
                    return Err(AppError::not_found("recipe", recipe.as_str()));
                }
                self.focus = Some(Focus {
                    recipe,
                    servings: servings.map(servings_count).transpose()?,
                });
                Ok(false)
            }
            Command::CloseRecipe => {
                self.focus = None;
                Ok(false)
            }
            Command::RenameUser { name } => {
                let name = text("name", &name)?;
                self.document
                    .put_user(&User::new(self.identity.user_id(), name))?;
                Ok(true)
            }
        }
    }

    // --- the library --------------------------------------------------------

    fn save_ingredient(&mut self, input: IngredientInput, library: &Library) -> Result<bool> {
        let name = text("name", &input.name)?.to_owned();
        let id = match &input.id {
            Some(id) => IngredientId::from_raw(id.clone()),
            None => IngredientId::from_raw(self.mint(id::INGREDIENT)?),
        };
        let existing = library.ingredients.get(&id);

        let mut ingredient = Ingredient::new(id.clone(), &name, input.aisle.into());
        ingredient.aliases = input
            .aliases
            .iter()
            .map(|alias| alias.trim().to_owned())
            .filter(|alias| !alias.is_empty())
            .collect();
        ingredient.staple = input.staple;
        ingredient.density = coefficient("density", input.density.as_deref())?;
        ingredient.unit_weight = coefficient("unit_weight", input.unit_weight.as_deref())?;

        self.document.put_ingredient(&ingredient)?;
        if existing.is_some() {
            self.record(Action::Edited, Subject::Ingredient(id), &name)?;
        }
        Ok(true)
    }

    fn delete_ingredient(&mut self, id: &str, library: &Library) -> Result<bool> {
        let id = IngredientId::from_raw(id);
        let ingredient = library
            .ingredients
            .get(&id)
            .ok_or_else(|| AppError::not_found("ingredient", id.as_str()))?;
        let label = ingredient.name.clone();

        // Recipes that still use it are left alone: referential integrity is
        // not enforceable under a CRDT, so the dangling reference is reported
        // by the domain and rendered as a warning (DECISIONS 0022).
        self.document.remove_ingredient(&id)?;
        self.record(Action::Deleted, Subject::Ingredient(id), &label)?;
        Ok(true)
    }

    fn save_recipe(&mut self, input: RecipeInput, library: &Library) -> Result<bool> {
        let name = text("name", &input.name)?.to_owned();
        let id = match &input.id {
            Some(id) => RecipeId::from_raw(id.clone()),
            None => RecipeId::from_raw(self.mint(id::RECIPE)?),
        };
        let existed = library.recipes.contains_key(&id);

        let mut recipe = Recipe::new(id.clone(), &name, servings_count(input.servings)?);
        recipe.yields = input
            .yields
            .as_ref()
            .map(|quantity| self.quantity("yields", quantity))
            .transpose()?;
        recipe.components = input
            .components
            .iter()
            .map(|component| self.component(component))
            .collect::<Result<_>>()?;
        recipe.steps = input
            .steps
            .iter()
            .map(|step| Step {
                segments: step
                    .segments
                    .iter()
                    .map(|segment| match segment {
                        SegmentInput::Text { text } => Segment::Text(text.clone()),
                        SegmentInput::Ingredient { usage, display } => Segment::Ingredient {
                            usage: UsageId::from_raw(usage.clone()),
                            display: (*display).into(),
                        },
                    })
                    .collect(),
            })
            .collect();

        self.document.put_recipe(&recipe)?;
        if existed {
            self.record(Action::Edited, Subject::Recipe(id), &name)?;
        }
        Ok(true)
    }

    /// One recipe line.
    ///
    /// Notice what is *not* checked: that the ingredient or the sub-recipe
    /// still exists. A recipe whose ingredient another device deleted must
    /// stay editable — refusing the save would make the recipe unfixable
    /// exactly when it needs fixing — and the missing reference already has a
    /// defined rendering (DECISIONS 0022).
    fn component(&self, input: &ComponentInput) -> Result<Component> {
        Ok(match input {
            ComponentInput::Ingredient {
                id,
                ingredient,
                quantity,
            } => Component::Ingredient(IngredientUsage {
                id: UsageId::from_raw(match id {
                    Some(id) => id.clone(),
                    None => self.mint(id::USAGE)?,
                }),
                ingredient: IngredientId::from_raw(ingredient.clone()),
                quantity: self.quantity("quantity", quantity)?,
            }),
            ComponentInput::SubRecipe { id, recipe, amount } => {
                Component::SubRecipe(SubRecipeUsage {
                    id: UsageId::from_raw(match id {
                        Some(id) => id.clone(),
                        None => self.mint(id::USAGE)?,
                    }),
                    recipe: RecipeId::from_raw(recipe.clone()),
                    amount: match amount {
                        SubRecipeAmountInput::Factor { factor } => {
                            SubRecipeAmount::Factor(number::parse_amount("factor", factor)?)
                        }
                        SubRecipeAmountInput::OfYield { quantity } => {
                            SubRecipeAmount::OfYield(self.quantity("quantity", quantity)?)
                        }
                    },
                })
            }
        })
    }

    fn delete_recipe(&mut self, id: &str, library: &Library) -> Result<bool> {
        let id = RecipeId::from_raw(id);
        let recipe = library
            .recipes
            .get(&id)
            .ok_or_else(|| AppError::not_found("recipe", id.as_str()))?;
        let label = recipe.name.clone();

        self.document.remove_recipe(&id)?;
        self.record(Action::Deleted, Subject::Recipe(id), &label)?;
        Ok(true)
    }

    // --- the list -----------------------------------------------------------

    fn add_recipe_to_list(
        &mut self,
        recipe: &str,
        servings: u32,
        library: &Library,
    ) -> Result<bool> {
        let recipe = RecipeId::from_raw(recipe);
        if !library.recipes.contains_key(&recipe) {
            return Err(AppError::not_found("recipe", recipe.as_str()));
        }
        let entry = self.entry(ListItem::Recipe {
            recipe,
            servings: servings_count(servings)?,
        })?;
        self.add_entry(entry, library)
    }

    fn add_ingredient_to_list(
        &mut self,
        ingredient: &str,
        quantity: &QuantityInput,
        library: &Library,
    ) -> Result<bool> {
        let ingredient = IngredientId::from_raw(ingredient);
        if !library.ingredients.contains_key(&ingredient) {
            return Err(AppError::not_found("ingredient", ingredient.as_str()));
        }
        let entry = self.entry(ListItem::Ingredient {
            ingredient,
            quantity: self.quantity("quantity", quantity)?,
        })?;
        self.add_entry(entry, library)
    }

    fn entry(&self, item: ListItem) -> Result<ListEntry> {
        Ok(ListEntry {
            id: ListEntryId::from_raw(self.mint(id::LIST_ENTRY)?),
            item,
            added_by: self.identity.user_id(),
            added_at: self.now(),
        })
    }

    /// Adds an entry through the domain, then persists **both** effects.
    ///
    /// The second one is the load-bearing one: adding a bare ingredient
    /// purges its overlay entry, so a staple that was auto-checked — or an
    /// item ticked off earlier in the same trip — comes back into view
    /// (Rule 3). The purge is computed by the domain and merely written down
    /// here; a store that re-derived it would be a second implementation of
    /// the rule.
    fn add_entry(&mut self, entry: ListEntry, library: &Library) -> Result<bool> {
        let mut list = library.list.clone();
        let mut overlay = library.overlay.clone();
        list.add(entry.clone(), &mut overlay);

        self.document.add_list_entry(&entry)?;
        self.apply_overlay_purge(library, &overlay)?;
        Ok(true)
    }

    fn set_entry_servings(
        &mut self,
        entry: &str,
        servings: u32,
        library: &Library,
    ) -> Result<bool> {
        let id = ListEntryId::from_raw(entry);
        let existing = library
            .list
            .entry(&id)
            .ok_or_else(|| AppError::not_found("list entry", id.as_str()))?;
        let ListItem::Recipe { recipe, .. } = &existing.item else {
            return Err(AppError::invalid(
                "servings",
                "a bare ingredient on the list has no serving count",
            ));
        };

        self.document.update_list_entry(&ListEntry {
            item: ListItem::Recipe {
                recipe: recipe.clone(),
                servings: servings_count(servings)?,
            },
            ..existing.clone()
        })?;
        Ok(true)
    }

    fn remove_list_entry(&mut self, entry: &str, library: &Library) -> Result<bool> {
        let id = ListEntryId::from_raw(entry);
        let existing = library
            .list
            .entry(&id)
            .ok_or_else(|| AppError::not_found("list entry", id.as_str()))?;
        let label = match &existing.item {
            ListItem::Recipe { recipe, .. } => library
                .recipes
                .get(recipe)
                .map(|r| r.name.clone())
                .unwrap_or_else(|| recipe.to_string()),
            ListItem::Ingredient { ingredient, .. } => library
                .ingredient_name(ingredient)
                .map(str::to_owned)
                .unwrap_or_else(|| ingredient.to_string()),
        };

        self.document.remove_list_entry(&id)?;
        self.record(Action::Deleted, Subject::ListEntry(id), &label)?;
        Ok(true)
    }

    // --- the cart -----------------------------------------------------------

    /// One tap on a cart line.
    ///
    /// Reads the *derived* state rather than the overlay, which is what makes
    /// unchecking an auto-checked staple work: there is no explicit entry to
    /// remove, so the tap has to store an explicit `Unchecked` — and without
    /// it the next derivation would put the staple straight back (Rule 3).
    fn toggle_cart_item(&mut self, ingredient: &str, library: &Library) -> Result<bool> {
        let id = IngredientId::from_raw(ingredient);
        let cart = project::derive(library).cart;
        let line = cart
            .line(&id)
            .ok_or_else(|| AppError::not_found("cart line", id.as_str()))?;

        let explicit = if line.is_settled() {
            Explicit::Unchecked
        } else {
            Explicit::Checked {
                by: self.identity.user_id(),
                at: self.now(),
            }
        };
        self.document.set_explicit(&id, &explicit)?;
        Ok(true)
    }

    /// Ends the trip. The domain decides what goes; this writes it down.
    ///
    /// Both halves matter and neither is guessable from the other: completed
    /// entries leave the list, and the overlay is pruned *selectively* — an
    /// ingredient shared with an entry that is staying keeps its tick
    /// (DECISIONS 0028).
    fn finish_shopping(&mut self, library: &Library) -> Result<bool> {
        let cart = project::derive(library).cart;
        let mut list = library.list.clone();
        let mut overlay = library.overlay.clone();
        finish_shopping(&mut list, &cart, &mut overlay);

        for entry in &library.list.entries {
            if !list.entries.iter().any(|kept| kept.id == entry.id) {
                self.document.remove_list_entry(&entry.id)?;
            }
        }
        self.apply_overlay_purge(library, &overlay)?;
        Ok(true)
    }

    /// Writes down the overlay entries the domain dropped.
    fn apply_overlay_purge(
        &mut self,
        library: &Library,
        remaining: &cabas_domain::Overlay,
    ) -> Result<()> {
        for ingredient in library.overlay.keys() {
            if !remaining.contains_key(ingredient) {
                self.document.clear_explicit(ingredient)?;
            }
        }
        Ok(())
    }

    // --- shared -------------------------------------------------------------

    fn quantity(&self, field: &'static str, input: &QuantityInput) -> Result<Quantity> {
        Ok(Quantity::new(
            number::parse_amount(field, &input.amount)?,
            input.unit.into(),
        ))
    }

    /// Records a deletion or an edit.
    ///
    /// Creations are absent on purpose: they are already attributed on the
    /// object itself, and logging them too would fill a capped log with
    /// entries carrying nothing the data does not already say
    /// (DECISIONS 0024). The label is copied rather than looked up, because
    /// the whole point of logging a deletion is that the subject is gone.
    fn record(&mut self, action: Action, subject: Subject, label: &str) -> Result<()> {
        self.document.record_event(&Event::new(
            self.now(),
            self.identity.user_id(),
            action,
            subject,
            label,
        ))?;
        Ok(())
    }
}

fn servings_count(servings: u32) -> Result<NonZeroU32> {
    NonZeroU32::new(servings)
        .ok_or_else(|| AppError::invalid("servings", "a recipe serves at least one person"))
}

fn text<'a>(field: &'static str, raw: &'a str) -> Result<&'a str> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::invalid(field, "must not be empty"));
    }
    Ok(trimmed)
}

/// A conversion coefficient: strictly positive, or absent.
///
/// An empty field means "not known", which is the honest and common answer —
/// and the one that keeps mass and volume on separate lines rather than
/// inventing a density (Rule 5).
fn coefficient(field: &'static str, raw: Option<&str>) -> Result<Option<Rational>> {
    match raw.map(str::trim) {
        None | Some("") => Ok(None),
        Some(value) => Ok(Some(number::parse_amount(field, value)?)),
    }
}
