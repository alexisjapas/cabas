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

use cabas_domain::{Aisle, Ingredient, IngredientId, PhotoId, Rational};
use cabas_store::{Document, IndexedDbPhotoStore, IndexedDbStorage, PhotoStore, Storage};
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
        &Ingredient::new(IngredientId::from_raw("flour"), "Flour", Aisle::Pantry)
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

// --- photos (DECISIONS 0062) ------------------------------------------------

#[wasm_bindgen_test]
async fn a_photo_this_device_does_not_hold_is_absent_rather_than_an_error() {
    let photos = IndexedDbPhotoStore::with_database("cabas-test-photos-empty");
    assert_eq!(
        photos
            .load(&PhotoId::from_raw("ph_0000000000000001"))
            .await
            .expect("load"),
        None,
        "a photo the other phone took is referenced long before it arrives"
    );
    assert!(photos.ids().await.expect("ids").is_empty());
}

#[wasm_bindgen_test]
async fn photos_round_trip_and_are_listed_by_id() {
    let photos = IndexedDbPhotoStore::with_database("cabas-test-photos-round-trip");
    let one = PhotoId::from_raw("ph_0000000000000001");
    let two = PhotoId::from_raw("ph_0000000000000002");

    // A JPEG's first bytes, zeroes included: the record is binary, and a
    // backend that went through a string would stop at the first zero.
    let jpeg: Vec<u8> = vec![0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x00, 0xff];
    photos.save(&one, &jpeg).await.expect("save");
    photos.save(&two, b"another").await.expect("save");

    assert_eq!(photos.load(&one).await.expect("load"), Some(jpeg));
    assert_eq!(
        photos.ids().await.expect("ids"),
        vec![one.clone(), two.clone()]
    );

    photos.remove(&one).await.expect("remove");
    assert_eq!(photos.load(&one).await.expect("load"), None);
    assert_eq!(photos.ids().await.expect("ids"), vec![two]);
}

#[wasm_bindgen_test]
async fn the_document_and_the_photos_share_one_database_without_disturbing_each_other() {
    // The whole point of the separate object store: saving the document is
    // every keystroke and every tick in a shop, and it must not touch a
    // single photo record (DECISIONS 0062).
    let storage = IndexedDbStorage::with_database("cabas-test-photos-beside");
    let photos = IndexedDbPhotoStore::with_database("cabas-test-photos-beside");
    let id = PhotoId::from_raw("ph_00000000000000ff");

    photos.save(&id, b"a photo").await.expect("save");
    storage.save(b"a snapshot").await.expect("save");
    storage.save(b"a later snapshot").await.expect("save");

    assert_eq!(
        photos.load(&id).await.expect("load"),
        Some(b"a photo".to_vec())
    );
    assert_eq!(
        storage.load().await.expect("load"),
        Some(b"a later snapshot".to_vec())
    );
}

#[wasm_bindgen_test]
async fn an_id_that_could_name_something_else_is_refused() {
    let photos = IndexedDbPhotoStore::with_database("cabas-test-photos-id");
    assert!(
        photos
            .save(&PhotoId::from_raw("../elsewhere"), b"x")
            .await
            .is_err()
    );
}
