//! A shopping trip, through the public command surface only.
//!
//! This is M3's exit criterion, and the best place to see what the app layer
//! actually does: build a library, put a recipe on the list for a different
//! number of people, tick things off in the shop, finish, and find everything
//! still there after a restart.
//!
//! # It runs on both targets
//!
//! Natively under nextest, and in headless chromium through
//! `wasm-bindgen-test`. The second run is not ceremony: `getrandom` without
//! its web backend, or a clock that panics on wasm32, are both invisible
//! natively and fatal on a phone — a blank page with nothing in the console
//! (Rule 8). The storage backend is the in-memory one on both, because what
//! is under test here is the app layer; IndexedDB itself has its own browser
//! test in `store` (DECISIONS 0030).

use cabas_app::command::{
    ComponentInput, IngredientInput, QuantityInput, RecipeInput, SegmentInput, StepInput,
};
use cabas_app::photos::Photos;
use cabas_app::tags::{ActionTag, AisleTag, CheckStateTag, RefDisplayTag, SubjectTag, UnitTag};
use cabas_app::view::{
    CartLineView, ComponentView, ListItemView, ProblemKind, SegmentView, StateView,
};
use cabas_app::{App, Command, Identity, Platform};
use cabas_domain::Timestamp;
use cabas_store::{MemoryPhotoStore, MemoryStorage};

/// A clock that does not tick and a "random" source that counts.
///
/// Both matter: the ids in a failing assertion are then readable, and two
/// runs of the suite produce the same document. The trait exists precisely so
/// a test can say this (Rule 1 pushed the impurity up here on purpose).
#[derive(Debug, Default)]
struct TestPlatform {
    counter: std::cell::Cell<u64>,
}

impl Platform for TestPlatform {
    fn now(&self) -> Timestamp {
        Timestamp(1_700_000_000_000)
    }

    fn random_u64(&self) -> cabas_app::Result<u64> {
        self.counter.set(self.counter.get() + 1);
        Ok(self.counter.get())
    }
}

fn identity() -> Identity {
    Identity {
        user: Some("usr_alice".into()),
        user_name: Some("Alice".into()),
        device: "dev_phone".into(),
        device_name: "Alice's iPhone".into(),
    }
}

async fn open(storage: MemoryStorage) -> App<MemoryStorage, TestPlatform> {
    App::open(storage, TestPlatform::default(), identity())
        .await
        .expect("the app opens")
}

fn amount(value: &str, unit: UnitTag) -> QuantityInput {
    QuantityInput {
        amount: value.into(),
        unit,
    }
}

fn new_ingredient(name: &str, aisle: AisleTag) -> IngredientInput {
    IngredientInput {
        id: None,
        name: name.into(),
        aliases: Vec::new(),
        aisle,
        staple: false,
        density: None,
        unit_weight: None,
        default_quantity: None,
        photo: None,
    }
}

fn id_of_ingredient(state: &StateView, name: &str) -> String {
    state
        .ingredients
        .iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| panic!("no ingredient named {name}"))
        .id
        .clone()
}

fn id_of_recipe(state: &StateView, name: &str) -> String {
    state
        .recipes
        .iter()
        .find(|r| r.name == name)
        .unwrap_or_else(|| panic!("no recipe named {name}"))
        .id
        .clone()
}

fn line<'a>(lines: &'a [CartLineView], name: &str) -> &'a CartLineView {
    lines
        .iter()
        .find(|l| l.name == name)
        .unwrap_or_else(|| panic!("no cart line for {name} in {:?}", names(lines)))
}

fn names(lines: &[CartLineView]) -> Vec<&str> {
    lines.iter().map(|l| l.name.as_str()).collect()
}

/// The library every scenario below starts from.
async fn stocked(app: &mut App<MemoryStorage, TestPlatform>) -> StateView {
    let mut flour = new_ingredient("Flour", AisleTag::Grocery);
    flour.staple = true;
    flour.density = Some("0.55".into());
    let mut tomato = new_ingredient("Tomato", AisleTag::Produce);
    tomato.unit_weight = Some("150".into());
    let egg = new_ingredient("Egg", AisleTag::Dairy);

    let mut state = StateView::clone(&app.state().expect("state"));
    for ingredient in [flour, tomato, egg] {
        state = app
            .dispatch(Command::SaveIngredient { ingredient })
            .await
            .expect("the ingredient is saved");
    }
    state
}

/// A tart for four: 200 g of flour, three tomatoes, two eggs.
async fn tart(app: &mut App<MemoryStorage, TestPlatform>, state: &StateView) -> StateView {
    let recipe = RecipeInput {
        id: None,
        name: "Tomato tart".into(),
        servings: 4,
        yields: None,
        photo: None,
        components: vec![
            ComponentInput::Ingredient {
                id: None,
                ingredient: id_of_ingredient(state, "Flour"),
                quantity: amount("200", UnitTag::G),
            },
            ComponentInput::Ingredient {
                id: None,
                ingredient: id_of_ingredient(state, "Tomato"),
                quantity: amount("3", UnitTag::Piece),
            },
            ComponentInput::Ingredient {
                id: None,
                ingredient: id_of_ingredient(state, "Egg"),
                quantity: amount("2", UnitTag::Piece),
            },
        ],
        steps: Vec::new(),
    };
    app.dispatch(Command::SaveRecipe { recipe })
        .await
        .expect("the recipe is saved")
}

async fn scenario() {
    let storage = MemoryStorage::new();
    let mut app = open(storage.clone()).await;

    // --- the library ------------------------------------------------------
    let state = stocked(&mut app).await;
    assert_eq!(state.ingredients.len(), 3);
    assert_eq!(state.me.as_ref().expect("a user is chosen").name, "Alice");

    let state = tart(&mut app, &state).await;
    let recipe = id_of_recipe(&state, "Tomato tart");

    // --- steps reference usages, so they must be named first --------------
    //
    // Saving the lines and reading their ids back out of `focus.edit` is one
    // way round it, and the one exercised here because it also proves that
    // `edit` round-trips. An editor takes the other — it mints the id before
    // the line exists, so a step can mention a line the recipe has never been
    // saved with (DECISIONS 0039); `a_recipe_is_writable_in_a_single_save`
    // covers that path.
    let state = app
        .dispatch(Command::OpenRecipe {
            recipe: recipe.clone(),
            servings: None,
        })
        .await
        .expect("the recipe opens");
    let focus = state.focus.as_ref().expect("a recipe is open");
    let flour_usage = match &focus.recipe.components[0] {
        ComponentView::Ingredient { usage, .. } => usage.clone(),
        other => panic!("expected an ingredient line, got {other:?}"),
    };

    let mut edit = focus.edit.clone();
    edit.steps = vec![StepInput {
        segments: vec![
            SegmentInput::Text {
                text: "Mix ".into(),
            },
            SegmentInput::Ingredient {
                usage: flour_usage.clone(),
                display: RefDisplayTag::Full,
            },
            SegmentInput::Text {
                text: " with the eggs.".into(),
            },
        ],
    }];
    let state = app
        .dispatch(Command::SaveRecipe { recipe: edit })
        .await
        .expect("the steps are saved");

    // The recipe as written: 200 g.
    let focus = state.focus.as_ref().expect("still open");
    assert_eq!(focus.recipe.servings, 4);
    match &focus.recipe.steps[0].segments[1] {
        SegmentView::Ingredient { name, quantity, .. } => {
            assert_eq!(name.as_deref(), Some("Flour"));
            let quantity = quantity.as_ref().expect("a full reference shows both");
            assert_eq!(quantity.amount, "200");
            assert_eq!(quantity.unit, UnitTag::G);
        }
        other => panic!("expected an ingredient segment, got {other:?}"),
    }

    // Read at six: every quantity in the step re-renders scaled.
    let state = app
        .apply(Command::OpenRecipe {
            recipe: recipe.clone(),
            servings: Some(6),
        })
        .expect("the recipe reopens at six");
    let focus = state.focus.as_ref().expect("still open");
    match &focus.recipe.steps[0].segments[1] {
        SegmentView::Ingredient { quantity, .. } => {
            assert_eq!(quantity.as_ref().expect("shown").amount, "300")
        }
        other => panic!("expected an ingredient segment, got {other:?}"),
    }
    // Which recipe is open is device-local. It moves the screen, not the
    // document, so there is nothing to write — a phone does not save 154 kB
    // because somebody looked at a recipe (DECISIONS 0032, 0033).
    assert!(
        !app.persist().await.expect("a save that has nothing to do"),
        "opening a recipe must not dirty the replica"
    );

    // --- onto the list ----------------------------------------------------
    let state = app
        .dispatch(Command::AddRecipeToList {
            recipe: recipe.clone(),
            servings: Some(6),
        })
        .await
        .expect("the recipe goes on the list");

    let entry = state.list[0].id.clone();
    assert_eq!(state.list.len(), 1);
    match &state.list[0].item {
        ListItemView::Recipe {
            name,
            servings,
            written_for,
            ..
        } => {
            assert_eq!(name, "Tomato tart");
            assert_eq!((*servings, *written_for), (6, 4));
        }
        other => panic!("expected a recipe entry, got {other:?}"),
    }
    assert_eq!(state.list[0].added_by.as_deref(), Some("Alice"));

    // The cart, sorted by aisle: produce, then dairy, then grocery.
    assert_eq!(names(&state.cart.to_buy), ["Tomato", "Egg"]);
    // 3 tomatoes for four people is 4.5 for six — and a shop sells five
    // (DECISIONS 0016).
    assert_eq!(line(&state.cart.to_buy, "Tomato").amounts[0].amount, "5");
    assert_eq!(line(&state.cart.to_buy, "Egg").amounts[0].amount, "3");
    // Flour is a staple that only a recipe asked for: out of the way, but
    // still there (DECISIONS 0023).
    assert_eq!(names(&state.cart.at_home), ["Flour"]);
    assert_eq!(line(&state.cart.at_home, "Flour").amounts[0].amount, "300");
    assert_eq!((state.cart.remaining, state.cart.total), (2, 3));
    // The auto-checked staple already counts as settled.
    assert_eq!(
        (state.list[0].progress.settled, state.list[0].progress.total),
        (1, 3)
    );

    // --- "we are eight tonight" ------------------------------------------
    let state = app
        .dispatch(Command::SetEntryServings {
            entry: entry.clone(),
            servings: 8,
        })
        .await
        .expect("the entry rescales");
    assert_eq!(line(&state.cart.to_buy, "Tomato").amounts[0].amount, "6");
    let state = app
        .dispatch(Command::SetEntryServings {
            entry: entry.clone(),
            servings: 6,
        })
        .await
        .expect("and back");
    assert_eq!(line(&state.cart.to_buy, "Tomato").amounts[0].amount, "5");

    // --- unchecking a staple has to survive the next derivation -----------
    let flour = id_of_ingredient(&state, "Flour");
    let state = app
        .dispatch(Command::ToggleCartItem {
            ingredient: flour.clone(),
        })
        .await
        .expect("the staple is unchecked");
    assert_eq!(
        line(&state.cart.to_buy, "Flour").state,
        CheckStateTag::ToBuy
    );
    assert!(state.cart.at_home.is_empty());

    // Tick it off for real, so the next step has something to purge.
    let state = app
        .dispatch(Command::ToggleCartItem {
            ingredient: flour.clone(),
        })
        .await
        .expect("the staple is checked");
    assert_eq!(
        line(&state.cart.bought, "Flour").state,
        CheckStateTag::Checked
    );
    assert_eq!(
        line(&state.cart.bought, "Flour").checked_by.as_deref(),
        Some("Alice")
    );

    // --- adding by hand brings it back (Rule 3) ---------------------------
    let state = app
        .dispatch(Command::AddIngredientToList {
            ingredient: flour.clone(),
            quantity: Some(amount("1", UnitTag::Kg)),
        })
        .await
        .expect("flour goes on the list by hand");
    let bought = line(&state.cart.to_buy, "Flour");
    assert_eq!(bought.state, CheckStateTag::ToBuy, "the tick was purged");
    // 300 g from the recipe plus a kilo by hand, in one line a human reads.
    assert_eq!(bought.amounts.len(), 1);
    assert_eq!(bought.amounts[0].amount, "1.3");
    assert_eq!(bought.amounts[0].unit, UnitTag::Kg);
    assert_eq!(state.list.len(), 2);

    // --- the trip ---------------------------------------------------------
    let mut state = state;
    for name in ["Tomato", "Egg", "Flour"] {
        let ingredient = id_of_ingredient(&state, name);
        state = app
            .dispatch(Command::ToggleCartItem { ingredient })
            .await
            .expect("ticked off");
    }
    assert_eq!(state.cart.remaining, 0);
    assert!(state.list.iter().all(|entry| entry.progress.complete));

    let state = app
        .dispatch(Command::FinishShopping)
        .await
        .expect("the trip ends");
    assert!(state.list.is_empty(), "completed entries leave the list");
    assert!(state.cart.to_buy.is_empty() && state.cart.bought.is_empty());
    // The library is untouched by any of it.
    assert_eq!(state.recipes.len(), 1);
    assert_eq!(state.ingredients.len(), 3);

    // --- and it is all still there after a cold restart -------------------
    let reopened = open(storage).await;
    let state = reopened.state().expect("state");
    assert_eq!(state.recipes.len(), 1);
    assert_eq!(state.ingredients.len(), 3);
    assert!(state.list.is_empty());
    assert_eq!(state.me.as_ref().expect("a user is chosen").name, "Alice");
    // Steps and their references survived the round trip through the CRDT.
    assert_eq!(state.problems, Vec::new());
}

/// The concurrent case, seen from one device: the other one deleted a recipe
/// this list still points at.
async fn broken_reference() {
    let mut app = open(MemoryStorage::new()).await;
    let state = stocked(&mut app).await;
    let state = tart(&mut app, &state).await;
    let recipe = id_of_recipe(&state, "Tomato tart");

    let state = app
        .dispatch(Command::AddRecipeToList {
            recipe: recipe.clone(),
            servings: Some(4),
        })
        .await
        .expect("on the list");
    assert_eq!(state.cart.total, 3);

    // Deleting is never blocked by a reference to it (DECISIONS 0022).
    let state = app
        .dispatch(Command::DeleteRecipe {
            recipe: recipe.clone(),
        })
        .await
        .expect("the recipe is deleted");

    // The entry is still on the list, flagged rather than fatal, and the rest
    // of the screen still renders.
    assert_eq!(state.list.len(), 1);
    assert_eq!(state.cart.total, 0);
    assert_eq!(state.problems.len(), 1);
    assert_eq!(state.problems[0].kind, ProblemKind::MissingRecipe);
    assert_eq!(state.problems[0].subject.as_deref(), Some(recipe.as_str()));

    // An ingredient deleted underneath a recipe keeps its cart line, so the
    // rest of the trip still works.
    let mut app = open(MemoryStorage::new()).await;
    let state = stocked(&mut app).await;
    let state = tart(&mut app, &state).await;
    let recipe = id_of_recipe(&state, "Tomato tart");
    let state = app
        .dispatch(Command::AddRecipeToList {
            recipe,
            servings: None,
        })
        .await
        .expect("on the list");
    let tomato = id_of_ingredient(&state, "Tomato");
    let state = app
        .dispatch(Command::DeleteIngredient {
            ingredient: tomato.clone(),
        })
        .await
        .expect("the ingredient is deleted");

    assert_eq!(state.cart.total, 3, "the other two lines are unaffected");
    assert_eq!(state.problems[0].kind, ProblemKind::MissingIngredient);
    assert_eq!(state.problems[0].subject.as_deref(), Some(tomato.as_str()));
}

/// What the recipe editor actually does: name the lines, then write the prose
/// that points at them, and send both in one command (DECISIONS 0039).
///
/// The property under test is that a usage id minted by the host is an
/// ordinary usage id — the document keeps it verbatim, and a step referencing
/// it resolves like any other. Without this the editor would have to save a
/// half-finished recipe into the library just to learn what its own lines are
/// called.
async fn written_in_one_save() {
    let mut app = open(MemoryStorage::new()).await;
    let state = stocked(&mut app).await;

    // The host's own source, exactly as the PWA calls `CabasApp.mintUsageId`
    // while a form is open — the replica knows nothing about this line yet.
    let host = TestPlatform::default();
    let flour_usage = cabas_app::mint_usage_id(&host).expect("the host mints a usage id");

    let state = app
        .dispatch(Command::SaveRecipe {
            recipe: RecipeInput {
                id: None,
                name: "Pastry".into(),
                servings: 4,
                yields: Some(amount("500", UnitTag::G)),
                photo: None,
                components: vec![ComponentInput::Ingredient {
                    id: Some(flour_usage.clone()),
                    ingredient: id_of_ingredient(&state, "Flour"),
                    quantity: amount("250", UnitTag::G),
                }],
                steps: vec![StepInput {
                    segments: vec![
                        SegmentInput::Text {
                            text: "Sift ".into(),
                        },
                        SegmentInput::Ingredient {
                            usage: flour_usage.clone(),
                            display: RefDisplayTag::Full,
                        },
                    ],
                }],
            },
        })
        .await
        .expect("lines and steps are saved together");

    let recipe = id_of_recipe(&state, "Pastry");
    let state = app
        .apply(Command::OpenRecipe {
            recipe,
            servings: Some(8),
        })
        .expect("the recipe opens at eight");
    let focus = state.focus.as_ref().expect("a recipe is open");

    // The id the host chose is the one the line carries.
    match &focus.recipe.components[0] {
        ComponentView::Ingredient { usage, .. } => assert_eq!(usage, &flour_usage),
        other => panic!("expected an ingredient line, got {other:?}"),
    }

    // And the step resolved against it — scaled, because this is read at
    // double what it was written for, and never `Missing`.
    match &focus.recipe.steps[0].segments[1] {
        SegmentView::Ingredient { name, quantity, .. } => {
            assert_eq!(name.as_deref(), Some("Flour"));
            assert_eq!(quantity.as_ref().expect("a full reference").amount, "500");
        }
        other => panic!("expected a resolved reference, got {other:?}"),
    }
    assert_eq!(state.problems, Vec::new());
}

/// An ingredient the picker named before the document had heard of it.
///
/// The frontend mints the id so it can select what it creates the moment it
/// is created, and `SaveIngredient` cannot tell which side minted it
/// (DECISIONS 0056). Two halves of that contract are easy to break by
/// hardening the obvious way — refusing a supplied id that names nothing
/// looks like a sensible guard and would make every picker's first save fail:
///
/// - a supplied id that does not exist yet **creates**, under exactly that
///   id, and records no `Edited` — nothing was edited;
/// - the same id sent again **edits** that ingredient rather than adding a
///   second one, and does record it.
async fn created_under_a_host_minted_id() {
    let mut app = open(MemoryStorage::new()).await;

    // The host's own source, exactly as the PWA calls
    // `CabasApp.mintIngredientId` while the picker's panel is open.
    let host = TestPlatform::default();
    let chosen = cabas_app::mint_ingredient_id(&host).expect("the host mints an ingredient id");

    let mut input = new_ingredient("Tomato", AisleTag::Produce);
    input.id = Some(chosen.clone());
    let state = app
        .dispatch(Command::SaveIngredient {
            ingredient: input.clone(),
        })
        .await
        .expect("a supplied id that names nothing creates it");

    assert_eq!(id_of_ingredient(&state, "Tomato"), chosen);
    // Nothing was edited, so the journal says nothing. An entry here would
    // put "Alice modified Tomato" above the creation that never happened.
    assert_eq!(state.events, Vec::new());

    // The same id again is the edit path, not a second ingredient.
    input.name = "Tomato".into();
    input.staple = true;
    let state = app
        .dispatch(Command::SaveIngredient { ingredient: input })
        .await
        .expect("the same id edits");

    assert_eq!(state.ingredients.len(), 1);
    let ingredient = &state.ingredients[0];
    assert_eq!(ingredient.id, chosen);
    assert_eq!(ingredient.name, "Tomato");
    assert!(ingredient.staple);

    let event = state.events.first().expect("the edit is in the journal");
    assert_eq!(event.action, ActionTag::Edited);
    assert_eq!(event.subject, SubjectTag::Ingredient);
    assert_eq!(event.label, "Tomato");
}

/// Joining a group and then saying which member you are (DECISIONS 0068).
///
/// The window in the middle is the point: the device is open, the replica is
/// there, the roster is readable — and nothing has been signed, because
/// nobody has been chosen. A device record written before that would have had
/// to invent an owner, and the invented person is exactly what the roster
/// exists to avoid.
async fn joining_and_saying_who_you_are() {
    let platform = TestPlatform::default();
    let joined = Identity::mint_device(&platform, "Alice's iPhone").expect("a device identity");
    let mut app = App::open(MemoryStorage::new(), TestPlatform::default(), joined)
        .await
        .expect("the app opens");

    let state = StateView::clone(&app.state().expect("state"));
    assert_eq!(state.me, None, "nobody has been chosen yet");
    assert_eq!(state.people, Vec::new(), "and nobody has been invented");

    // Nothing attributable may be written in that window. Creating an
    // ingredient is not attributed and does go through — the refusal is about
    // signing something, not about being read-only.
    let state = app
        .dispatch(Command::SaveIngredient {
            ingredient: new_ingredient("Flour", AisleTag::Grocery),
        })
        .await
        .expect("the library is writable before anybody is named");
    let flour = id_of_ingredient(&state, "Flour");
    assert_eq!(
        app.dispatch(Command::AddIngredientToList {
            ingredient: flour.clone(),
            quantity: None,
        })
        .await,
        Err(cabas_app::AppError::NoUser),
        "a list entry carries who added it, so it has to wait"
    );

    // The device is named before anybody is chosen, so its record is written
    // once and already carries the name.
    app.dispatch(Command::NameDevice {
        name: "Le téléphone d'Alice".into(),
    })
    .await
    .expect("the device is named");

    // The roster is empty, so the only way through is to add yourself.
    let state = app
        .dispatch(Command::CreateUser {
            name: "Alice".into(),
        })
        .await
        .expect("a person is created");
    assert_eq!(state.people[0].devices[0].name, "Le téléphone d'Alice");
    assert_eq!(state.me.as_ref().expect("chosen").name, "Alice");
    let alice = state.me.as_ref().expect("chosen").id.clone();
    assert_eq!(state.people.len(), 1);
    assert!(state.people[0].is_me);
    assert_eq!(state.people[0].devices.len(), 1);
    assert!(state.people[0].devices[0].is_this_one);
    let joined_at = state.people[0].devices[0].paired_at;

    // And now it can be signed.
    let state = app
        .dispatch(Command::AddIngredientToList {
            ingredient: flour,
            quantity: None,
        })
        .await
        .expect("the same command, once there is somebody to attribute it to");
    assert_eq!(state.list.len(), 1);

    // The phone changes hands. The device is one record: it moves to Bob
    // rather than being duplicated, and it does not claim to have joined the
    // group again today.
    let state = app
        .dispatch(Command::CreateUser { name: "Bob".into() })
        .await
        .expect("a second person");
    assert_eq!(state.people.len(), 2);
    let bob = state.me.as_ref().expect("chosen").id.clone();
    assert_ne!(bob, alice);
    let carried: Vec<_> = state
        .people
        .iter()
        .map(|person| (person.name.as_str(), person.devices.len()))
        .collect();
    assert_eq!(carried, vec![("Alice", 0), ("Bob", 1)]);
    let bob_view = state.people.iter().find(|p| p.id == bob).expect("Bob");
    assert_eq!(bob_view.devices[0].paired_at, joined_at);

    // What Alice already added stays hers: attribution records who did
    // something, and handing over a phone does not rewrite the past (Rule 7).
    assert_eq!(state.list[0].added_by.as_deref(), Some("Alice"));

    // And back, by picking a name off the roster rather than typing one.
    let state = app
        .dispatch(Command::ChooseUser {
            user: alice.clone(),
        })
        .await
        .expect("an existing member is chosen");
    assert_eq!(state.me.as_ref().expect("chosen").name, "Alice");
    assert_eq!(state.people.len(), 2, "choosing creates nobody");

    // A row that is not on the roster is refused rather than invented: the
    // frontend was given the list it is allowed to send back.
    assert!(matches!(
        app.dispatch(Command::ChooseUser {
            user: "usr_nobody".into(),
        })
        .await,
        Err(cabas_app::AppError::NotFound { kind: "user", .. })
    ));
}

/// Adding an ingredient to the list without saying how much (DECISIONS
/// 0066) — what a swiped row does, since a gesture carries no amount.
///
/// The rule is the core's, not the frontend's (Rule 9): a sized ingredient
/// contributes what it was sized at, and an unsized one contributes one
/// piece, which is a thing a person can pick up and correct.
async fn added_without_an_amount() {
    let mut app = open(MemoryStorage::new()).await;

    let mut sized = new_ingredient("Flour", AisleTag::Grocery);
    sized.default_quantity = Some(amount("1", UnitTag::Kg));
    let state = app
        .dispatch(Command::SaveIngredient { ingredient: sized })
        .await
        .expect("save");
    let flour = id_of_ingredient(&state, "Flour");

    let state = app
        .dispatch(Command::SaveIngredient {
            ingredient: new_ingredient("Tomato", AisleTag::Produce),
        })
        .await
        .expect("save");
    let tomatoes = id_of_ingredient(&state, "Tomato");

    // The sized one round-trips through the view as the form would show it:
    // lossless, so an edit does not write back a rounded amount.
    let stored = state
        .ingredients
        .iter()
        .find(|i| i.id == flour)
        .expect("flour");
    assert_eq!(stored.default_quantity, Some(amount("1", UnitTag::Kg)));
    // And an unsized one says "nobody said", which is not one piece.
    let stored = state
        .ingredients
        .iter()
        .find(|i| i.id == tomatoes)
        .expect("tomatoes");
    assert_eq!(stored.default_quantity, None);

    let _ = app
        .dispatch(Command::AddIngredientToList {
            ingredient: flour,
            quantity: None,
        })
        .await
        .expect("a swipe adds without an amount");
    let state = app
        .dispatch(Command::AddIngredientToList {
            ingredient: tomatoes,
            quantity: None,
        })
        .await
        .expect("so does one on an ingredient nobody has sized");

    let flour_line = line(&state.cart.to_buy, "Flour");
    assert_eq!(flour_line.amounts.len(), 1);
    assert_eq!(flour_line.amounts[0].amount, "1");
    assert_eq!(flour_line.amounts[0].unit, UnitTag::Kg);

    let tomato_line = line(&state.cart.to_buy, "Tomato");
    assert_eq!(tomato_line.amounts.len(), 1);
    assert_eq!(tomato_line.amounts[0].amount, "1");
    assert_eq!(tomato_line.amounts[0].unit, UnitTag::Piece);

    // A stated amount still wins: the option is a default, not a policy.
    let state = app
        .dispatch(Command::AddIngredientToList {
            ingredient: id_of_ingredient(&state, "Flour"),
            quantity: Some(amount("500", UnitTag::G)),
        })
        .await
        .expect("an amount that was stated");
    let flour_line = line(&state.cart.to_buy, "Flour");
    assert_eq!(flour_line.amounts[0].amount, "1 1/2");
    assert_eq!(flour_line.amounts[0].unit, UnitTag::Kg);
}

/// A photo, from the bytes to the aisle it is meant to be recognised in
/// (DECISIONS 0062).
///
/// Two steps on purpose, and the test is where that shows: the bytes go to a
/// store of their own on a call that is awaited, and the id then rides on the
/// save that would have happened anyway. Nothing about the photo makes
/// `apply` asynchronous.
async fn a_photo_from_the_bytes_to_the_cart() {
    let mut app = open(MemoryStorage::new()).await;
    let photos = Photos::new(MemoryPhotoStore::new());
    let state = stocked(&mut app).await;

    // A JPEG, as far as anything here is concerned.
    let bytes = [0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46];
    let id = photos
        .put(&TestPlatform::default(), &bytes)
        .await
        .expect("the photo is stored");

    let tomatoes = id_of_ingredient(&state, "Tomato");
    let mut input = new_ingredient("Tomato", AisleTag::Produce);
    input.id = Some(tomatoes.clone());
    input.unit_weight = Some("150".into());
    input.photo = Some(id.to_string());
    let state = app
        .dispatch(Command::SaveIngredient { ingredient: input })
        .await
        .expect("the photo is attached");

    // The document names it, on the shelf and on the line a person reads in
    // an aisle. It never carries the bytes.
    let shelf = state
        .ingredients
        .iter()
        .find(|i| i.id == tomatoes)
        .expect("the ingredient");
    assert_eq!(shelf.photo.as_deref(), Some(id.as_str()));

    let state = app
        .dispatch(Command::AddIngredientToList {
            ingredient: tomatoes.clone(),
            quantity: Some(amount("3", UnitTag::Piece)),
        })
        .await
        .expect("on the list");
    assert_eq!(
        line(&state.cart.to_buy, "Tomato").photo.as_deref(),
        Some(id.as_str())
    );

    // And the bytes are where the document is not.
    assert_eq!(photos.get(&id).await.expect("read"), Some(bytes.to_vec()));

    // A photo the replica references and this device has not got is what a
    // second phone sees the instant the two merge: named, absent, and not an
    // error (Rule 6).
    let elsewhere = Photos::new(MemoryPhotoStore::new());
    assert_eq!(
        elsewhere
            .missing(&app.referenced_photos().expect("referenced"))
            .await
            .expect("missing"),
        vec![id.clone()]
    );

    // Detaching it is the same ordinary save. The bytes stay put: only a
    // sweep deletes them, and only once the relay has its own copy.
    let mut input = new_ingredient("Tomato", AisleTag::Produce);
    input.id = Some(tomatoes.clone());
    input.unit_weight = Some("150".into());
    let state = app
        .dispatch(Command::SaveIngredient { ingredient: input })
        .await
        .expect("the photo is detached");
    assert_eq!(line(&state.cart.to_buy, "Tomato").photo, None);
    assert!(app.referenced_photos().expect("referenced").is_empty());

    let forgotten = photos
        .forget_unreferenced(&app.referenced_photos().expect("referenced"))
        .await
        .expect("sweep");
    assert_eq!(forgotten, vec![id.clone()]);
    assert_eq!(photos.get(&id).await.expect("read"), None);
}

#[cfg(not(target_family = "wasm"))]
mod native {
    use super::*;

    /// Drives a future to completion without an async runtime. Every backend
    /// used here completes without ever yielding, so a no-op waker is enough
    /// — and it keeps `tokio` out of this crate for the sake of two tests.
    fn block_on<F: std::future::Future>(future: F) -> F::Output {
        use std::pin::pin;
        use std::task::{Context, Poll, Waker};

        let mut future = pin!(future);
        let mut context = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
                return value;
            }
        }
    }

    #[test]
    fn a_shopping_trip_from_an_empty_library_to_a_finished_cart() {
        block_on(scenario());
    }

    #[test]
    fn a_deleted_recipe_or_ingredient_degrades_to_a_warning() {
        block_on(broken_reference());
    }

    #[test]
    fn a_recipe_is_writable_in_a_single_save() {
        block_on(written_in_one_save());
    }

    #[test]
    fn an_ingredient_is_creatable_under_an_id_the_host_minted() {
        block_on(created_under_a_host_minted_id());
    }

    #[test]
    fn a_photo_reaches_the_aisle_it_is_for() {
        block_on(a_photo_from_the_bytes_to_the_cart());
    }

    #[test]
    fn an_ingredient_added_without_an_amount_uses_its_own_default() {
        block_on(added_without_an_amount());
    }

    #[test]
    fn a_device_joins_a_group_and_then_says_who_carries_it() {
        block_on(joining_and_saying_who_you_are());
    }
}

#[cfg(target_family = "wasm")]
mod browser {
    use super::*;
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn a_shopping_trip_from_an_empty_library_to_a_finished_cart() {
        scenario().await;
    }

    #[wasm_bindgen_test]
    async fn a_deleted_recipe_or_ingredient_degrades_to_a_warning() {
        broken_reference().await;
    }

    #[wasm_bindgen_test]
    async fn a_recipe_is_writable_in_a_single_save() {
        written_in_one_save().await;
    }

    #[wasm_bindgen_test]
    async fn an_ingredient_is_creatable_under_an_id_the_host_minted() {
        created_under_a_host_minted_id().await;
    }

    #[wasm_bindgen_test]
    async fn a_photo_reaches_the_aisle_it_is_for() {
        a_photo_from_the_bytes_to_the_cart().await;
    }

    #[wasm_bindgen_test]
    async fn an_ingredient_added_without_an_amount_uses_its_own_default() {
        added_without_an_amount().await;
    }

    #[wasm_bindgen_test]
    async fn a_device_joins_a_group_and_then_says_who_carries_it() {
        joining_and_saying_who_you_are().await;
    }

    /// The PWA's actual path: a command built as a JS object, through the
    /// exported binding, into IndexedDB and back out as a state object.
    ///
    /// Everything above this test uses the Rust API directly. This one is the
    /// only thing that exercises what M4 will actually call — the JS boundary
    /// where a serde mismatch, a missing `null`, or a borrow held across an
    /// await turns into a blank page rather than a failing assertion.
    #[wasm_bindgen_test]
    async fn the_binding_layer_round_trips_a_command_and_a_state() {
        use cabas_app::CabasApp;
        use cabas_app::view::StateView;

        let identity =
            CabasApp::mint_device("Alice's iPhone".into()).expect("a device identity is minted");
        let app = CabasApp::open(identity).await.expect("the app opens");

        // A device that has not said who carries it yet (DECISIONS 0068).
        // Through the binding, because `me` being `null` rather than
        // `undefined` is the whole of 0034 and this is where it would break.
        let opened: StateView =
            serde_wasm_bindgen::from_value(app.state().expect("state")).expect("a state");
        assert!(opened.me.is_none(), "nobody has been chosen yet");

        let named = serde_wasm_bindgen::to_value(&Command::CreateUser {
            name: "Alice".into(),
        })
        .expect("the command becomes a JS object");
        let state: StateView =
            serde_wasm_bindgen::from_value(app.apply(named).expect("the user is created"))
                .expect("a state");
        assert_eq!(state.me.expect("a user is chosen").name, "Alice");

        // And the host can read the identity back to persist it.
        let identity: cabas_app::Identity =
            serde_wasm_bindgen::from_value(app.identity().expect("identity")).expect("an identity");
        assert_eq!(identity.user_name.as_deref(), Some("Alice"));

        let command = serde_wasm_bindgen::to_value(&Command::SaveIngredient {
            ingredient: new_ingredient("Saffron", AisleTag::Grocery),
        })
        .expect("the command becomes a JS object");
        let state: StateView =
            serde_wasm_bindgen::from_value(app.apply(command).expect("the command is applied"))
                .expect("the state comes back as a JS object");

        assert!(state.ingredients.iter().any(|i| i.name == "Saffron"));

        // `None` must arrive as `null`, not `undefined` — the generated
        // TypeScript says `FocusView | null`, and a UI written against it
        // would otherwise be testing something that never happens.
        let raw = app.state().expect("state");
        let focus = js_sys::Reflect::get(&raw, &wasm_bindgen::JsValue::from_str("focus"))
            .expect("the state has a focus property");
        assert!(focus.is_null(), "an absent focus must be null: {focus:?}");

        assert!(app.flush().await.expect("the write succeeds"));
        // Nothing changed since, so the second call has nothing to write.
        assert!(!app.flush().await.expect("no second write"));
    }

    /// The sync half of the same boundary: a sealed frame in, a state out,
    /// and a cursor the engine can put in `localStorage`.
    ///
    /// The composition itself is tested natively in `sync.rs`. What only a
    /// browser can answer is what these calls *become* on the JS side — and
    /// the answer that matters is `number`. A `u64` serialised as a `BigInt`
    /// would look right in every Rust assertion and throw in
    /// `JSON.stringify`, which is where the cursor is headed (DECISIONS 0043).
    #[wasm_bindgen_test]
    async fn the_binding_layer_opens_a_frame_and_reports_a_cursor() {
        use cabas_app::CabasApp;
        use cabas_app::sync::{SyncCursor, SyncEvent, SyncSession};
        use cabas_sync::protocol::{ClientMessage, ServerMessage, decode_client, encode_server};
        use wasm_bindgen::JsValue;

        let phrase = CabasApp::mint_phrase().expect("a phrase is minted");

        // The other device, over its own storage: a real replica, sealed by a
        // real session, so what arrives below is a frame and not a fixture.
        let mut sender = open(MemoryStorage::new()).await;
        sender
            .dispatch(Command::SaveIngredient {
                ingredient: new_ingredient("Cardamome", AisleTag::Grocery),
            })
            .await
            .expect("the other device saves an ingredient");
        let sending = SyncSession::open(&phrase, SyncCursor::default()).expect("a session opens");
        let push = sending.push(&sender, &[]).expect("the delta seals");
        let frame = match decode_client(&push).expect("a client message") {
            ClientMessage::Push { kind, payload } => encode_server(&ServerMessage::Frame {
                seq: 12,
                kind,
                payload,
            })
            .expect("the relay's frame encodes"),
            other => panic!("expected a push, got {other:?}"),
        };

        let identity =
            CabasApp::mint_device("Bob's iPhone".into()).expect("a device identity is minted");
        let app = CabasApp::open(identity).await.expect("the app opens");

        assert!(
            app.sync_status().expect("status").is_null(),
            "no connection, no status"
        );
        // A real relay mints its epoch from 64 bits of randomness, so this is
        // the ordinary case and not an edge one: a `u64` above 2^53 has no
        // exact `Number`. It crosses as text, and a build that changes that
        // fails here rather than in front of a phone — which is where it
        // failed the first time.
        let epoch = 15_150_583_726_198_229_639;
        let cursor = serde_wasm_bindgen::to_value(&SyncCursor { epoch, since: 0 })
            .expect("the cursor becomes a JS object");
        assert!(
            !app.sync_hello(&phrase, cursor)
                .expect("the hello is produced")
                .is_empty()
        );

        let event: SyncEvent =
            serde_wasm_bindgen::from_value(app.sync_handle(&frame).expect("the frame is handled"))
                .expect("the event comes back as a JS object");
        match event {
            SyncEvent::Merged { state } => {
                assert!(state.ingredients.iter().any(|i| i.name == "Cardamome"));
            }
            other => panic!("expected a merge, got {other:?}"),
        }

        // The shape the engine persists, read the way JS reads it.
        let status = app.sync_status().expect("status");
        let cursor = js_sys::Reflect::get(&status, &JsValue::from_str("cursor"))
            .expect("the status has a cursor");
        let since = js_sys::Reflect::get(&cursor, &JsValue::from_str("since"))
            .expect("the cursor has a sequence");
        assert_eq!(
            since.as_f64(),
            Some(12.0),
            "the cursor followed the frame, as a JS number: {since:?}"
        );
        let epoch_out = js_sys::Reflect::get(&cursor, &JsValue::from_str("epoch"))
            .expect("the cursor has an epoch");
        assert_eq!(
            epoch_out.as_string().as_deref(),
            Some(epoch.to_string().as_str()),
            "the epoch survived the round trip as text: {epoch_out:?}"
        );

        assert!(!app.sync_version().is_empty(), "the shadow to remember");
        app.sync_close();
        assert!(app.sync_status().expect("status").is_null());
    }
}
