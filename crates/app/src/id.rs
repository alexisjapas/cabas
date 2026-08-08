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

/// Mints the id of a recipe line, for a host that has to name one before the
/// recipe it belongs to has ever been saved (DECISIONS 0039).
///
/// The one id an editor cannot wait for. A step references a *usage* rather
/// than an ingredient (DECISIONS 0022), so mentioning a line the user has just
/// added means naming it, and [`crate::Command::SaveRecipe`] does not return
/// until every line already carries the name the steps used. The host mints it
/// up front and sends it back as the line's `id`.
///
/// It comes from here rather than from the host's own random source for the
/// reason the module note gives: two devices adding a line to the same recipe
/// offline must not choose the same id, and a locally invented `"line-1"`
/// would guarantee that they do — which under a CRDT is not a conflict but two
/// different lines silently merging into one.
pub fn mint_usage_id(platform: &impl Platform) -> Result<String> {
    mint(platform, USAGE)
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
    fn a_host_minted_usage_id_is_an_ordinary_usage_id() {
        // The editor's ids must be indistinguishable from the ones `component`
        // mints, or a line created in the editor would be a second kind of id
        // for something the document treats as one kind (DECISIONS 0039).
        let platform = Sequence(Cell::new(0));
        let host = mint_usage_id(&platform).expect("mint");
        let internal = mint(&platform, USAGE).expect("mint");
        assert!(host.starts_with(USAGE));
        assert_eq!(host.len(), internal.len());
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
