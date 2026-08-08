//! The family document: one replica, held whole on every device.
//!
//! This is the entire surface `app` and `sync` are allowed to see. No Loro
//! type appears in any signature here — versions travel as opaque bytes, and
//! everything else is a plain domain struct (Rule 2). What that costs is one
//! translation layer; what it buys is that replacing the CRDT touches this
//! crate and nothing else.
//!
//! # What is stored, and what is not
//!
//! Sources only (Rule 3): recipes, ingredients, the single list, the
//! *explicit* check overlay, users, devices and the event log. The cart is a
//! pure derivation over those and is never written down — ask
//! [`cabas_domain::cart::derive`] for it.
//!
//! # Commands compose; they do not think
//!
//! `add_list_entry` persists an entry and nothing more. It does **not** purge
//! the ingredient's overlay entry, even though Rule 3 requires that purge to
//! happen: the rule lives in [`cabas_domain::list::ShoppingList::add`], and
//! `app` is what calls the domain and then persists both effects. A store
//! that re-derived domain rules would be a second implementation of them,
//! and the two would drift.

use std::borrow::Cow;

use cabas_domain::list::ListEntry;
use cabas_domain::overlay::Explicit;
use cabas_domain::{
    Device, Event, EventLog, Ingredient, IngredientId, ListEntryId, Overlay, Recipe, RecipeId,
    ShoppingList, User,
};
use loro::{ExportMode, LoroDoc, LoroList, LoroMap, LoroMovableList, LoroValue, VersionVector};

use crate::codec;
use crate::error::{Result, StoreError, crdt, snapshot};
use crate::mapping;
use crate::schema::{self, SCHEMA_VERSION};

#[derive(Debug)]
pub struct Document {
    doc: LoroDoc,
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl Document {
    /// An empty document, stamped with the schema version this build writes.
    pub fn new() -> Self {
        let doc = LoroDoc::new();
        let document = Self { doc };
        // Infallible on a fresh document: the only failure mode of a register
        // write is a detached container, and nothing is detached here.
        let _ = document.stamp_schema();
        document
    }

    /// Reads a document back from a snapshot.
    pub fn load(bytes: &[u8]) -> Result<Self> {
        let doc = LoroDoc::new();
        doc.import(bytes).map_err(snapshot)?;
        let document = Self { doc };
        document.check_schema()?;
        Ok(document)
    }

    /// Pins this replica's identity in the CRDT.
    ///
    /// Worth setting to something stable per device: peer ids are how the
    /// CRDT tells concurrent edits apart, and a replica that draws a fresh
    /// one on every launch leaves a trail of dead peers in the history.
    /// Deliberately *not* the `DeviceId` — DECISIONS 0024 keeps attribution
    /// out of Loro's internals, and this is the same boundary seen from the
    /// other side.
    pub fn set_peer(&self, peer: u64) -> Result<()> {
        self.doc.set_peer_id(peer).map_err(crdt)
    }

    // --- snapshots and sync -------------------------------------------------

    /// The full document — state and history — as bytes to persist.
    pub fn snapshot(&self) -> Result<Vec<u8>> {
        self.doc.commit();
        self.doc.export(ExportMode::Snapshot).map_err(snapshot)
    }

    /// The document with its history dropped up to the current version.
    ///
    /// This is the compaction that keeps a document from growing without
    /// bound (DECISIONS 0006): the state survives, the operations that built
    /// it do not. The cost is that time travel before this point is gone, and
    /// that a replica which has been offline since before the cut has to take
    /// a full snapshot instead of a delta — which is why this belongs on a
    /// slow cadence at the relay, not on every save.
    pub fn compacted_snapshot(&self) -> Result<Vec<u8>> {
        self.doc.commit();
        self.doc
            .export(ExportMode::ShallowSnapshot(Cow::Owned(
                self.doc.oplog_frontiers(),
            )))
            .map_err(snapshot)
    }

    /// This replica's version, opaque. Hand it to a peer and it can compute
    /// exactly what you are missing.
    pub fn version(&self) -> Vec<u8> {
        self.doc.commit();
        self.doc.oplog_vv().encode()
    }

    /// Everything this replica knows that the holder of `version` does not.
    pub fn changes_since(&self, version: &[u8]) -> Result<Vec<u8>> {
        self.doc.commit();
        let from = VersionVector::decode(version).map_err(snapshot)?;
        self.doc
            .export(ExportMode::Updates {
                from: Cow::Owned(from),
            })
            .map_err(snapshot)
    }

    /// Applies a peer's changes. Snapshots and deltas are both accepted, so a
    /// caller never has to know which one it was handed.
    pub fn merge(&self, updates: &[u8]) -> Result<()> {
        self.doc.import(updates).map_err(snapshot)?;
        self.check_schema()?;
        // A merge concatenates two logs and can push the result past the cap;
        // re-applying it here is why `EventLog::trim` is documented as safe to
        // run repeatedly.
        self.trim_events()?;
        self.doc.commit();
        Ok(())
    }

    // --- schema -------------------------------------------------------------

    fn stamp_schema(&self) -> Result<()> {
        let meta = self.doc.get_map(schema::root::META);
        mapping::set(&meta, schema::meta::SCHEMA, LoroValue::I64(SCHEMA_VERSION))?;
        self.doc.commit();
        Ok(())
    }

    /// Refuses a document from the future.
    ///
    /// A missing marker is accepted as "this build's version": an empty
    /// document has nothing to be incompatible about, and failing there would
    /// make a fresh install unopenable.
    fn check_schema(&self) -> Result<()> {
        let meta = self.doc.get_map(schema::root::META);
        let value = meta.get_deep_value();
        let map = codec::map(&value, schema::root::META)?;
        let Some(found) = codec::optional(map, schema::meta::SCHEMA) else {
            return Ok(());
        };
        let found = codec::int(found, "meta.schema")?;
        if found > SCHEMA_VERSION {
            return Err(StoreError::FutureSchema {
                found,
                supported: SCHEMA_VERSION,
            });
        }
        Ok(())
    }

    // --- reads --------------------------------------------------------------

    /// Ingredients, ordered by id.
    ///
    /// The order is imposed rather than inherited: a Loro map hands its keys
    /// back in hash order, which is not stable between replicas, and two
    /// devices showing the same library in different orders is a bug that
    /// only appears on the second device.
    pub fn ingredients(&self) -> Result<Vec<Ingredient>> {
        self.keyed(schema::root::INGREDIENTS, mapping::read_ingredient)
    }

    pub fn recipes(&self) -> Result<Vec<Recipe>> {
        self.keyed(schema::root::RECIPES, mapping::read_recipe)
    }

    pub fn users(&self) -> Result<Vec<User>> {
        self.keyed(schema::root::USERS, mapping::read_user)
    }

    pub fn devices(&self) -> Result<Vec<Device>> {
        self.keyed(schema::root::DEVICES, mapping::read_device)
    }

    /// The shopping list, in the order the CRDT holds it.
    pub fn list(&self) -> Result<ShoppingList> {
        let value = self
            .doc
            .get_movable_list(schema::root::LIST)
            .get_deep_value();
        let entries = codec::list(&value, schema::root::LIST)?
            .iter()
            .enumerate()
            .map(|(i, v)| mapping::read_list_entry(v, &format!("{}[{i}]", schema::root::LIST)))
            .collect::<Result<Vec<_>>>()?;
        Ok(ShoppingList { entries })
    }

    /// Explicit actions only — never a derived state (Rule 3).
    pub fn overlay(&self) -> Result<Overlay> {
        let value = self.doc.get_map(schema::root::OVERLAY).get_deep_value();
        let map = codec::map(&value, schema::root::OVERLAY)?;
        let mut overlay = Overlay::new();
        for (id, entry) in map.iter() {
            let path = format!("{}.{id}", schema::root::OVERLAY);
            overlay.insert(
                IngredientId::from_raw(id.as_str()),
                mapping::read_explicit(entry, &path)?,
            );
        }
        Ok(overlay)
    }

    pub fn events(&self) -> Result<EventLog> {
        let value = self.doc.get_list(schema::root::EVENTS).get_deep_value();
        let events = codec::list(&value, schema::root::EVENTS)?
            .iter()
            .enumerate()
            .map(|(i, v)| mapping::read_event(v, &format!("{}[{i}]", schema::root::EVENTS)))
            .collect::<Result<Vec<_>>>()?;
        Ok(EventLog { events })
    }

    fn keyed<T>(&self, root: &str, read: impl Fn(&str, &LoroValue) -> Result<T>) -> Result<Vec<T>> {
        let value = self.doc.get_map(root).get_deep_value();
        let map = codec::map(&value, root)?;
        let mut ids: Vec<&String> = map.keys().collect();
        ids.sort_unstable();
        ids.into_iter()
            .map(|id| {
                let entry = map.get(id).expect("key came from this map");
                read(id, entry)
            })
            .collect()
    }

    // --- writes -------------------------------------------------------------

    /// Creates or updates an ingredient. Only the fields that actually
    /// changed become operations.
    pub fn put_ingredient(&self, ingredient: &Ingredient) -> Result<()> {
        let entry = self.entry(schema::root::INGREDIENTS, ingredient.id.as_str())?;
        mapping::write_ingredient(&entry, ingredient)?;
        self.doc.commit();
        Ok(())
    }

    /// Removes an ingredient.
    ///
    /// Nothing checks whether a recipe still refers to it, deliberately:
    /// referential integrity is not enforceable under a CRDT — one device
    /// deletes while another references — so the domain reports dangling
    /// references instead of preventing them (DECISIONS 0022).
    pub fn remove_ingredient(&self, id: &IngredientId) -> Result<()> {
        self.remove(schema::root::INGREDIENTS, id.as_str())
    }

    pub fn put_recipe(&self, recipe: &Recipe) -> Result<()> {
        let entry = self.entry(schema::root::RECIPES, recipe.id.as_str())?;
        mapping::write_recipe(&entry, recipe)?;
        self.doc.commit();
        Ok(())
    }

    pub fn remove_recipe(&self, id: &RecipeId) -> Result<()> {
        self.remove(schema::root::RECIPES, id.as_str())
    }

    pub fn put_user(&self, user: &User) -> Result<()> {
        let entry = self.entry(schema::root::USERS, user.id.as_str())?;
        mapping::write_user(&entry, user)?;
        self.doc.commit();
        Ok(())
    }

    pub fn put_device(&self, device: &Device) -> Result<()> {
        let entry = self.entry(schema::root::DEVICES, device.id.as_str())?;
        mapping::write_device(&entry, device)?;
        self.doc.commit();
        Ok(())
    }

    pub fn add_list_entry(&self, entry: &ListEntry) -> Result<()> {
        let list = self.doc.get_movable_list(schema::root::LIST);
        list.push(mapping::list_entry_value(entry)).map_err(crdt)?;
        self.doc.commit();
        Ok(())
    }

    /// Replaces an entry in place, keeping its position in the list.
    ///
    /// Rewrites the whole value rather than one field, which is what the
    /// schema already implies for a list entry: it is a plain value map, so
    /// two concurrent edits of the same entry resolve last-writer-wins
    /// (DECISIONS 0029). That is the right trade here — the edit is "serve 6
    /// instead of 4", and there is no half of it worth merging — but it is
    /// also why this is `set` and not a re-`push`: re-adding would move the
    /// entry to the end of the list, and changing a serving count must not
    /// reorder the shopping list under the other person's thumb.
    ///
    /// An entry that is no longer there is not an error: the other device may
    /// have removed it, and the domain reports rather than prevents that
    /// (DECISIONS 0022).
    pub fn update_list_entry(&self, entry: &ListEntry) -> Result<()> {
        let list = self.doc.get_movable_list(schema::root::LIST);
        for index in self.list_positions(&list, &entry.id)? {
            list.set(index, mapping::list_entry_value(entry))
                .map_err(crdt)?;
        }
        self.doc.commit();
        Ok(())
    }

    pub fn remove_list_entry(&self, id: &ListEntryId) -> Result<()> {
        let list = self.doc.get_movable_list(schema::root::LIST);
        // Back to front, so each removal leaves the earlier indices valid.
        for index in self.list_positions(&list, id)?.into_iter().rev() {
            list.delete(index, 1).map_err(crdt)?;
        }
        self.doc.commit();
        Ok(())
    }

    /// Records an explicit check or uncheck.
    ///
    /// `Unchecked` is stored, not represented by absence — see
    /// [`mapping::explicit_value`] for why that distinction is load-bearing.
    pub fn set_explicit(&self, ingredient: &IngredientId, explicit: &Explicit) -> Result<()> {
        let overlay = self.doc.get_map(schema::root::OVERLAY);
        mapping::set(
            &overlay,
            ingredient.as_str(),
            mapping::explicit_value(explicit),
        )?;
        self.doc.commit();
        Ok(())
    }

    /// Drops an explicit action so the line returns to its derived default.
    ///
    /// This is the purge Rule 3 requires when an ingredient is added to the
    /// list by hand, and the pruning `finish_shopping` performs.
    pub fn clear_explicit(&self, ingredient: &IngredientId) -> Result<()> {
        self.remove(schema::root::OVERLAY, ingredient.as_str())
    }

    pub fn record_event(&self, event: &Event) -> Result<()> {
        let events = self.doc.get_list(schema::root::EVENTS);
        events.push(mapping::event_value(event)).map_err(crdt)?;
        self.trim_events()?;
        self.doc.commit();
        Ok(())
    }

    // --- internals ----------------------------------------------------------

    /// The container holding one entity, created on first write.
    ///
    /// `ensure_mergeable_map`, never `get_or_create_container`: the latter
    /// gives the new child an operation-derived id, so two devices that each
    /// create the same ingredient while offline end up with **two different
    /// containers under one key**, and the merge keeps one and silently drops
    /// the other's fields. The mergeable form derives the child's id from the
    /// key, so the two creations are the same container and their edits
    /// combine — which is the entire reason for using a CRDT here.
    fn entry(&self, root: &str, id: &str) -> Result<LoroMap> {
        self.doc
            .get_map(root)
            .ensure_mergeable_map(id)
            .map_err(crdt)
    }

    fn remove(&self, root: &str, id: &str) -> Result<()> {
        self.doc.get_map(root).delete(id).map_err(crdt)?;
        self.doc.commit();
        Ok(())
    }

    fn list_positions(&self, list: &LoroMovableList, id: &ListEntryId) -> Result<Vec<usize>> {
        let value = list.get_deep_value();
        let entries = codec::list(&value, schema::root::LIST)?;
        let mut found = Vec::new();
        for (index, entry) in entries.iter().enumerate() {
            let path = format!("{}[{index}]", schema::root::LIST);
            if mapping::read_list_entry(entry, &path)?.id == *id {
                found.push(index);
            }
        }
        Ok(found)
    }

    fn trim_events(&self) -> Result<()> {
        let events: LoroList = self.doc.get_list(schema::root::EVENTS);
        let len = events.len();
        if len > EventLog::CAP {
            events.delete(0, len - EventLog::CAP).map_err(crdt)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cabas_domain::event::{Action, Subject};
    use cabas_domain::units::{MassUnit, Unit};
    use cabas_domain::{Aisle, Quantity, Rational, Timestamp, UserId};

    /// Walks every value in the document, containers included.
    fn walk(value: &LoroValue, visit: &mut impl FnMut(&LoroValue)) {
        visit(value);
        match value {
            LoroValue::Map(map) => map.values().for_each(|v| walk(v, visit)),
            LoroValue::List(list) => list.iter().for_each(|v| walk(v, visit)),
            _ => {}
        }
    }

    fn document_values(doc: &Document) -> Vec<LoroValue> {
        let mut found = Vec::new();
        for root in [
            schema::root::META,
            schema::root::INGREDIENTS,
            schema::root::RECIPES,
            schema::root::OVERLAY,
            schema::root::USERS,
            schema::root::DEVICES,
        ] {
            walk(&doc.doc.get_map(root).get_deep_value(), &mut |v| {
                found.push(v.clone())
            });
        }
        walk(
            &doc.doc
                .get_movable_list(schema::root::LIST)
                .get_deep_value(),
            &mut |v| found.push(v.clone()),
        );
        walk(
            &doc.doc.get_list(schema::root::EVENTS).get_deep_value(),
            &mut |v| found.push(v.clone()),
        );
        found
    }

    #[test]
    fn no_float_ever_reaches_the_document() {
        // Rule 4 is only as strong as its weakest serialiser. `LoroValue` has
        // a `Double` variant and no exact numeric type, so the one way to be
        // sure quantities stay exact is to assert that variant never appears.
        let doc = Document::new();
        doc.put_ingredient(
            &Ingredient::new(IngredientId::from_raw("flour"), "Flour", Aisle::Grocery)
                .with_density(Rational::new(55, 100)),
        )
        .expect("write");
        doc.put_recipe(
            &Recipe::new(
                RecipeId::from_raw("tart"),
                "Tart",
                std::num::NonZeroU32::new(4).expect("non-zero"),
            )
            .with_yield(Quantity::new(
                Rational::new(1, 3),
                Unit::Mass(MassUnit::Gram),
            )),
        )
        .expect("write");
        doc.record_event(&Event::new(
            Timestamp(1),
            UserId::from_raw("alice"),
            Action::Deleted,
            Subject::Recipe(RecipeId::from_raw("gone")),
            "Gone",
        ))
        .expect("write");

        for value in document_values(&doc) {
            assert!(
                !matches!(value, LoroValue::Double(_)),
                "a float reached the document: {value:?}"
            );
        }
    }

    #[test]
    fn a_document_from_the_future_is_refused_rather_than_half_read() {
        // Reading it partially would drop the fields this build cannot see,
        // and the next save would propagate that loss to every other device.
        let doc = Document::new();
        let meta = doc.doc.get_map(schema::root::META);
        mapping::set(
            &meta,
            schema::meta::SCHEMA,
            LoroValue::I64(SCHEMA_VERSION + 1),
        )
        .expect("stamp");
        doc.doc.commit();

        let bytes = doc.snapshot().expect("snapshot");
        match Document::load(&bytes) {
            Err(StoreError::FutureSchema { found, supported }) => {
                assert_eq!(found, SCHEMA_VERSION + 1);
                assert_eq!(supported, SCHEMA_VERSION);
            }
            other => panic!("expected FutureSchema, got {other:?}"),
        }
    }

    #[test]
    fn a_new_document_is_stamped_with_the_schema_version() {
        let doc = Document::new();
        assert!(doc.check_schema().is_ok());
        let value = doc.doc.get_map(schema::root::META).get_deep_value();
        let map = codec::map(&value, "meta").expect("a map");
        assert_eq!(
            codec::optional(map, schema::meta::SCHEMA),
            Some(&LoroValue::I64(SCHEMA_VERSION))
        );
    }

    #[test]
    fn the_event_log_stays_capped_across_merges() {
        let doc = Document::new();
        for i in 0..(EventLog::CAP as i64 + 40) {
            doc.record_event(&Event::new(
                Timestamp(i),
                UserId::from_raw("alice"),
                Action::Edited,
                Subject::Recipe(RecipeId::from_raw("r")),
                format!("edit {i}"),
            ))
            .expect("record");
        }
        assert_eq!(doc.events().expect("read").events.len(), EventLog::CAP);
    }
}
