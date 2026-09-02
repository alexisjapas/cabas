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

use std::collections::BTreeSet;
use std::num::NonZeroU32;

use cabas_domain::event::{Action, Subject};
use cabas_domain::list::{ListEntry, ListItem};
use cabas_domain::recipe::{Component, IngredientUsage, Segment, Step, SubRecipeUsage};
use cabas_domain::{
    Device, Event, Explicit, Ingredient, IngredientId, ListEntryId, Nudged, PhotoId, Quantity,
    Rational, Recipe, RecipeId, Shop, ShopId, SubRecipeAmount, Timestamp, UsageId, User, UserId,
    finish_shopping, nudge_quantity, nudge_servings,
};
use cabas_store::{Document, Storage};

use crate::command::{
    Command, ComponentInput, IngredientInput, QuantityInput, RecipeInput, SegmentInput, ShopInput,
    SubRecipeAmountInput,
};
use crate::error::{AppError, Result};
use crate::library::Library;
use crate::platform::{Identity, Platform};
use crate::project::{self, Focus};
use crate::transfer::{self, ImportReport, LibraryFile};
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

    /// Writes this device and its owner into the group document, if they are
    /// not already there.
    ///
    /// Only ever *adds*. The name in the document wins over the one the host
    /// passed in, because the other device may have renamed the person since
    /// this one last launched, and a launch is not a rename.
    ///
    /// **Does nothing at all on a device that has not said who is carrying
    /// it** (DECISIONS 0068). That is the whole of the joining window: a
    /// device record needs an owner, so writing one before there is a person
    /// would mean inventing the person — which is exactly the ghost member
    /// the roster is there to avoid. `choose_user` and `create_user` call it
    /// again the moment somebody is chosen.
    fn enrol(&mut self) -> Result<()> {
        let Some(user) = self.identity.user_id() else {
            return Ok(());
        };
        if !self.document.users()?.iter().any(|u| u.id == user) {
            // The name is the host's copy, which is only ever consulted when
            // the document has lost the record — a replica rebuilt from
            // nothing under a `localStorage` that survived.
            let name = self.identity.user_name.clone().unwrap_or_default();
            self.document.put_user(&User::new(user.clone(), name))?;
            self.mark_changed();
        }

        let device = self.identity.device_id();
        match self.device_record()? {
            // Already here and pointing at the right person: nothing to say.
            Some(held) if held.owner == user => {}
            // Here, pointing elsewhere: somebody handed the phone over
            // (0068). The device keeps its id, its name and the date it
            // joined — only who carries it changes, and rewriting `paired_at`
            // would claim it joined again today.
            Some(held) => {
                self.document
                    .put_device(&Device::new(device, user, held.name, held.paired_at))?;
                self.mark_changed();
            }
            None => {
                self.document.put_device(&Device::new(
                    device,
                    user,
                    &self.identity.device_name,
                    self.platform.now(),
                ))?;
                self.mark_changed();
            }
        }
        Ok(())
    }

    /// This device's own row in the roster, if the document holds one yet.
    ///
    /// It does not until somebody is carrying the device: a record needs an
    /// owner, so the joining window has an identity with no row (0068).
    fn device_record(&self) -> Result<Option<Device>> {
        let id = self.identity.device_id();
        Ok(self.document.devices()?.into_iter().find(|d| d.id == id))
    }

    /// The person this device belongs to, or a refusal (DECISIONS 0068).
    ///
    /// Every attributable write goes through here rather than reading the
    /// identity directly, so the joining window cannot leak a row signed by
    /// nobody.
    fn user_id(&self) -> Result<UserId> {
        self.identity.user_id().ok_or(AppError::NoUser)
    }

    /// This device now belongs to a person the group already knows
    /// (DECISIONS 0068).
    ///
    /// The name comes out of the document rather than from the caller: the
    /// frontend showed a roster it had been given, and the only thing it is
    /// trusted to send back is which row was tapped.
    fn choose_user(&mut self, user: &str, library: &Library) -> Result<bool> {
        let id = UserId::from_raw(user);
        let name = library
            .user_name(&id)
            .ok_or_else(|| AppError::not_found("user", id.as_str()))?
            .to_owned();
        self.identity.belongs_to(&id, &name);
        self.enrol()?;
        Ok(true)
    }

    /// Names this device (DECISIONS 0068).
    ///
    /// Before there is an owner it is only the host's copy that changes,
    /// which is the whole reason this command exists: the record is then
    /// written once, already named, by the choose or create that follows.
    /// Afterwards it is a rename of a record that is already in the roster.
    fn name_device(&mut self, name: &str) -> Result<bool> {
        let name = text("name", name)?.to_owned();
        self.identity.device_name = name.clone();

        let id = self.identity.device_id();
        let Some(held) = self.device_record()? else {
            // Nothing in the document yet, and nothing to say about it: the
            // host's copy is the only place this lives until somebody is
            // chosen. No document change, so no state push is owed either.
            return Ok(false);
        };
        self.document
            .put_device(&Device::new(id, held.owner, name, held.paired_at))?;
        Ok(true)
    }

    /// A person the group did not have yet, and this device is them.
    ///
    /// The same command whether it is the first member of a brand-new group
    /// or a third person joining an old one — there is no difference to make,
    /// and inventing one would be inventing a notion of ownership the key
    /// model does not have (Rule 7).
    ///
    /// The record itself is left to `enrol`, which writes a user the document
    /// does not hold and is called here the moment the identity moves — so a
    /// person is minted in one place whether they were chosen or invented.
    fn create_user(&mut self, name: &str) -> Result<bool> {
        let name = text("name", name)?.to_owned();
        let id = UserId::from_raw(self.mint(id::USER)?);
        self.identity.belongs_to(&id, &name);
        self.enrol()?;
        Ok(true)
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

    /// Who this device says it is.
    ///
    /// The host holds the only durable copy (DECISIONS 0031), and
    /// [`Command::ChooseUser`] and [`Command::CreateUser`] change it — so a
    /// host that ran one reads this back and writes it down again, or the
    /// next launch is a device that has forgotten who is carrying it
    /// (DECISIONS 0068).
    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    /// Every photo this replica currently references, deduplicated and in
    /// order.
    ///
    /// The document names photos and never carries them (DECISIONS 0062), so
    /// this is the whole of what the replica has to say about them: what a
    /// prefetch is owed, and what a local cleanup may keep. Both live in
    /// [`crate::photos::Photos`], which holds the store this one deliberately
    /// does not.
    pub fn referenced_photos(&self) -> Result<Vec<PhotoId>> {
        let mut ids: Vec<PhotoId> = self
            .document
            .ingredients()?
            .into_iter()
            .filter_map(|ingredient| ingredient.photo)
            .collect();
        ids.extend(
            self.document
                .recipes()?
                .into_iter()
                .filter_map(|recipe| recipe.photo),
        );
        ids.sort();
        ids.dedup();
        Ok(ids)
    }

    // --- the library as a file (DECISIONS 0076) -----------------------------

    /// The whole library, ready to be written to a file.
    ///
    /// Photos are **named and not carried**: reading them is asynchronous and
    /// this is not, for the same reason every read here is synchronous — a
    /// host with one thread must not hold a borrow of the app across an await
    /// (DECISIONS 0032). The host takes [`LibraryFile::photo_ids`], fetches
    /// what it wants from [`crate::Photos`], and puts them in with
    /// [`LibraryFile::attach_photo`]. An export with no photos in it is a
    /// complete, valid file — it is the form somebody edits by hand.
    pub fn export_library(&self) -> Result<LibraryFile> {
        let library = Library::read(&self.document)?;
        Ok(transfer::export(
            &library,
            env!("CARGO_PKG_VERSION"),
            self.now(),
        ))
    }

    /// Merges a file into this replica, and says what it did.
    ///
    /// Synchronous like [`App::apply`], and for the same reason. It writes no
    /// photo: the bytes live in a store of their own (DECISIONS 0062), so the
    /// host restores them first with [`crate::Photos::restore`] — and an
    /// entity that arrives naming a photo nobody carried is the ordinary
    /// state, not a failure.
    ///
    /// **Nothing is ever deleted here.** What the file holds wins over what
    /// is already in the document; what it does not mention is left exactly
    /// as it was. That is not timidity: a delete under a CRDT is a group-wide
    /// fact, so an import that pruned would reach through the relay and take
    /// a recipe off the other person's phone — one person opening a file
    /// would be deciding for two.
    pub fn import_library(&mut self, file: &LibraryFile) -> Result<ImportReport> {
        let library = Library::read(&self.document)?;

        let resolved = {
            let platform = &self.platform;
            let mut mint = |prefix: &'static str| id::mint(platform, prefix);
            transfer::resolve(file, &library, &mut mint)?
        };

        // Everything is parsed and every reference is decided before the
        // first write. There is no transaction under the document, so this is
        // the only thing that makes "the import failed" mean "nothing
        // happened" rather than "some of it happened, in an order nobody
        // recorded".
        let shops = resolved
            .shops
            .iter()
            .map(|input| self.shop_from(input))
            .collect::<Result<Vec<_>>>()?;
        let ingredients = resolved
            .ingredients
            .iter()
            .map(|input| self.ingredient_from(input))
            .collect::<Result<Vec<_>>>()?;
        let recipes = resolved
            .recipes
            .iter()
            .map(|input| self.recipe_from(input))
            .collect::<Result<Vec<_>>>()?;

        let touched = shops.len() + ingredients.len() + recipes.len();
        for shop in &shops {
            self.document.put_shop(shop)?;
        }
        for ingredient in &ingredients {
            self.document.put_ingredient(ingredient)?;
        }
        for recipe in &recipes {
            self.document.put_recipe(recipe)?;
        }
        if touched > 0 {
            self.mark_changed();
        }

        Ok(resolved.report)
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
    /// cursor (DECISIONS 0045). The cursor and the replica are two files on a
    /// device and can be lost separately; a cursor that survived says "I
    /// already have everything up to frame N" on behalf of a replica that has
    /// nothing, and the relay then honestly replays nothing at all. The
    /// library would come back only when somebody else pushed something
    /// (DECISIONS 0042).
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
            Command::SaveShop { shop } => self.save_shop(shop),
            Command::DeleteShop { shop } => self.delete_shop(&shop, library),
            Command::SaveRecipe { recipe } => self.save_recipe(recipe, library),
            Command::DeleteRecipe { recipe } => self.delete_recipe(&recipe, library),
            Command::AddRecipeToList {
                recipe,
                servings,
                only,
            } => self.add_recipe_to_list(&recipe, servings, only, library),
            Command::AddIngredientToList {
                ingredient,
                quantity,
            } => self.add_ingredient_to_list(&ingredient, quantity.as_ref(), library),
            Command::SetEntryServings { entry, servings } => {
                self.set_entry_servings(&entry, servings, library)
            }
            Command::SetEntryComponents { entry, only } => {
                self.set_entry_components(&entry, only, library)
            }
            Command::SetEntryQuantity { entry, quantity } => {
                self.set_entry_quantity(&entry, &quantity, library)
            }
            Command::NudgeListEntry { entry, steps } => {
                self.nudge_list_entry(&entry, steps, library)
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
                let name = text("name", &name)?.to_owned();
                let user = self.user_id()?;
                self.document.put_user(&User::new(user.clone(), &name))?;
                // The host's copy follows, or a replica rebuilt from nothing
                // would put the old name back (see `enrol`).
                self.identity.belongs_to(&user, &name);
                Ok(true)
            }
            Command::ChooseUser { user } => self.choose_user(&user, library),
            Command::CreateUser { name } => self.create_user(&name),
            Command::NameDevice { name } => self.name_device(&name),
        }
    }

    // --- the library --------------------------------------------------------

    fn save_ingredient(&mut self, input: IngredientInput, library: &Library) -> Result<bool> {
        let ingredient = self.ingredient_from(&input)?;
        let existed = library.ingredients.contains_key(&ingredient.id);
        let (id, name) = (ingredient.id.clone(), ingredient.name.clone());

        self.document.put_ingredient(&ingredient)?;
        if existed {
            self.record(Action::Edited, Subject::Ingredient(id), &name)?;
        }
        Ok(true)
    }

    /// One ingredient, as the document will hold it — everything that
    /// validates, and nothing that writes.
    ///
    /// Split out of [`App::save_ingredient`] because an import builds the
    /// same value and must not do the two things left above it (DECISIONS
    /// 0076). It must not **log**: the event log is capped at 200 entries and
    /// one file can carry more ingredients than that, so an import routed
    /// through the save would push out every deletion the log exists to
    /// remember. And it must not **write yet**: an import decides every
    /// entity before the first `put`, because there is no transaction under
    /// this and a failure halfway would leave a library nobody could
    /// describe.
    fn ingredient_from(&self, input: &IngredientInput) -> Result<Ingredient> {
        let name = text("name", &input.name)?.to_owned();
        let id = match &input.id {
            Some(id) => IngredientId::from_raw(id.clone()),
            None => IngredientId::from_raw(self.mint(id::INGREDIENT)?),
        };

        let mut ingredient = Ingredient::new(id, &name, input.aisle.into());
        ingredient.aliases = input
            .aliases
            .iter()
            .map(|alias| alias.trim().to_owned())
            .filter(|alias| !alias.is_empty())
            .collect();
        // Ids, and not checked against the shop library: a shop the *other*
        // device deleted a second ago must not make this save fail, and a
        // dangling id filters nothing (DECISIONS 0022, 0071).
        ingredient.shops = input
            .shops
            .iter()
            .map(|shop| ShopId::from_raw(shop.clone()))
            .collect();
        ingredient.keeping = input.keeping.into();
        ingredient.staple = input.staple;
        ingredient.density = coefficient("density", input.density.as_deref())?;
        ingredient.unit_weight = coefficient("unit_weight", input.unit_weight.as_deref())?;
        ingredient.default_quantity = input
            .default_quantity
            .as_ref()
            .map(|q| self.quantity("default_quantity", q))
            .transpose()?;
        ingredient.photo = photo(input.photo.as_deref())?;
        Ok(ingredient)
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

    /// Creates or renames a shop (DECISIONS 0071).
    ///
    /// Nothing is recorded in the event log, unlike every other library
    /// write. The log exists for what the data cannot remember — a deleted
    /// recipe leaves no field behind to hold "and Alexis did this" (0024) —
    /// and a shop is a label on a handful of ingredients: forgetting one
    /// changes no amount, breaks no line, and is undone by typing the name
    /// again. Recording it would also mean widening the log's persisted
    /// `subject_kind`, which is a schema change bought for a row nobody
    /// would read.
    fn save_shop(&mut self, input: ShopInput) -> Result<bool> {
        let shop = self.shop_from(&input)?;
        self.document.put_shop(&shop)?;
        Ok(true)
    }

    /// One shop, validated and not yet written. See [`App::ingredient_from`]
    /// for why the two halves are separate.
    fn shop_from(&self, input: &ShopInput) -> Result<Shop> {
        let name = text("name", &input.name)?.to_owned();
        let id = match &input.id {
            Some(id) => ShopId::from_raw(id.clone()),
            None => ShopId::from_raw(self.mint(id::SHOP)?),
        };
        Ok(Shop::new(id, name))
    }

    fn delete_shop(&mut self, id: &str, library: &Library) -> Result<bool> {
        let id = ShopId::from_raw(id);
        if library.shop(&id).is_none() {
            return Err(AppError::not_found("shop", id.as_str()));
        }
        // Ingredients still naming it are left alone, for the reason
        // `delete_ingredient` gives: referential integrity is not enforceable
        // under a CRDT. Here the dangling id is harmless — it matches no
        // shop, so it filters nothing.
        self.document.remove_shop(&id)?;
        Ok(true)
    }

    fn save_recipe(&mut self, input: RecipeInput, library: &Library) -> Result<bool> {
        let recipe = self.recipe_from(&input)?;
        let existed = library.recipes.contains_key(&recipe.id);
        let (id, name) = (recipe.id.clone(), recipe.name.clone());

        self.document.put_recipe(&recipe)?;
        if existed {
            self.record(Action::Edited, Subject::Recipe(id), &name)?;
        }
        Ok(true)
    }

    /// One recipe, validated and not yet written. See [`App::ingredient_from`]
    /// for why the two halves are separate.
    fn recipe_from(&self, input: &RecipeInput) -> Result<Recipe> {
        let name = text("name", &input.name)?.to_owned();
        let id = match &input.id {
            Some(id) => RecipeId::from_raw(id.clone()),
            None => RecipeId::from_raw(self.mint(id::RECIPE)?),
        };

        let mut recipe = Recipe::new(id, &name, servings_count(input.servings)?);
        recipe.yields = input
            .yields
            .as_ref()
            .map(|quantity| self.quantity("yields", quantity))
            .transpose()?;
        recipe.photo = photo(input.photo.as_deref())?;
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
        Ok(recipe)
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

    /// Puts a recipe on the list.
    ///
    /// No `servings` means "as it is written": the recipe's own count, read
    /// off the recipe rather than decided by the caller, for the same reason
    /// a missing quantity is (DECISIONS 0066, 0067).
    fn add_recipe_to_list(
        &mut self,
        recipe: &str,
        servings: Option<u32>,
        only: Option<Vec<String>>,
        library: &Library,
    ) -> Result<bool> {
        let recipe = RecipeId::from_raw(recipe);
        let held = library
            .recipes
            .get(&recipe)
            .ok_or_else(|| AppError::not_found("recipe", recipe.as_str()))?;
        let servings = match servings {
            Some(servings) => servings_count(servings)?,
            None => held.servings,
        };
        let only = chosen_components(held, only)?;
        let entry = self.entry(ListItem::Recipe {
            recipe,
            servings,
            only,
        })?;
        self.add_entry(entry, library)
    }

    /// Puts a bare ingredient on the list.
    ///
    /// No `quantity` means "as much of it as one usually buys": the
    /// ingredient's own default, or one piece (DECISIONS 0066). The
    /// substitution is read off the ingredient rather than decided here,
    /// because it is the same rule wherever an amount is missing.
    fn add_ingredient_to_list(
        &mut self,
        ingredient: &str,
        quantity: Option<&QuantityInput>,
        library: &Library,
    ) -> Result<bool> {
        let id = IngredientId::from_raw(ingredient);
        let Some(known) = library.ingredients.get(&id) else {
            return Err(AppError::not_found("ingredient", id.as_str()));
        };
        let quantity = match quantity {
            Some(input) => self.quantity("quantity", input)?,
            None => known.shopping_quantity(),
        };
        let entry = self.entry(ListItem::Ingredient {
            ingredient: id,
            quantity,
        })?;
        self.add_entry(entry, library)
    }

    fn entry(&self, item: ListItem) -> Result<ListEntry> {
        Ok(ListEntry {
            id: ListEntryId::from_raw(self.mint(id::LIST_ENTRY)?),
            item,
            added_by: self.user_id()?,
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

    /// Changes what an entry asks for, then persists **both** effects.
    ///
    /// The mirror of [`Self::add_entry`], and it exists for the same second
    /// half: asking for more or less of a bare ingredient is the same
    /// statement as putting it on the list, so it purges the tick the same
    /// way (Rule 3, DECISIONS 0079). Every command that rewrites a line goes
    /// through here — writing `update_list_entry` directly is how the rule
    /// was missing from three of them.
    fn update_entry(&mut self, entry: ListEntry, library: &Library) -> Result<bool> {
        let mut list = library.list.clone();
        let mut overlay = library.overlay.clone();
        list.update(entry.clone(), &mut overlay);

        self.document.update_list_entry(&entry)?;
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
        let ListItem::Recipe { recipe, only, .. } = &existing.item else {
            return Err(AppError::invalid(
                "servings",
                "a bare ingredient on the list has no serving count",
            ));
        };

        self.update_entry(
            ListEntry {
                item: ListItem::Recipe {
                    recipe: recipe.clone(),
                    servings: servings_count(servings)?,
                    // Rescaling says nothing about *which* lines are wanted,
                    // and scaling only the chosen ones is the whole point of
                    // them being on a recipe entry (DECISIONS 0091).
                    only: only.clone(),
                },
                ..existing.clone()
            },
            library,
        )
    }

    /// Which of a recipe's lines an entry already on the list asks for
    /// (DECISIONS 0091).
    ///
    /// The other door into the same field: [`Self::add_recipe_to_list`] opens
    /// an entry with a selection, and this changes one. Both validate against
    /// the recipe as it stands right now, which is why the check lives in
    /// [`chosen_components`] rather than at either call site.
    fn set_entry_components(
        &mut self,
        entry: &str,
        only: Option<Vec<String>>,
        library: &Library,
    ) -> Result<bool> {
        let id = ListEntryId::from_raw(entry);
        let existing = library
            .list
            .entry(&id)
            .ok_or_else(|| AppError::not_found("list entry", id.as_str()))?;
        let ListItem::Recipe {
            recipe, servings, ..
        } = &existing.item
        else {
            return Err(AppError::invalid(
                "only",
                "a bare ingredient on the list has no lines to choose from",
            ));
        };
        let held = library
            .recipes
            .get(recipe)
            .ok_or_else(|| AppError::not_found("recipe", recipe.as_str()))?;

        self.update_entry(
            ListEntry {
                item: ListItem::Recipe {
                    recipe: recipe.clone(),
                    servings: *servings,
                    only: chosen_components(held, only)?,
                },
                ..existing.clone()
            },
            library,
        )
    }

    /// Sets exactly what a bare ingredient on the list asks for — what the
    /// long press opens (DECISIONS 0072).
    fn set_entry_quantity(
        &mut self,
        entry: &str,
        quantity: &QuantityInput,
        library: &Library,
    ) -> Result<bool> {
        let id = ListEntryId::from_raw(entry);
        let existing = library
            .list
            .entry(&id)
            .ok_or_else(|| AppError::not_found("list entry", id.as_str()))?;
        let ListItem::Ingredient { ingredient, .. } = &existing.item else {
            return Err(AppError::invalid(
                "quantity",
                "a recipe on the list is measured in people",
            ));
        };

        self.update_entry(
            ListEntry {
                item: ListItem::Ingredient {
                    ingredient: ingredient.clone(),
                    quantity: self.quantity("quantity", quantity)?,
                },
                ..existing.clone()
            },
            library,
        )
    }

    /// One more of this, or one less (DECISIONS 0072).
    ///
    /// What a notch is worth is the domain's, and it differs by what is on
    /// the line: the ingredient's usual shopping quantity, or one whole
    /// recipe as written. Nudging the last one down removes the entry
    /// **through the ordinary path**, so it is recorded in the log and reads
    /// the same as pressing "Annuler" — which is what it is.
    fn nudge_list_entry(&mut self, entry: &str, steps: i32, library: &Library) -> Result<bool> {
        let id = ListEntryId::from_raw(entry);
        let existing = library
            .list
            .entry(&id)
            .ok_or_else(|| AppError::not_found("list entry", id.as_str()))?
            .clone();

        let item = match &existing.item {
            ListItem::Ingredient {
                ingredient,
                quantity,
            } => {
                let known = library
                    .ingredients
                    .get(ingredient)
                    .ok_or_else(|| AppError::not_found("ingredient", ingredient.as_str()))?;
                match nudge_quantity(quantity, known, steps) {
                    Nudged::To(quantity) => ListItem::Ingredient {
                        ingredient: ingredient.clone(),
                        quantity,
                    },
                    Nudged::Off => return self.remove_list_entry(entry, library),
                    // Refused rather than guessed: the notch and the line do
                    // not share a dimension and this ingredient has no
                    // coefficient to cross it (Rule 5), or the line is an
                    // amount nobody measured.
                    Nudged::Refused => {
                        return Err(AppError::invalid(
                            "steps",
                            "this amount cannot be counted up or down",
                        ));
                    }
                }
            }
            ListItem::Recipe {
                recipe,
                servings,
                only,
            } => {
                let written_for = library
                    .recipes
                    .get(recipe)
                    .map_or(*servings, |held| held.servings);
                match nudge_servings(*servings, written_for, steps) {
                    Some(servings) => ListItem::Recipe {
                        recipe: recipe.clone(),
                        servings,
                        // A notch is a number of people, not a change of mind
                        // about which lines are wanted (DECISIONS 0091).
                        only: only.clone(),
                    },
                    None => return self.remove_list_entry(entry, library),
                }
            }
        };

        self.update_entry(ListEntry { item, ..existing }, library)
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
                by: self.user_id()?,
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
            self.user_id()?,
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

/// The lines of `recipe` a list entry may ask for (DECISIONS 0091).
///
/// `None` in, `None` out: the whole recipe, which is what every gesture
/// produces and what a stored entry written before 0091 says.
///
/// Two things are refused rather than tidied. An **empty** selection is a row
/// that asks for nothing, can never complete and therefore never leaves the
/// list — what the person meant is `RemoveListEntry`, and inventing that here
/// would be deciding for them. And an id the recipe **does not have** is
/// refused because the caller can be told: this runs while somebody is
/// looking at the recipe. Once stored, the same situation is tolerated in
/// silence — the other device deleted the line since — and `expand_only` and
/// the view both read past it.
fn chosen_components(
    recipe: &cabas_domain::Recipe,
    only: Option<Vec<String>>,
) -> Result<Option<BTreeSet<UsageId>>> {
    let Some(only) = only else {
        return Ok(None);
    };
    let chosen: BTreeSet<UsageId> = only.into_iter().map(UsageId::from_raw).collect();
    if chosen.is_empty() {
        return Err(AppError::invalid(
            "only",
            "an entry asking for none of a recipe is an entry to remove",
        ));
    }
    if let Some(stranger) = chosen
        .iter()
        .find(|id| !recipe.components.iter().any(|c| c.id() == *id))
    {
        return Err(AppError::not_found("recipe line", stranger.as_str()));
    }
    Ok(Some(chosen))
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

/// A photo reference as it arrives from a host: an id, an empty string, or
/// nothing at all — the last two both meaning "no photo" (DECISIONS 0062).
///
/// It is not checked against the photo store. The bytes may legitimately not
/// be here: the id may name a photo the other phone took, whose bytes are
/// still on their way, and refusing the save would be refusing to merge.
fn photo(raw: Option<&str>) -> Result<Option<PhotoId>> {
    match raw.map(str::trim) {
        None | Some("") => Ok(None),
        Some(id) => Ok(Some(PhotoId::from_raw(id))),
    }
}
