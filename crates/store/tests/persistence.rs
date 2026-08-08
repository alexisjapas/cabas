//! M2's exit criterion, through the public API only.
//!
//! Two things have to hold: what goes in comes back out unchanged, and two
//! replicas that edited the same document while apart agree once they meet.
//! The second is the one that decides whether the product works, because the
//! shape of the use case is fixed — one person in a shop with no signal, the
//! other at home — and it is not reproducible by hand.
//!
//! Native only. Nothing here touches a platform, so there is no wasm-specific
//! behaviour to catch: `wasm-check` proves the same code compiles for wasm32,
//! and `tests/indexeddb.rs` covers the one part that is genuinely
//! browser-shaped.
#![cfg(not(target_family = "wasm"))]

use std::num::NonZeroU32;

use cabas_domain::event::{Action, Subject};
use cabas_domain::list::{ListEntry, ListItem};
use cabas_domain::overlay::Explicit;
use cabas_domain::recipe::{
    Component, IngredientUsage, RefDisplay, Segment, Step, SubRecipeAmount, SubRecipeUsage,
};
use cabas_domain::units::{MassUnit, Unit, VolumeUnit};
use cabas_domain::{
    Aisle, Device, DeviceId, Event, Ingredient, IngredientId, ListEntryId, Quantity, Rational,
    Recipe, RecipeId, Timestamp, UsageId, User, UserId,
};
use cabas_store::{Document, StoreError};

const G: Unit = Unit::Mass(MassUnit::Gram);
const ML: Unit = Unit::Volume(VolumeUnit::Milliliter);

fn nz(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).expect("non-zero")
}

fn flour() -> Ingredient {
    Ingredient::new(IngredientId::from_raw("flour"), "Flour", Aisle::Grocery)
        .with_density(Rational::new(55, 100))
        .as_staple()
}

fn tomato() -> Ingredient {
    let mut t = Ingredient::new(IngredientId::from_raw("tomato"), "Tomato", Aisle::Produce)
        .with_unit_weight(Rational::new(150, 1));
    t.aliases.push("Tomates".into());
    t
}

/// A recipe exercising every branch of the schema: a yield, an ingredient
/// usage, a sub-recipe by factor, and steps with both kinds of segment.
fn tart() -> Recipe {
    Recipe::new(RecipeId::from_raw("tart"), "Apple tart", nz(4))
        .with_yield(Quantity::whole(500, G))
        .with_component(Component::Ingredient(IngredientUsage {
            id: UsageId::from_raw("u_flour"),
            ingredient: IngredientId::from_raw("flour"),
            // A third, to prove no decimal drift survives a round trip.
            quantity: Quantity::new(Rational::new(1, 3), ML),
        }))
        .with_component(Component::SubRecipe(SubRecipeUsage {
            id: UsageId::from_raw("u_pastry"),
            recipe: RecipeId::from_raw("pastry"),
            amount: SubRecipeAmount::Factor(Rational::new(1, 2)),
        }))
        .with_step(Step {
            segments: vec![
                Segment::Text("Add ".into()),
                Segment::Ingredient {
                    usage: UsageId::from_raw("u_flour"),
                    display: RefDisplay::Full,
                },
                Segment::Text(" and mix.".into()),
            ],
        })
        .with_step(Step::text("Bake for 40 minutes."))
}

/// A sub-recipe referenced by absolute yield amount — the other
/// `SubRecipeAmount` branch (DECISIONS 0017).
fn pastry() -> Recipe {
    Recipe::new(RecipeId::from_raw("pastry"), "Shortcrust", nz(4))
        .with_yield(Quantity::whole(300, G))
        .with_component(Component::Ingredient(IngredientUsage {
            id: UsageId::from_raw("u_butter"),
            ingredient: IngredientId::from_raw("butter"),
            quantity: Quantity::whole(125, G),
        }))
}

fn recipe_entry() -> ListEntry {
    ListEntry {
        id: ListEntryId::from_raw("e_tart"),
        item: ListItem::Recipe {
            recipe: RecipeId::from_raw("tart"),
            servings: nz(6),
        },
        added_by: UserId::from_raw("alice"),
        added_at: Timestamp(1_000),
    }
}

fn ingredient_entry() -> ListEntry {
    ListEntry {
        id: ListEntryId::from_raw("e_tomato"),
        item: ListItem::Ingredient {
            ingredient: IngredientId::from_raw("tomato"),
            quantity: Quantity::whole(3, Unit::Piece),
        },
        added_by: UserId::from_raw("bob"),
        added_at: Timestamp(2_000),
    }
}

/// A document holding one of everything.
fn populated() -> Document {
    let doc = Document::new();
    doc.put_ingredient(&flour()).expect("write");
    doc.put_ingredient(&tomato()).expect("write");
    doc.put_recipe(&tart()).expect("write");
    doc.put_recipe(&pastry()).expect("write");
    doc.add_list_entry(&recipe_entry()).expect("write");
    doc.add_list_entry(&ingredient_entry()).expect("write");
    doc.set_explicit(
        &IngredientId::from_raw("flour"),
        &Explicit::Checked {
            by: UserId::from_raw("alice"),
            at: Timestamp(3_000),
        },
    )
    .expect("write");
    doc.set_explicit(&IngredientId::from_raw("salt"), &Explicit::Unchecked)
        .expect("write");
    doc.put_user(&User::new(UserId::from_raw("alice"), "Alice"))
        .expect("write");
    doc.put_device(&Device::new(
        DeviceId::from_raw("d_phone"),
        UserId::from_raw("alice"),
        "Alice's iPhone",
        Timestamp(500),
    ))
    .expect("write");
    doc.record_event(&Event::new(
        Timestamp(4_000),
        UserId::from_raw("bob"),
        Action::Deleted,
        Subject::Recipe(RecipeId::from_raw("gone")),
        "A deleted recipe",
    ))
    .expect("write");
    doc
}

// --- round trip -------------------------------------------------------------

#[test]
fn every_persisted_type_survives_a_snapshot() {
    let original = populated();
    let bytes = original.snapshot().expect("snapshot");
    let reloaded = Document::load(&bytes).expect("load");

    assert_eq!(
        reloaded.ingredients().expect("read"),
        vec![flour(), tomato()]
    );
    assert_eq!(reloaded.recipes().expect("read"), vec![pastry(), tart()]);
    assert_eq!(
        reloaded.list().expect("read"),
        original.list().expect("read")
    );
    assert_eq!(
        reloaded.overlay().expect("read"),
        original.overlay().expect("read")
    );
    assert_eq!(reloaded.users().expect("read").len(), 1);
    assert_eq!(
        reloaded.devices().expect("read"),
        original.devices().expect("read")
    );
    assert_eq!(reloaded.events().expect("read").events.len(), 1);
}

#[test]
fn a_third_of_a_millilitre_is_still_a_third_after_a_round_trip() {
    // Rule 4 end to end: the document has no exact numeric type, so the
    // quantity goes through the codec as a rational string. If it ever went
    // through an f64 this is where it would show up.
    let doc = populated();
    let bytes = doc.snapshot().expect("snapshot");
    let recipes = Document::load(&bytes)
        .expect("load")
        .recipes()
        .expect("read");

    let tart = recipes
        .iter()
        .find(|r| r.name == "Apple tart")
        .expect("tart");
    let usage = tart.usage(&UsageId::from_raw("u_flour")).expect("usage");
    assert_eq!(usage.quantity.value, Rational::new(1, 3));
    assert_eq!(usage.quantity.unit, ML);
}

#[test]
fn an_empty_document_reads_as_empty_rather_than_failing() {
    let doc = Document::new();
    assert!(doc.ingredients().expect("read").is_empty());
    assert!(doc.recipes().expect("read").is_empty());
    assert!(doc.list().expect("read").entries.is_empty());
    assert!(doc.overlay().expect("read").is_empty());
    assert!(doc.users().expect("read").is_empty());
    assert!(doc.devices().expect("read").is_empty());
    assert!(doc.events().expect("read").events.is_empty());
}

#[test]
fn removals_take_effect_and_survive_a_reload() {
    let doc = populated();
    doc.remove_ingredient(&IngredientId::from_raw("tomato"))
        .expect("remove");
    doc.remove_recipe(&RecipeId::from_raw("pastry"))
        .expect("remove");
    doc.remove_list_entry(&ListEntryId::from_raw("e_tomato"))
        .expect("remove");
    doc.clear_explicit(&IngredientId::from_raw("flour"))
        .expect("clear");

    let reloaded = Document::load(&doc.snapshot().expect("snapshot")).expect("load");
    assert_eq!(reloaded.ingredients().expect("read"), vec![flour()]);
    assert_eq!(reloaded.recipes().expect("read"), vec![tart()]);
    assert_eq!(reloaded.list().expect("read").entries.len(), 1);
    // Only the explicit `Unchecked` on salt is left — and it must be left,
    // or the next derivation re-checks a staple the user unchecked (Rule 3).
    let overlay = reloaded.overlay().expect("read");
    assert_eq!(overlay.len(), 1);
    assert_eq!(
        overlay.get(&IngredientId::from_raw("salt")),
        Some(&Explicit::Unchecked)
    );
}

#[test]
fn a_truncated_snapshot_is_reported_rather_than_half_read() {
    let bytes = populated().snapshot().expect("snapshot");
    let truncated = &bytes[..bytes.len() / 2];
    assert!(matches!(
        Document::load(truncated),
        Err(StoreError::Snapshot(_))
    ));
}

// --- convergence ------------------------------------------------------------

/// Exchanges everything each replica is missing, in both directions.
fn reconcile(a: &Document, b: &Document) {
    let from_a = a.changes_since(&b.version()).expect("export");
    let from_b = b.changes_since(&a.version()).expect("export");
    b.merge(&from_a).expect("merge");
    a.merge(&from_b).expect("merge");
}

fn assert_converged(a: &Document, b: &Document) {
    assert_eq!(
        a.ingredients().expect("read"),
        b.ingredients().expect("read")
    );
    assert_eq!(a.recipes().expect("read"), b.recipes().expect("read"));
    assert_eq!(a.list().expect("read"), b.list().expect("read"));
    assert_eq!(a.overlay().expect("read"), b.overlay().expect("read"));
    assert_eq!(a.users().expect("read"), b.users().expect("read"));
    assert_eq!(a.devices().expect("read"), b.devices().expect("read"));
    assert_eq!(
        a.events().expect("read").events,
        b.events().expect("read").events
    );
}

/// Two replicas of the same document, as after a pairing.
fn paired() -> (Document, Document) {
    let origin = populated();
    let bytes = origin.snapshot().expect("snapshot");

    let phone = Document::load(&bytes).expect("load");
    phone.set_peer(1).expect("peer");
    let laptop = Document::load(&bytes).expect("load");
    laptop.set_peer(2).expect("peer");
    (phone, laptop)
}

#[test]
fn the_shop_and_the_kitchen_converge() {
    // The dimensioning case (DECISIONS 0006): one person checking items in a
    // shop with no signal while the other adds to the list from home.
    let (phone, laptop) = paired();

    phone
        .set_explicit(
            &IngredientId::from_raw("tomato"),
            &Explicit::Checked {
                by: UserId::from_raw("alice"),
                at: Timestamp(5_000),
            },
        )
        .expect("check");

    laptop
        .add_list_entry(&ListEntry {
            id: ListEntryId::from_raw("e_butter"),
            item: ListItem::Ingredient {
                ingredient: IngredientId::from_raw("butter"),
                quantity: Quantity::whole(250, G),
            },
            added_by: UserId::from_raw("bob"),
            added_at: Timestamp(5_100),
        })
        .expect("add");

    reconcile(&phone, &laptop);
    assert_converged(&phone, &laptop);

    // Neither edit was lost: they touched different things.
    assert_eq!(phone.list().expect("read").entries.len(), 3);
    assert!(
        phone
            .overlay()
            .expect("read")
            .contains_key(&IngredientId::from_raw("tomato"))
    );
}

#[test]
fn checking_and_unchecking_the_same_item_converges_on_one_answer() {
    // The explicit M2 criterion. There is no "right" winner — both people
    // acted — but both devices must end up showing the same thing.
    let (phone, laptop) = paired();
    let salt = IngredientId::from_raw("salt");

    phone
        .set_explicit(
            &salt,
            &Explicit::Checked {
                by: UserId::from_raw("alice"),
                at: Timestamp(6_000),
            },
        )
        .expect("check");
    laptop
        .set_explicit(&salt, &Explicit::Unchecked)
        .expect("uncheck");

    reconcile(&phone, &laptop);
    assert_converged(&phone, &laptop);

    let settled = phone.overlay().expect("read");
    let winner = settled.get(&salt).expect("an explicit action survives");
    assert!(
        matches!(winner, Explicit::Checked { .. } | Explicit::Unchecked),
        "the surviving action must still be an explicit one"
    );
}

#[test]
fn the_same_ingredient_created_twice_offline_becomes_one_ingredient() {
    // The trap `ensure_mergeable_map` exists to avoid: with an
    // operation-derived container id these two writes would land in two
    // different containers under one key, and the merge would keep one and
    // drop the other's fields — or worse, read back as a half-written entity.
    //
    // Both devices write *every* field here, because both are creating the
    // ingredient from scratch. So each field is a genuine conflict and
    // last-writer-wins settles it; what must not happen is a split entity.
    let (phone, laptop) = paired();
    let id = IngredientId::from_raw("saffron");

    let mut mine = Ingredient::new(id.clone(), "Saffron", Aisle::Grocery);
    mine.aliases.push("Safran".into());
    phone.put_ingredient(&mine).expect("write");

    let theirs = Ingredient::new(id.clone(), "Saffron", Aisle::Other).as_staple();
    laptop.put_ingredient(&theirs).expect("write");

    reconcile(&phone, &laptop);
    assert_converged(&phone, &laptop);

    let saffron: Vec<_> = phone
        .ingredients()
        .expect("read")
        .into_iter()
        .filter(|i| i.id == id)
        .collect();
    assert_eq!(saffron.len(), 1, "one key must mean one ingredient");
    // Whichever side won, the result is a coherent ingredient rather than a
    // mixture with holes in it.
    let merged = &saffron[0];
    assert_eq!(merged.name, "Saffron");
    assert!(matches!(merged.aisle, Aisle::Grocery | Aisle::Other));
}

#[test]
fn edits_to_different_fields_of_one_ingredient_both_survive() {
    // The per-field merge, shown where it is real: the ingredient already
    // exists, and each device touches a field the other left alone. This is
    // what "write only what changed" buys — a `put_ingredient` that rewrote
    // every field would turn these disjoint edits into a conflict and lose
    // one of them.
    let (phone, laptop) = paired();

    let mut moved = flour();
    moved.aisle = Aisle::Bakery;
    phone.put_ingredient(&moved).expect("write");

    let mut renamed_alias = flour();
    renamed_alias.aliases.push("Farine".into());
    laptop.put_ingredient(&renamed_alias).expect("write");

    reconcile(&phone, &laptop);
    assert_converged(&phone, &laptop);

    let merged = phone
        .ingredients()
        .expect("read")
        .into_iter()
        .find(|i| i.id == IngredientId::from_raw("flour"))
        .expect("flour survived");
    assert_eq!(merged.aisle, Aisle::Bakery);
    assert_eq!(merged.aliases, vec!["Farine".to_string()]);
}

#[test]
fn editing_a_recipes_name_while_the_other_edits_its_lines_keeps_both() {
    // What "write only what changed" buys. A coarse `put_recipe` that
    // rewrote every field would make these two edits compete, and the
    // ingredient line would lose to the rename.
    let (phone, laptop) = paired();

    let mut renamed = tart();
    renamed.name = "Tarte aux pommes".into();
    phone.put_recipe(&renamed).expect("write");

    let mut extended = tart();
    extended
        .components
        .push(Component::Ingredient(IngredientUsage {
            id: UsageId::from_raw("u_sugar"),
            ingredient: IngredientId::from_raw("sugar"),
            quantity: Quantity::whole(80, G),
        }));
    laptop.put_recipe(&extended).expect("write");

    reconcile(&phone, &laptop);
    assert_converged(&phone, &laptop);

    let merged = phone
        .recipes()
        .expect("read")
        .into_iter()
        .find(|r| r.id == RecipeId::from_raw("tart"))
        .expect("tart survived");
    assert_eq!(merged.name, "Tarte aux pommes");
    assert_eq!(merged.components.len(), 3);
}

#[test]
fn replicas_that_are_never_online_together_still_converge() {
    // The relay case (DECISIONS 0009): each side syncs with a store-and-
    // forward middle, never with the other device directly.
    let (phone, laptop) = paired();
    let relay = Document::load(&populated().snapshot().expect("snapshot")).expect("load");
    relay.set_peer(3).expect("peer");

    phone
        .set_explicit(&IngredientId::from_raw("flour"), &Explicit::Unchecked)
        .expect("uncheck");

    // The phone talks to the relay, then goes away for good.
    let from_phone = phone.changes_since(&relay.version()).expect("export");
    relay.merge(&from_phone).expect("merge");

    // Much later, the laptop talks to the relay. The two never met.
    laptop
        .add_list_entry(&ListEntry {
            id: ListEntryId::from_raw("e_milk"),
            item: ListItem::Ingredient {
                ingredient: IngredientId::from_raw("milk"),
                quantity: Quantity::whole(1, Unit::Piece),
            },
            added_by: UserId::from_raw("bob"),
            added_at: Timestamp(7_000),
        })
        .expect("add");
    reconcile(&laptop, &relay);

    assert_eq!(
        laptop
            .overlay()
            .expect("read")
            .get(&IngredientId::from_raw("flour")),
        Some(&Explicit::Unchecked),
        "the phone's uncheck reached the laptop through the relay"
    );
    assert_converged(&laptop, &relay);
}

#[test]
fn merging_is_idempotent() {
    // A relay may well hand the same delta twice; applying it again must not
    // duplicate a list entry or an event.
    let (phone, laptop) = paired();
    phone
        .add_list_entry(&ListEntry {
            id: ListEntryId::from_raw("e_dup"),
            item: ListItem::Ingredient {
                ingredient: IngredientId::from_raw("milk"),
                quantity: Quantity::whole(1, Unit::Piece),
            },
            added_by: UserId::from_raw("alice"),
            added_at: Timestamp(8_000),
        })
        .expect("add");

    let delta = phone.changes_since(&laptop.version()).expect("export");
    laptop.merge(&delta).expect("first");
    laptop.merge(&delta).expect("second");

    assert_eq!(laptop.list().expect("read").entries.len(), 3);
}

// --- compaction -------------------------------------------------------------

#[test]
fn a_compacted_snapshot_keeps_the_state_and_drops_the_history() {
    let doc = populated();
    // Churn: the same ingredient rewritten many times is exactly what makes a
    // document grow without bound.
    for i in 0..200 {
        let mut ingredient = tomato();
        ingredient.name = format!("Tomato {i}");
        doc.put_ingredient(&ingredient).expect("write");
    }

    let full = doc.snapshot().expect("snapshot");
    let compacted = doc.compacted_snapshot().expect("compact");
    assert!(
        compacted.len() < full.len(),
        "compaction should shed history: {} vs {} bytes",
        compacted.len(),
        full.len()
    );

    // And it is still a usable document.
    let reloaded = Document::load(&compacted).expect("load");
    assert_eq!(reloaded.ingredients().expect("read").len(), 2);
    assert_eq!(reloaded.recipes().expect("read"), vec![pastry(), tart()]);
}
