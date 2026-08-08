//! End-to-end exercise of the domain through its public surface only.
//!
//! This is the M1 exit criterion: a list holding recipes, sub-recipes and bare
//! ingredients produces a correct aggregated cart, with no I/O of any kind.

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use cabas_domain::cart::{self, IngredientIndex};
use cabas_domain::expand::RecipeIndex;
use cabas_domain::ingredient::Aisle;
use cabas_domain::list::{ListEntry, ListItem, ShoppingList};
use cabas_domain::overlay::{CheckState, Explicit, Overlay};
use cabas_domain::recipe::{Component, IngredientUsage, SubRecipeAmount, SubRecipeUsage};
use cabas_domain::units::{MassUnit, Unit, VolumeUnit};
use cabas_domain::{
    Ingredient, IngredientId, ListEntryId, Quantity, Rational, Recipe, RecipeId, Timestamp,
    UsageId, UserId,
};
use proptest::prelude::*;

const G: Unit = Unit::Mass(MassUnit::Gram);
const ML: Unit = Unit::Volume(VolumeUnit::Milliliter);
const PIECE: Unit = Unit::Piece;

fn nz(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).expect("non-zero")
}

fn rat(n: i128, d: i128) -> Rational {
    Rational::new(n, d)
}

fn iid(s: &str) -> IngredientId {
    IngredientId::from_raw(s)
}

fn rid(s: &str) -> RecipeId {
    RecipeId::from_raw(s)
}

fn alice() -> UserId {
    UserId::from_raw("alice")
}

fn uses(usage: &str, ingredient: &str, quantity: Quantity) -> Component {
    Component::Ingredient(IngredientUsage {
        id: UsageId::from_raw(usage),
        ingredient: iid(ingredient),
        quantity,
    })
}

// ---------------------------------------------------------------- fixtures

fn ingredients() -> IngredientIndex {
    let all = vec![
        Ingredient::new(iid("flour"), "Flour", Aisle::Grocery)
            .with_density(rat(55, 100))
            .as_staple(),
        Ingredient::new(iid("salt"), "Salt", Aisle::Grocery).as_staple(),
        Ingredient::new(iid("butter"), "Butter", Aisle::Dairy).with_density(rat(911, 1000)),
        Ingredient::new(iid("milk"), "Milk", Aisle::Dairy).with_density(rat(103, 100)),
        Ingredient::new(iid("egg"), "Egg", Aisle::Dairy).with_unit_weight(rat(60, 1)),
        Ingredient::new(iid("apple"), "Apple", Aisle::Produce).with_unit_weight(rat(150, 1)),
        Ingredient::new(iid("tomato"), "Tomato", Aisle::Produce).with_unit_weight(rat(150, 1)),
    ];
    all.into_iter().map(|i| (i.id.clone(), i)).collect()
}

/// Pastry serves 4 and yields 500 g. Apple tart uses 400 g of it — the case
/// that `servings` alone could not express.
fn recipes() -> RecipeIndex {
    let pastry = Recipe::new(rid("pastry"), "Shortcrust", nz(4))
        .with_yield(Quantity::whole(500, G))
        .with_component(uses("u_p_flour", "flour", Quantity::whole(300, G)))
        .with_component(uses("u_p_butter", "butter", Quantity::whole(150, G)))
        .with_component(uses("u_p_salt", "salt", Quantity::to_taste()));

    let tart = Recipe::new(rid("tart"), "Apple tart", nz(4))
        .with_component(Component::SubRecipe(SubRecipeUsage {
            id: UsageId::from_raw("u_t_pastry"),
            recipe: rid("pastry"),
            amount: SubRecipeAmount::OfYield(Quantity::whole(400, G)),
        }))
        .with_component(uses("u_t_apple", "apple", Quantity::whole(4, PIECE)))
        .with_component(uses("u_t_egg", "egg", Quantity::whole(1, PIECE)));

    let crepes = Recipe::new(rid("crepes"), "Crepes", nz(4))
        .with_component(uses("u_c_flour", "flour", Quantity::whole(250, G)))
        .with_component(uses("u_c_milk", "milk", Quantity::whole(500, ML)))
        .with_component(uses("u_c_egg", "egg", Quantity::whole(3, PIECE)))
        .with_component(uses("u_c_salt", "salt", Quantity::whole(1, Unit::Pinch)));

    [pastry, tart, crepes]
        .into_iter()
        .map(|r| (r.id.clone(), r))
        .collect()
}

fn recipe_entry(id: &str, recipe: &str, servings: u32) -> ListEntry {
    ListEntry {
        id: ListEntryId::from_raw(id),
        item: ListItem::Recipe {
            recipe: rid(recipe),
            servings: nz(servings),
        },
        added_by: alice(),
        added_at: Timestamp(0),
    }
}

fn ingredient_entry(id: &str, ingredient: &str, quantity: Quantity) -> ListEntry {
    ListEntry {
        id: ListEntryId::from_raw(id),
        item: ListItem::Ingredient {
            ingredient: iid(ingredient),
            quantity,
        },
        added_by: alice(),
        added_at: Timestamp(0),
    }
}

/// Tart for 6 (written for 4), crepes for 4, plus 3 tomatoes by hand.
fn scenario() -> (ShoppingList, Overlay) {
    let mut overlay = Overlay::new();
    let mut list = ShoppingList::default();
    list.add(recipe_entry("e_tart", "tart", 6), &mut overlay);
    list.add(recipe_entry("e_crepes", "crepes", 4), &mut overlay);
    list.add(
        ingredient_entry("e_tomato", "tomato", Quantity::whole(3, PIECE)),
        &mut overlay,
    );
    (list, overlay)
}

// ------------------------------------------------------------------- tests

#[test]
fn a_realistic_list_aggregates_into_a_correct_cart() {
    let (list, overlay) = scenario();
    let cart = cart::derive(&list, &recipes(), &ingredients(), &overlay).expect("derives");

    // Sorted by aisle (Produce, Dairy, Grocery), then by name.
    let names: Vec<&str> = cart.lines.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(
        names,
        ["Apple", "Tomato", "Butter", "Egg", "Milk", "Flour", "Salt"]
    );

    let amount = |ingredient: &str| -> Vec<Quantity> {
        cart.line(&iid(ingredient)).expect("line").amounts.clone()
    };

    // Tart ×1.5; pastry is 400/500 of itself, so 1.2 of what it is written for.
    // Flour: 300×1.2 = 360 from the pastry, + 250 from the crepes.
    assert_eq!(amount("flour"), vec![Quantity::whole(610, G)]);
    assert_eq!(amount("butter"), vec![Quantity::whole(180, G)]);
    assert_eq!(amount("milk"), vec![Quantity::whole(500, ML)]);
    assert_eq!(amount("apple"), vec![Quantity::whole(6, PIECE)]);
    assert_eq!(amount("tomato"), vec![Quantity::whole(3, PIECE)]);

    // Eggs: 1×1.5 from the tart + 3 from the crepes = 4.5, and the shop sells 5.
    assert_eq!(amount("egg"), vec![Quantity::whole(5, PIECE)]);

    // Salt arrives as a pinch and as "to taste": two dimensionless amounts
    // that must not be invented into one.
    assert_eq!(
        amount("salt"),
        vec![
            Quantity::whole(1, Unit::Pinch),
            Quantity::new(Rational::from_integer(1), Unit::ToTaste),
        ]
    );
}

#[test]
fn staples_start_out_of_the_way_and_everything_else_is_to_buy() {
    let (list, overlay) = scenario();
    let cart = cart::derive(&list, &recipes(), &ingredients(), &overlay).expect("derives");

    let state = |i: &str| cart.line(&iid(i)).expect("line").state.clone();
    assert_eq!(state("flour"), CheckState::AutoChecked);
    assert_eq!(state("salt"), CheckState::AutoChecked);
    assert_eq!(state("butter"), CheckState::ToBuy);

    assert_eq!(cart.already_at_home().count(), 2);
    assert_eq!(cart.to_buy().count(), 5);
    assert_eq!(cart.bought().count(), 0);
}

#[test]
fn adding_a_staple_by_hand_makes_it_visible_again() {
    let (mut list, mut overlay) = scenario();

    // It starts auto-checked because only recipes asked for it…
    let before = cart::derive(&list, &recipes(), &ingredients(), &overlay).expect("derives");
    assert_eq!(
        before.line(&iid("salt")).expect("line").state,
        CheckState::AutoChecked
    );

    // …and putting it on the list by hand is a statement that you need some.
    list.add(
        ingredient_entry("e_salt", "salt", Quantity::whole(1, PIECE)),
        &mut overlay,
    );
    let after = cart::derive(&list, &recipes(), &ingredients(), &overlay).expect("derives");
    assert_eq!(
        after.line(&iid("salt")).expect("line").state,
        CheckState::ToBuy
    );
}

#[test]
fn adding_by_hand_also_clears_an_earlier_explicit_check() {
    let (mut list, mut overlay) = scenario();
    overlay.insert(
        iid("tomato"),
        Explicit::Checked {
            by: alice(),
            at: Timestamp(10),
        },
    );

    // You bought tomatoes, then realise you need more: adding them back must
    // return the line to "to buy" rather than leave it checked.
    list.add(
        ingredient_entry("e_more_tomato", "tomato", Quantity::whole(2, PIECE)),
        &mut overlay,
    );
    let cart = cart::derive(&list, &recipes(), &ingredients(), &overlay).expect("derives");
    let line = cart.line(&iid("tomato")).expect("line");
    assert_eq!(line.state, CheckState::ToBuy);
    assert_eq!(line.amounts, vec![Quantity::whole(5, PIECE)]);
}

#[test]
fn unchecking_a_staple_survives_the_next_derivation() {
    let (list, mut overlay) = scenario();
    overlay.insert(iid("flour"), Explicit::Unchecked);
    let cart = cart::derive(&list, &recipes(), &ingredients(), &overlay).expect("derives");
    assert_eq!(
        cart.line(&iid("flour")).expect("line").state,
        CheckState::ToBuy
    );
}

#[test]
fn checking_a_shared_ingredient_advances_every_recipe_at_once() {
    let (list, mut overlay) = scenario();
    // Flour is asked for by both the tart (via the pastry) and the crepes.
    let cart = cart::derive(&list, &recipes(), &ingredients(), &overlay).expect("derives");
    let flour = cart.line(&iid("flour")).expect("line");
    assert_eq!(flour.sources.len(), 2);

    overlay.insert(
        iid("flour"),
        Explicit::Checked {
            by: alice(),
            at: Timestamp(1),
        },
    );
    let cart = cart::derive(&list, &recipes(), &ingredients(), &overlay).expect("derives");
    let progress = cart.progress(&list);

    // Both recipe entries counted the same single check.
    let tart = progress
        .iter()
        .find(|p| p.entry.as_str() == "e_tart")
        .expect("entry");
    let crepes = progress
        .iter()
        .find(|p| p.entry.as_str() == "e_crepes")
        .expect("entry");
    // Tart wants flour, butter and salt (through the pastry), plus apple and
    // egg: flour is now checked and salt auto-checks, so 2 of 5.
    assert_eq!((tart.settled, tart.total), (2, 5));
    // Crepes want flour, milk, egg and salt: the same flour check, plus salt.
    assert_eq!((crepes.settled, crepes.total), (2, 4));
}

#[test]
fn amounts_stay_on_separate_lines_without_a_coefficient() {
    // Sugar has no density: 200 g and 100 ml cannot honestly become one number.
    let mut index = ingredients();
    index.insert(
        iid("sugar"),
        Ingredient::new(iid("sugar"), "Sugar", Aisle::Grocery),
    );
    let recipe = Recipe::new(rid("mix"), "Mix", nz(1))
        .with_component(uses("u1", "sugar", Quantity::whole(200, G)))
        .with_component(uses("u2", "sugar", Quantity::whole(100, ML)));
    let recipes: RecipeIndex = [(recipe.id.clone(), recipe)].into_iter().collect();

    let mut overlay = Overlay::new();
    let mut list = ShoppingList::default();
    list.add(recipe_entry("e", "mix", 1), &mut overlay);

    let cart = cart::derive(&list, &recipes, &index, &overlay).expect("derives");
    assert_eq!(
        cart.line(&iid("sugar")).expect("line").amounts,
        vec![Quantity::whole(200, G), Quantity::whole(100, ML)]
    );
}

#[test]
fn a_coefficient_merges_the_same_amounts_into_one_line() {
    // Flour has a density, so grams and millilitres do combine.
    let recipe = Recipe::new(rid("mix"), "Mix", nz(1))
        .with_component(uses("u1", "flour", Quantity::whole(200, G)))
        .with_component(uses("u2", "flour", Quantity::whole(100, ML)));
    let recipes: RecipeIndex = [(recipe.id.clone(), recipe)].into_iter().collect();

    let mut overlay = Overlay::new();
    let mut list = ShoppingList::default();
    list.add(recipe_entry("e", "mix", 1), &mut overlay);

    let cart = cart::derive(&list, &recipes, &ingredients(), &overlay).expect("derives");
    // 100 ml × 0.55 g/ml = 55 g.
    assert_eq!(
        cart.line(&iid("flour")).expect("line").amounts,
        vec![Quantity::whole(255, G)]
    );
}

#[test]
fn count_wins_over_mass_when_the_ingredient_can_be_counted() {
    // 2 apples + 300 g, at 150 g each, is 4 apples — the actionable answer in
    // a shop, rather than 600 g of apple.
    let recipe = Recipe::new(rid("mix"), "Mix", nz(1))
        .with_component(uses("u1", "apple", Quantity::whole(2, PIECE)))
        .with_component(uses("u2", "apple", Quantity::whole(300, G)));
    let recipes: RecipeIndex = [(recipe.id.clone(), recipe)].into_iter().collect();

    let mut overlay = Overlay::new();
    let mut list = ShoppingList::default();
    list.add(recipe_entry("e", "mix", 1), &mut overlay);

    let cart = cart::derive(&list, &recipes, &ingredients(), &overlay).expect("derives");
    assert_eq!(
        cart.line(&iid("apple")).expect("line").amounts,
        vec![Quantity::whole(4, PIECE)]
    );
}

#[test]
fn an_entry_completes_only_once_all_its_ingredients_are_settled() {
    let (list, mut overlay) = scenario();
    let recipes = recipes();
    let index = ingredients();

    let check = |overlay: &mut Overlay, ingredient: &str| {
        overlay.insert(
            iid(ingredient),
            Explicit::Checked {
                by: alice(),
                at: Timestamp(1),
            },
        );
    };

    // The tart needs flour (auto), salt (auto), butter, apple, egg.
    for ingredient in ["butter", "apple"] {
        check(&mut overlay, ingredient);
    }
    let cart = cart::derive(&list, &recipes, &index, &overlay).expect("derives");
    let tart = cart
        .progress(&list)
        .into_iter()
        .find(|p| p.entry.as_str() == "e_tart")
        .expect("entry");
    assert_eq!((tart.settled, tart.total), (4, 5));
    assert!(!tart.is_complete());

    check(&mut overlay, "egg");
    let cart = cart::derive(&list, &recipes, &index, &overlay).expect("derives");
    let tart = cart
        .progress(&list)
        .into_iter()
        .find(|p| p.entry.as_str() == "e_tart")
        .expect("entry");
    assert!(tart.is_complete());
}

#[test]
fn finishing_the_trip_keeps_the_progress_of_what_is_left() {
    let (mut list, mut overlay) = scenario();
    let recipes = recipes();
    let index = ingredients();

    // Buy everything the tart needs; the crepes still want milk.
    for ingredient in ["butter", "apple", "egg", "flour"] {
        overlay.insert(
            iid(ingredient),
            Explicit::Checked {
                by: alice(),
                at: Timestamp(1),
            },
        );
    }
    overlay.insert(
        iid("tomato"),
        Explicit::Checked {
            by: alice(),
            at: Timestamp(1),
        },
    );

    let cart = cart::derive(&list, &recipes, &index, &overlay).expect("derives");
    cart::finish_shopping(&mut list, &cart, &mut overlay);

    // Tart and tomato are done and gone; the crepes remain.
    let remaining: Vec<&str> = list.entries.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(remaining, ["e_crepes"]);

    // Flour and egg were shared with the crepes, so their checks survive; the
    // tart's own ingredients are dropped along with it. This is the whole
    // point of not clearing the overlay wholesale.
    assert!(overlay.contains_key(&iid("flour")));
    assert!(overlay.contains_key(&iid("egg")));
    assert!(!overlay.contains_key(&iid("butter")));
    assert!(!overlay.contains_key(&iid("apple")));
    assert!(!overlay.contains_key(&iid("tomato")));

    let cart = cart::derive(&list, &recipes, &index, &overlay).expect("derives");
    let crepes = cart
        .progress(&list)
        .into_iter()
        .find(|p| p.entry.as_str() == "e_crepes")
        .expect("entry");
    // flour and egg still checked, salt auto, only the milk left to buy.
    assert_eq!((crepes.settled, crepes.total), (3, 4));
}

#[test]
fn an_unknown_ingredient_is_reported_rather_than_silently_dropped() {
    let recipe = Recipe::new(rid("mix"), "Mix", nz(1)).with_component(uses(
        "u1",
        "unobtainium",
        Quantity::whole(1, G),
    ));
    let recipes: RecipeIndex = [(recipe.id.clone(), recipe)].into_iter().collect();
    let mut overlay = Overlay::new();
    let mut list = ShoppingList::default();
    list.add(recipe_entry("e", "mix", 1), &mut overlay);

    let err = cart::derive(&list, &recipes, &ingredients(), &overlay).expect_err("unknown");
    assert_eq!(
        err,
        cabas_domain::CartError::UnknownIngredient(iid("unobtainium"))
    );
}

// -------------------------------------------------------------- properties

/// A single-ingredient recipe, used to drive the scaling properties.
fn single(grams_a: i128, grams_b: i128, servings: u32) -> (RecipeIndex, IngredientIndex) {
    let recipe = Recipe::new(rid("r"), "R", nz(servings))
        .with_component(uses("u1", "butter", Quantity::whole(grams_a, G)))
        .with_component(uses("u2", "butter", Quantity::whole(grams_b, G)));
    let recipes = [(recipe.id.clone(), recipe)].into_iter().collect();
    let index: IngredientIndex = [(
        iid("butter"),
        Ingredient::new(iid("butter"), "Butter", Aisle::Dairy),
    )]
    .into_iter()
    .collect();
    (recipes, index)
}

proptest! {
    /// The invariant the constitution names: scaling then aggregating equals
    /// aggregating then scaling.
    ///
    /// Stated on mass only, deliberately. Rounding a countable line up is not
    /// a linear operation, so the identity genuinely does not hold there — and
    /// asserting it would be asserting a bug.
    #[test]
    fn scaling_commutes_with_aggregation(
        a in 1i128..10_000,
        b in 1i128..10_000,
        wanted in 1u32..64,
        written in 1u32..64,
    ) {
        let (recipes, index) = single(a, b, written);
        let mut overlay = Overlay::new();
        let mut list = ShoppingList::default();
        list.add(recipe_entry("e", "r", wanted), &mut overlay);

        let cart = cart::derive(&list, &recipes, &index, &overlay).expect("derives");
        let aggregated_then_scaled = Quantity::whole(a + b, G)
            .scaled(rat(i128::from(wanted), i128::from(written)))
            .humanized();

        prop_assert_eq!(&cart.line(&iid("butter")).expect("line").amounts[0], &aggregated_then_scaled);
    }

    /// Derivation is total over the inputs it accepts: whatever the servings,
    /// it returns a cart rather than panicking, and the ingredient set is
    /// unchanged by scaling.
    #[test]
    fn derivation_is_total_and_preserves_the_ingredient_set(servings in 1u32..500) {
        let mut overlay = Overlay::new();
        let mut list = ShoppingList::default();
        list.add(recipe_entry("e_tart", "tart", servings), &mut overlay);

        let cart = cart::derive(&list, &recipes(), &ingredients(), &overlay).expect("derives");
        let mut names: Vec<&str> = cart.lines.iter().map(|l| l.name.as_str()).collect();
        names.sort_unstable();
        prop_assert_eq!(names, ["Apple", "Butter", "Egg", "Flour", "Salt"]);
    }

    /// Mass conversions round-trip exactly, at any scale (Rule 4).
    #[test]
    fn mass_conversion_round_trips(value in 1i128..1_000_000, denom in 1i128..1000) {
        let grams = Quantity::new(rat(value, denom), G);
        let kilos = grams.convert_to(Unit::Mass(MassUnit::Kilogram)).expect("same dimension");
        prop_assert_eq!(kilos.convert_to(G).expect("same dimension"), grams);
    }
}

/// Keeps the unused-import lint honest about `BTreeMap`, which the index
/// aliases resolve to.
#[allow(dead_code)]
fn _index_is_a_btreemap(i: IngredientIndex) -> BTreeMap<IngredientId, Ingredient> {
    i
}
