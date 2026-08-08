//! What a family-sized library actually costs.
//!
//! DECISIONS 0008 rests on an estimate — "a few hundred kilobytes, so a
//! serialized snapshot beats a relational store". This measures it, because
//! the number is what budgets the PWA's cold start: on iOS the app is
//! evicted from memory on app switch and restarts from scratch (DECISIONS
//! 0003), so loading the document is on the path the user waits through
//! every single time they open the app in a shop.
//!
//! Native only: it times things, and `std::time::Instant` panics on wasm32
//! (Rule 8). The wasm figure will differ and is measured for real at M4, on
//! the phone, which is the only place the answer means anything.
#![cfg(not(target_family = "wasm"))]

use std::num::NonZeroU32;
use std::time::Instant;

use cabas_domain::recipe::{Component, IngredientUsage, RefDisplay, Segment, Step};
use cabas_domain::units::{MassUnit, Unit};
use cabas_domain::{
    Aisle, Ingredient, IngredientId, Quantity, Rational, Recipe, RecipeId, UsageId,
};
use cabas_store::Document;

/// A family library: 200 recipes over a 300-ingredient vocabulary.
const RECIPES: usize = 200;
const INGREDIENTS: usize = 300;
const USAGES_PER_RECIPE: usize = 8;
const STEPS_PER_RECIPE: usize = 6;

const G: Unit = Unit::Mass(MassUnit::Gram);

fn nz(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).expect("non-zero")
}

fn ingredient(i: usize) -> Ingredient {
    let aisles = [
        Aisle::Produce,
        Aisle::Butcher,
        Aisle::Dairy,
        Aisle::Grocery,
        Aisle::Frozen,
    ];
    let mut ing = Ingredient::new(
        IngredientId::from_raw(format!("ing_{i:04}")),
        format!("Ingredient {i}"),
        aisles[i % aisles.len()],
    );
    ing.aliases.push(format!("alias {i}"));
    if i.is_multiple_of(3) {
        ing = ing.with_density(Rational::new(55, 100));
    }
    if i.is_multiple_of(5) {
        ing = ing.with_unit_weight(Rational::new(150, 1));
    }
    if i.is_multiple_of(11) {
        ing = ing.as_staple();
    }
    ing
}

fn recipe(r: usize) -> Recipe {
    let mut recipe = Recipe::new(
        RecipeId::from_raw(format!("rec_{r:04}")),
        format!("Recipe number {r}"),
        nz(4),
    )
    .with_yield(Quantity::whole(500, G));

    for u in 0..USAGES_PER_RECIPE {
        let ingredient = (r * USAGES_PER_RECIPE + u) % INGREDIENTS;
        recipe = recipe.with_component(Component::Ingredient(IngredientUsage {
            id: UsageId::from_raw(format!("u_{r:04}_{u}")),
            ingredient: IngredientId::from_raw(format!("ing_{ingredient:04}")),
            // Thirds and halves, so the exact-rational encoding is exercised
            // at scale rather than on round numbers only.
            quantity: Quantity::new(Rational::new((u as i128) + 1, 3), G),
        }));
    }

    for s in 0..STEPS_PER_RECIPE {
        recipe = recipe.with_step(Step {
            segments: vec![
                Segment::Text(format!(
                    "Step {s}: combine everything carefully and keep going. "
                )),
                Segment::Ingredient {
                    usage: UsageId::from_raw(format!("u_{r:04}_{}", s % USAGES_PER_RECIPE)),
                    display: RefDisplay::Full,
                },
                Segment::Text(" then rest for ten minutes.".into()),
            ],
        });
    }
    recipe
}

fn library() -> Document {
    let doc = Document::new();
    for i in 0..INGREDIENTS {
        doc.put_ingredient(&ingredient(i)).expect("write");
    }
    for r in 0..RECIPES {
        doc.put_recipe(&recipe(r)).expect("write");
    }
    doc
}

#[test]
fn a_family_library_stays_within_the_cold_start_budget() {
    let built = Instant::now();
    let doc = library();
    let build_time = built.elapsed();

    let snapshot = doc.snapshot().expect("snapshot");
    let compacted = doc.compacted_snapshot().expect("compact");

    let started = Instant::now();
    let reloaded = Document::load(&snapshot).expect("load");
    let load_time = started.elapsed();

    let started = Instant::now();
    let recipes = reloaded.recipes().expect("read");
    let ingredients = reloaded.ingredients().expect("read");
    let read_time = started.elapsed();

    println!("--- {RECIPES} recipes / {INGREDIENTS} ingredients ---");
    println!("  build          {build_time:?}");
    println!("  snapshot       {} bytes", snapshot.len());
    println!("  compacted      {} bytes", compacted.len());
    println!("  load           {load_time:?}");
    println!("  read all       {read_time:?}");

    assert_eq!(recipes.len(), RECIPES);
    assert_eq!(ingredients.len(), INGREDIENTS);

    // A ceiling, not a target. It exists to catch an encoding change that
    // multiplies the document — the kind of regression that is invisible
    // until a phone takes four seconds to open. The measured value at the
    // time of writing is in ROADMAP M2, an order of magnitude below this.
    assert!(
        snapshot.len() < 4 * 1024 * 1024,
        "snapshot ballooned to {} bytes",
        snapshot.len()
    );
}

#[test]
fn reading_a_full_library_back_gives_the_same_recipes() {
    // Size means nothing if the big document does not survive the trip.
    let doc = library();
    let reloaded = Document::load(&doc.snapshot().expect("snapshot")).expect("load");

    let expected: Vec<Recipe> = (0..RECIPES).map(recipe).collect();
    let mut sorted = expected.clone();
    sorted.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
    assert_eq!(reloaded.recipes().expect("read"), sorted);
}
