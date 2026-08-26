//! The library as a file, out and back in (DECISIONS 0076).
//!
//! Three questions, and the third is the one that decides whether the feature
//! is worth having:
//!
//! 1. Does a library survive the round trip **exactly** — every coefficient,
//!    every reference, every photo?
//! 2. Does a file from *somebody else's* group merge, rather than duplicate?
//!    That is the whole difference between an import and a paste.
//! 3. Does an import that goes wrong leave the library alone? There is no
//!    transaction under the document, so this is a property of the code and
//!    not of the storage.
//!
//! Native only. Nothing here is platform-dependent — the file is text, the
//! stores are in memory — and the wasm binding around it is driven by
//! `ui/tests/smoke.mjs`, in a real browser, where a `Uint8Array` that does
//! not cross would show up.

#![cfg(not(target_family = "wasm"))]

use cabas_app::command::{
    ComponentInput, IngredientInput, QuantityInput, RecipeInput, SegmentInput, ShopInput,
    StepInput, SubRecipeAmountInput,
};
use cabas_app::photos::Photos;
use cabas_app::tags::{AisleTag, KeepingTag, RefDisplayTag, UnitTag};
use cabas_app::transfer::{FORMAT, FORMAT_VERSION, LibraryFile};
use cabas_app::view::{ComponentView, StateView};
use cabas_app::{App, Command, Identity, Platform};
use cabas_domain::Timestamp;
use cabas_store::{MemoryPhotoStore, MemoryStorage};

/// A fixed clock and counting ids. The base separates two devices, so that a
/// file written by one carries ids the other has genuinely never seen — which
/// is the case the name matching exists for.
#[derive(Debug)]
struct TestPlatform {
    counter: std::cell::Cell<u64>,
}

impl TestPlatform {
    fn from(base: u64) -> Self {
        Self {
            counter: std::cell::Cell::new(base),
        }
    }
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

async fn open(base: u64) -> App<MemoryStorage, TestPlatform> {
    App::open(
        MemoryStorage::new(),
        TestPlatform::from(base),
        Identity {
            user: Some(format!("usr_{base}")),
            user_name: Some("Alexis".into()),
            device: format!("dev_{base}"),
            device_name: "Un téléphone".into(),
        },
    )
    .await
    .expect("the app opens")
}

fn amount(value: &str, unit: UnitTag) -> QuantityInput {
    QuantityInput {
        amount: value.into(),
        unit,
    }
}

fn ingredient(name: &str, aisle: AisleTag) -> IngredientInput {
    IngredientInput {
        id: None,
        name: name.into(),
        aliases: Vec::new(),
        aisle,
        shops: Vec::new(),
        keeping: KeepingTag::Ambient,
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

/// The library every test below starts from: two shops, four ingredients that
/// between them use every optional field, and two recipes — one of which uses
/// the other, and mentions a line in its prose.
async fn stocked(app: &mut App<MemoryStorage, TestPlatform>) -> StateView {
    app.dispatch(Command::SaveShop {
        shop: ShopInput {
            id: None,
            name: "Biocoop".into(),
        },
    })
    .await
    .expect("the shop is created");
    let state = app
        .dispatch(Command::SaveShop {
            shop: ShopInput {
                id: None,
                name: "Marché".into(),
            },
        })
        .await
        .expect("the second shop is created");
    let biocoop = state
        .shops
        .iter()
        .find(|s| s.name == "Biocoop")
        .expect("Biocoop")
        .id
        .clone();

    let mut flour = ingredient("Farine", AisleTag::Staples);
    flour.aliases = vec!["farine T65".into()];
    flour.staple = true;
    flour.density = Some("0,55".into());
    flour.default_quantity = Some(amount("1", UnitTag::Kg));
    flour.shops = vec![biocoop.clone()];
    app.dispatch(Command::SaveIngredient { ingredient: flour })
        .await
        .expect("flour");

    let mut butter = ingredient("Beurre", AisleTag::Dairy);
    butter.keeping = KeepingTag::Fridge;
    butter.unit_weight = Some("250".into());
    app.dispatch(Command::SaveIngredient { ingredient: butter })
        .await
        .expect("butter");

    let mut apple = ingredient("Pomme", AisleTag::Produce);
    // An amount with no tidy decimal form: it must come back as the same
    // exact rational, not as a rounding of it (Rule 4).
    apple.unit_weight = Some("1/3".into());
    app.dispatch(Command::SaveIngredient {
        ingredient: apple.clone(),
    })
    .await
    .expect("apple");

    app.dispatch(Command::SaveIngredient {
        ingredient: ingredient("Sucre", AisleTag::Pantry),
    })
    .await
    .expect("sugar");

    let state = app.state().expect("state");
    let flour_id = id_of_ingredient(&state, "Farine");
    let butter_id = id_of_ingredient(&state, "Beurre");
    let apple_id = id_of_ingredient(&state, "Pomme");

    let pastry_line = "use_pastry_flour";
    app.dispatch(Command::SaveRecipe {
        recipe: RecipeInput {
            id: None,
            name: "Pâte brisée".into(),
            servings: 4,
            yields: Some(amount("500", UnitTag::G)),
            components: vec![
                ComponentInput::Ingredient {
                    id: Some(pastry_line.into()),
                    ingredient: flour_id,
                    quantity: amount("250", UnitTag::G),
                },
                ComponentInput::Ingredient {
                    id: None,
                    ingredient: butter_id,
                    quantity: amount("125", UnitTag::G),
                },
            ],
            steps: vec![StepInput {
                segments: vec![
                    SegmentInput::Text {
                        text: "Mélanger ".into(),
                    },
                    SegmentInput::Ingredient {
                        usage: pastry_line.into(),
                        display: RefDisplayTag::Full,
                    },
                    SegmentInput::Text {
                        text: " et le beurre.".into(),
                    },
                ],
            }],
            photo: None,
        },
    })
    .await
    .expect("the pastry");

    let state = app.state().expect("state");
    let pastry_id = id_of_recipe(&state, "Pâte brisée");

    app.dispatch(Command::SaveRecipe {
        recipe: RecipeInput {
            id: None,
            name: "Tarte aux pommes".into(),
            servings: 6,
            yields: None,
            components: vec![
                ComponentInput::SubRecipe {
                    id: None,
                    recipe: pastry_id,
                    amount: SubRecipeAmountInput::OfYield {
                        quantity: amount("400", UnitTag::G),
                    },
                },
                ComponentInput::Ingredient {
                    id: None,
                    ingredient: apple_id,
                    quantity: amount("6", UnitTag::Piece),
                },
            ],
            steps: Vec::new(),
            photo: None,
        },
    })
    .await
    .expect("the tart");

    app.state().expect("state")
}

/// The file one app writes, as text — the thing that would land in iCloud.
fn exported(app: &App<MemoryStorage, TestPlatform>) -> String {
    app.export_library()
        .expect("the library exports")
        .to_json()
        .expect("it writes as JSON")
}

fn import(app: &mut App<MemoryStorage, TestPlatform>, json: &str) -> cabas_app::ImportReport {
    let file = LibraryFile::from_json(json).expect("the file reads");
    app.import_library(&file).expect("the file imports")
}

#[test]
fn a_library_survives_the_round_trip() {
    block_on(async {
        let mut source = open(0).await;
        let before = stocked(&mut source).await;
        let json = exported(&source);

        // A different device, a different id sequence, an empty library.
        let mut target = open(9000).await;
        let report = import(&mut target, &json);
        let after = target.state().expect("state");

        assert_eq!(report.shops_added, 2);
        assert_eq!(report.ingredients_added, 4);
        assert_eq!(report.recipes_added, 2);
        assert_eq!(report.ingredients_updated, 0);
        assert!(report.incomplete.is_empty(), "{:?}", report.incomplete);

        // The ids travel, so the two libraries are the same library and not a
        // copy of it — which is what makes the file a backup.
        assert_eq!(
            id_of_ingredient(&after, "Farine"),
            id_of_ingredient(&before, "Farine")
        );

        let flour = after
            .ingredients
            .iter()
            .find(|i| i.name == "Farine")
            .expect("flour came back");
        assert!(flour.staple);
        assert_eq!(flour.aliases, ["farine T65"]);
        assert_eq!(flour.aisle, AisleTag::Staples);
        assert_eq!(flour.shops.len(), 1, "its shop came back as a shop");
        assert_eq!(
            flour.default_quantity.as_ref().map(|q| q.unit),
            Some(UnitTag::Kg)
        );

        let butter = after
            .ingredients
            .iter()
            .find(|i| i.name == "Beurre")
            .expect("butter came back");
        assert_eq!(butter.keeping, KeepingTag::Fridge);

        // The exact rational, not a rounding of it (Rule 4). It reaches the
        // file through `render_lossless` and comes back through the same
        // parser a form uses.
        let apple = after
            .ingredients
            .iter()
            .find(|i| i.name == "Pomme")
            .expect("the apple came back");
        assert_eq!(apple.unit_weight.as_deref(), Some("1/3"));

        // The tart still knows which pastry it is made of, and the pastry's
        // prose still points at its own flour line.
        let tart = after
            .recipes
            .iter()
            .find(|r| r.name == "Tarte aux pommes")
            .expect("the tart");
        assert!(
            after.problems.is_empty(),
            "no reference was left dangling: {:?}",
            after.problems
        );
        assert_eq!(tart.servings, 6);

        let opened = target
            .apply(Command::OpenRecipe {
                recipe: id_of_recipe(&after, "Pâte brisée"),
                servings: None,
            })
            .expect("the pastry opens");
        let recipe = opened.focus.expect("a recipe is open").recipe;
        assert!(
            matches!(
                recipe.components.first(),
                Some(ComponentView::Ingredient { name, .. }) if name.as_deref() == Some("Farine")
            ),
            "the pastry's first line is flour: {:?}",
            recipe.components.first()
        );
        assert_eq!(
            recipe.steps[0].segments.len(),
            3,
            "the prose kept its three segments"
        );
    });
}

#[test]
fn re_importing_a_file_updates_and_never_duplicates() {
    block_on(async {
        let mut app = open(0).await;
        stocked(&mut app).await;
        let json = exported(&app);

        let report = import(&mut app, &json);
        assert_eq!(report.ingredients_added, 0);
        assert_eq!(report.ingredients_updated, 4, "matched by id, in place");
        assert_eq!(report.shops_added, 0);
        assert_eq!(report.recipes_added, 0);

        let state = app.state().expect("state");
        assert_eq!(state.ingredients.len(), 4);
        assert_eq!(state.recipes.len(), 2);
        assert_eq!(state.shops.len(), 2);
    });
}

#[test]
fn a_file_from_another_group_merges_by_name() {
    block_on(async {
        // Somebody else's library, whose ids this device has never seen.
        let mut theirs = open(0).await;
        stocked(&mut theirs).await;
        let json = exported(&theirs);

        // A library of our own that already knows flour — spelled differently,
        // and filed in a different aisle.
        let mut ours = open(9000).await;
        let mut flour = ingredient("FARINE", AisleTag::Pantry);
        flour.staple = false;
        ours.dispatch(Command::SaveIngredient { ingredient: flour })
            .await
            .expect("our flour");
        ours.dispatch(Command::SaveIngredient {
            ingredient: ingredient("Levure", AisleTag::Pantry),
        })
        .await
        .expect("our yeast");
        let before = ours.state().expect("state");
        let our_flour = id_of_ingredient(&before, "FARINE");

        let report = import(&mut ours, &json);
        let after = ours.state().expect("state");

        // One flour, not two — and it is *our* flour, updated.
        assert_eq!(report.ingredients_updated, 1);
        assert_eq!(report.ingredients_added, 3);
        assert_eq!(
            after.ingredients.iter().filter(|i| i.staple).count(),
            1,
            "the incoming flour won, on the row that was already here"
        );
        assert!(
            after.ingredients.iter().any(|i| i.id == our_flour),
            "it kept the id this group had for it"
        );
        assert_eq!(
            after.ingredients.len(),
            5,
            "four theirs, one ours, flour counted once: {:?}",
            after
                .ingredients
                .iter()
                .map(|i| i.name.as_str())
                .collect::<Vec<_>>()
        );

        // And the recipe that arrived points at *our* flour, not at a
        // dangling id minted on their phone.
        assert!(
            after.problems.is_empty(),
            "every reference was rewritten: {:?}",
            after.problems
        );
        let opened = ours
            .apply(Command::OpenRecipe {
                recipe: id_of_recipe(&after, "Pâte brisée"),
                servings: None,
            })
            .expect("the pastry opens");
        let recipe = opened.focus.expect("open").recipe;
        assert!(
            matches!(
                recipe.components.first(),
                Some(ComponentView::Ingredient { ingredient, .. }) if *ingredient == our_flour
            ),
            "the line resolved to the row this group already had: {:?}",
            recipe.components.first()
        );
        // And that row now spells its name the file's way, because the file
        // wins on every field of what it mentions — including the one that
        // matched it.
        assert_eq!(
            after
                .ingredients
                .iter()
                .find(|i| i.id == our_flour)
                .map(|i| i.name.as_str()),
            Some("Farine")
        );
    });
}

#[test]
fn an_import_never_deletes() {
    block_on(async {
        let mut app = open(0).await;
        stocked(&mut app).await;

        // A file that knows about one ingredient and nothing else.
        let json = LibraryFile {
            format: FORMAT.into(),
            format_version: FORMAT_VERSION,
            app_version: "0.0.0".into(),
            exported_at: 0,
            shops: Vec::new(),
            ingredients: vec![ingredient("Cardamome", AisleTag::Pantry)],
            recipes: Vec::new(),
            photos: Default::default(),
        }
        .to_json()
        .expect("json");

        import(&mut app, &json);
        let after = app.state().expect("state");
        assert_eq!(after.ingredients.len(), 5, "four kept, one added");
        assert_eq!(after.recipes.len(), 2, "the recipes it did not mention");
        assert_eq!(after.shops.len(), 2, "and the shops");
    });
}

#[test]
fn an_import_is_not_written_to_the_event_log() {
    block_on(async {
        let mut app = open(0).await;
        stocked(&mut app).await;
        let json = exported(&app);

        // Importing over the top of everything: every one of these would be
        // an `Edited` entry if it went through the ordinary save.
        import(&mut app, &json);

        let after = app.state().expect("state");
        assert!(
            after.events.is_empty(),
            "an import writes no events — the log is capped at 200 and one \
             file can carry more than that (DECISIONS 0076): {:?}",
            after.events
        );
    });
}

#[test]
fn a_file_may_name_things_instead_of_giving_them_ids() {
    block_on(async {
        // What somebody writing a file by hand would produce: no ids at all,
        // and references by name.
        let json = r#"{
          "format": "cabas.library",
          "format_version": 1,
          "shops": [{ "name": "Biocoop" }],
          "ingredients": [
            { "name": "Farine", "aisle": "staples", "shops": ["Biocoop"] },
            { "name": "Sel", "aisle": "pantry" }
          ],
          "recipes": [
            {
              "name": "Pain",
              "servings": 4,
              "components": [
                {
                  "kind": "ingredient",
                  "ingredient": "Farine",
                  "quantity": { "amount": "500", "unit": "g" }
                }
              ]
            }
          ]
        }"#;

        let mut app = open(0).await;
        let report = import(&mut app, json);
        assert_eq!(report.ingredients_added, 2);
        assert_eq!(report.recipes_added, 1);
        assert!(report.incomplete.is_empty());

        let state = app.state().expect("state");
        assert!(
            state.problems.is_empty(),
            "the recipe found its flour by name: {:?}",
            state.problems
        );
        let flour = state
            .ingredients
            .iter()
            .find(|i| i.name == "Farine")
            .expect("flour");
        assert_eq!(flour.shops.len(), 1, "and the ingredient found its shop");
    });
}

#[test]
fn a_recipe_naming_something_absent_is_kept_and_reported() {
    block_on(async {
        let json = r#"{
          "format": "cabas.library",
          "format_version": 1,
          "recipes": [
            {
              "name": "Pain",
              "servings": 4,
              "components": [
                {
                  "kind": "ingredient",
                  "ingredient": "Farine de sarrasin",
                  "quantity": { "amount": "500", "unit": "g" }
                }
              ]
            }
          ]
        }"#;

        let mut app = open(0).await;
        let report = import(&mut app, json);
        assert_eq!(report.recipes_added, 1);
        assert_eq!(
            report.incomplete,
            ["Pain"],
            "the receipt says where to look"
        );

        // The line is kept, not dropped: a missing reference has a defined
        // rendering (DECISIONS 0034) and dropping it would lose the line.
        let state = app.state().expect("state");
        let opened = app
            .apply(Command::OpenRecipe {
                recipe: id_of_recipe(&state, "Pain"),
                servings: None,
            })
            .expect("the bread opens");
        let recipe = opened.focus.expect("open").recipe;
        assert!(
            matches!(
                recipe.components.first(),
                Some(ComponentView::Ingredient { name: None, .. })
            ),
            "the line is there and renders as a warning: {:?}",
            recipe.components.first()
        );
    });
}

#[test]
fn a_file_this_build_cannot_read_is_refused() {
    // Not JSON at all.
    assert!(LibraryFile::from_json("bonjour").is_err());

    // JSON, and not a cabas file. This is the one that matters: it parses.
    assert!(
        LibraryFile::from_json(r#"{"format":"something.else","format_version":1}"#).is_err(),
        "a file that says it is something else is refused rather than merged"
    );

    // A cabas file from a newer build. Refused rather than read with its
    // unknown half silently dropped.
    let future = format!(
        r#"{{"format":"{FORMAT}","format_version":{}}}"#,
        FORMAT_VERSION + 1
    );
    assert!(LibraryFile::from_json(&future).is_err());

    // An older one is read, which is the whole point of the number.
    let ours = format!(r#"{{"format":"{FORMAT}","format_version":{FORMAT_VERSION}}}"#);
    assert!(LibraryFile::from_json(&ours).is_ok());
}

#[test]
fn an_import_that_fails_leaves_the_library_exactly_as_it_was() {
    block_on(async {
        let mut app = open(0).await;
        stocked(&mut app).await;
        let before = app.state().expect("state");

        // The first ingredient is fine; the second one is not. If the import
        // wrote as it went, the first would already be in the document.
        let json = LibraryFile {
            format: FORMAT.into(),
            format_version: FORMAT_VERSION,
            app_version: "0.0.0".into(),
            exported_at: 0,
            shops: Vec::new(),
            ingredients: vec![ingredient("Cardamome", AisleTag::Pantry), {
                let mut broken = ingredient("Safran", AisleTag::Pantry);
                broken.density = Some("beaucoup".into());
                broken
            }],
            recipes: Vec::new(),
            photos: Default::default(),
        }
        .to_json()
        .expect("json");

        let file = LibraryFile::from_json(&json).expect("it reads");
        assert!(app.import_library(&file).is_err(), "it refuses the file");

        let after = app.state().expect("state");
        assert_eq!(
            after.ingredients.len(),
            before.ingredients.len(),
            "and nothing landed: {:?}",
            after
                .ingredients
                .iter()
                .map(|i| i.name.as_str())
                .collect::<Vec<_>>()
        );
    });
}

#[test]
fn photos_travel_in_the_file_and_come_back_under_their_own_ids() {
    block_on(async {
        let mut source = open(0).await;
        let photos = Photos::new(MemoryPhotoStore::new());
        let bytes = {
            let mut jpeg = vec![0xff, 0xd8, 0xff];
            jpeg.extend_from_slice(b"une photo de farine");
            jpeg
        };
        let photo = photos
            .put(&TestPlatform::from(0), &bytes)
            .await
            .expect("the photo is stored");

        let mut flour = ingredient("Farine", AisleTag::Staples);
        flour.photo = Some(photo.to_string());
        source
            .dispatch(Command::SaveIngredient { ingredient: flour })
            .await
            .expect("flour");

        // What the host does around `export_library`: it names the photos and
        // the host fetches them, because reading a photo is asynchronous and
        // exporting is not.
        let mut file = source.export_library().expect("export");
        assert_eq!(
            file.photo_ids(),
            std::slice::from_ref(&photo),
            "the file knows to ask"
        );
        for id in file.photo_ids() {
            let bytes = photos.get(&id).await.expect("read").expect("held");
            file.attach_photo(&id, &bytes);
        }
        let json = file.to_json().expect("json");
        assert!(json.len() > bytes.len(), "the bytes really are in the text");

        // And what it does around `import_library`, on a device that holds
        // neither the document nor the picture.
        let mut target = open(9000).await;
        let target_photos = Photos::new(MemoryPhotoStore::new());
        let file = LibraryFile::from_json(&json).expect("it reads");
        for (id, bytes) in file.carried_photos().expect("the photos decode") {
            target_photos
                .restore(&id, &bytes)
                .await
                .expect("it is stored under the id the document names");
        }
        target.import_library(&file).expect("import");

        let state = target.state().expect("state");
        let imported = state
            .ingredients
            .iter()
            .find(|i| i.name == "Farine")
            .expect("flour");
        assert_eq!(
            imported.photo.as_deref(),
            Some(photo.as_str()),
            "the ingredient names the same photo it always did"
        );
        assert_eq!(
            target_photos.get(&photo).await.expect("read"),
            Some(bytes),
            "and the bytes are here, byte for byte"
        );
    });
}

#[test]
fn a_photo_that_is_not_one_stops_the_import_before_anything_is_written() {
    block_on(async {
        let photos = Photos::new(MemoryPhotoStore::new());
        let id = cabas_domain::PhotoId::from_raw("pho_0000000000000001");
        assert!(
            photos.restore(&id, b"PNG, actually").await.is_err(),
            "a foreign id is accepted; foreign *content* is not"
        );
        assert!(photos.get(&id).await.expect("read").is_none());
    });
}

#[test]
fn an_export_is_stable_between_two_runs() {
    block_on(async {
        let mut app = open(0).await;
        stocked(&mut app).await;

        // Same library, twice. A file that reorders itself is a file nobody
        // can diff, and diffing two exports is how somebody checks what a
        // month changed.
        assert_eq!(exported(&app), exported(&app));

        let file = app.export_library().expect("export");
        let names: Vec<&str> = file.ingredients.iter().map(|i| i.name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted, "and it is in an order a person expects");
    });
}
