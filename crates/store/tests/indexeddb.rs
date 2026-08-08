//! The IndexedDB backend, in a real browser.
//!
//! There is no honest alternative: IndexedDB does not exist outside a
//! browser, and a mock of it would only ever prove that the mock works. These
//! run under `wasm-bindgen-test-runner` in headless chromium — `nix develop
//! .#wasm-test -c wasm-test`, and the `wasm-storage` CI job.
//!
//! Every case uses its own database name. The runner shares one browser
//! profile across the whole file, so a fixed name would make each test
//! depend on what the previous one left behind.
#![cfg(target_family = "wasm")]

use cabas_domain::{Aisle, Ingredient, IngredientId, Rational};
use cabas_store::{Document, IndexedDbStorage, Storage};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn a_database_that_was_never_written_loads_as_a_first_run() {
    let storage = IndexedDbStorage::with_database("cabas-test-empty");
    assert_eq!(
        storage.load().await.expect("load"),
        None,
        "a missing snapshot is a first run, not a failure"
    );
}

#[wasm_bindgen_test]
async fn bytes_round_trip_through_indexeddb() {
    let storage = IndexedDbStorage::with_database("cabas-test-bytes");
    storage.save(b"a snapshot").await.expect("save");
    assert_eq!(
        storage.load().await.expect("load"),
        Some(b"a snapshot".to_vec())
    );

    // Saving again replaces rather than accumulating.
    storage.save(b"newer").await.expect("save");
    assert_eq!(storage.load().await.expect("load"), Some(b"newer".to_vec()));
}

#[wasm_bindgen_test]
async fn a_snapshot_with_a_zero_byte_in_it_survives() {
    // The snapshot is a binary blob, not text. A backend that round-tripped
    // it through a string would truncate here.
    let storage = IndexedDbStorage::with_database("cabas-test-binary");
    let blob: Vec<u8> = vec![0, 255, 1, 0, 128, 0];
    storage.save(&blob).await.expect("save");
    assert_eq!(storage.load().await.expect("load"), Some(blob));
}

#[wasm_bindgen_test]
async fn a_real_document_survives_a_save_and_a_reload() {
    // The property that actually matters: the library is still there after
    // the PWA is evicted from memory and restarted (DECISIONS 0003).
    let storage = IndexedDbStorage::with_database("cabas-test-document");

    let doc = Document::new();
    doc.put_ingredient(
        &Ingredient::new(IngredientId::from_raw("flour"), "Flour", Aisle::Grocery)
            .with_density(Rational::new(55, 100))
            .as_staple(),
    )
    .expect("write");
    storage
        .save(&doc.snapshot().expect("snapshot"))
        .await
        .expect("save");

    let bytes = storage.load().await.expect("load").expect("a snapshot");
    let reloaded = Document::load(&bytes).expect("load");
    let ingredients = reloaded.ingredients().expect("read");

    assert_eq!(ingredients.len(), 1);
    assert_eq!(ingredients[0].name, "Flour");
    assert!(ingredients[0].staple);
    // Rule 4 all the way to the browser: an exact rational, not 0.55.
    assert_eq!(ingredients[0].density, Some(Rational::new(11, 20)));
}

#[wasm_bindgen_test]
async fn two_handles_on_one_database_see_the_same_data() {
    // Each operation opens its own connection, so this is the case that
    // would break if opening ever raced with itself.
    let writer = IndexedDbStorage::with_database("cabas-test-shared");
    let reader = IndexedDbStorage::with_database("cabas-test-shared");

    writer.save(b"written by one").await.expect("save");
    assert_eq!(
        reader.load().await.expect("load"),
        Some(b"written by one".to_vec())
    );
}
