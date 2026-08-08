//! Minting identifiers.
//!
//! Ids are opaque everywhere else in the workspace (see the `id_type!` macro
//! in `domain`), so this module is the only place their shape is decided.
//!
//! **Random, not sequential.** Two devices offline at the same time both mint
//! ids, and a counter would have them both mint the same one — which under a
//! CRDT is not a conflict but a silent merge of two different recipes into
//! one. Sixty-four bits makes that collision impossible in practice for a
//! library of a few thousand entities.
//!
//! The prefix is for humans: a raw document dump, a log line or a sync trace
//! is much easier to read when `rec_…` is visibly not `ing_…`. Nothing parses
//! it, and nothing may start to — an id that carries meaning is an id that
//! cannot be changed.

use crate::error::Result;
use crate::platform::Platform;

pub(crate) const INGREDIENT: &str = "ing_";
pub(crate) const RECIPE: &str = "rec_";
pub(crate) const USAGE: &str = "use_";
pub(crate) const LIST_ENTRY: &str = "ent_";
pub(crate) const USER: &str = "usr_";
pub(crate) const DEVICE: &str = "dev_";

pub(crate) fn mint(platform: &impl Platform, prefix: &str) -> Result<String> {
    Ok(format!("{prefix}{:016x}", platform.random_u64()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cabas_domain::Timestamp;
    use std::cell::Cell;

    struct Sequence(Cell<u64>);

    impl Platform for Sequence {
        fn now(&self) -> Timestamp {
            Timestamp(0)
        }

        fn random_u64(&self) -> Result<u64> {
            self.0.set(self.0.get() + 1);
            Ok(self.0.get())
        }
    }

    #[test]
    fn ids_are_prefixed_and_fixed_width() {
        let platform = Sequence(Cell::new(0));
        let id = mint(&platform, RECIPE).expect("mint");
        assert_eq!(id, "rec_0000000000000001");
        // Fixed width: ids sort the same way whatever the value, which is
        // what keeps `Document`'s by-id ordering meaningful.
        assert_eq!(mint(&platform, RECIPE).expect("mint").len(), id.len());
    }

    #[test]
    fn every_prefix_is_distinct() {
        let mut prefixes = [INGREDIENT, RECIPE, USAGE, LIST_ENTRY, USER, DEVICE];
        prefixes.sort_unstable();
        let before = prefixes.len();
        let mut unique = prefixes.to_vec();
        unique.dedup();
        assert_eq!(unique.len(), before);
    }
}
