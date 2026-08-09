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
use cabas_app::tags::{AisleTag, CheckStateTag, RefDisplayTag, UnitTag};
use cabas_app::view::{
    CartLineView, ComponentView, ListItemView, ProblemKind, SegmentView, StateView,
};
use cabas_app::{App, Command, Identity, Platform};
use cabas_domain::Timestamp;
use cabas_store::MemoryStorage;

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
        user: "usr_alice".into(),
        user_name: "Alice".into(),
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
    assert_eq!(state.me.name, "Alice");

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
            servings: 6,
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
            quantity: amount("1", UnitTag::Kg),
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
    assert_eq!(state.me.name, "Alice");
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
            servings: 4,
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
            servings: 4,
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

        let identity = CabasApp::mint_identity("Alice".into(), "Alice's iPhone".into())
            .expect("an identity is minted");
        let app = CabasApp::open(identity).await.expect("the app opens");

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

        let identity = CabasApp::mint_identity("Bob".into(), "Bob's iPhone".into())
            .expect("an identity is minted");
        let app = CabasApp::open(identity).await.expect("the app opens");

        assert!(
            app.sync_status().expect("status").is_null(),
            "no connection, no status"
        );
        let cursor = serde_wasm_bindgen::to_value(&SyncCursor::default())
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

        assert!(!app.sync_version().is_empty(), "the shadow to remember");
        app.sync_close();
        assert!(app.sync_status().expect("status").is_null());
    }
}
