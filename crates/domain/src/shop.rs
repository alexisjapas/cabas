//! The shops the group buys from.
//!
//! A shop is a **name and nothing else** (DECISIONS 0071). It has no address,
//! no opening hours and no aisle order of its own: what it is for is
//! answering "can I get this here", and every one of those extras would be a
//! second thing to keep up to date for an answer that does not change.
//!
//! Called `Shop` rather than `Store` on purpose — `cabas-store` is the
//! persistence crate, and a `Store` inside it would be two unrelated things
//! under one word in the file that maps between them.

use crate::ShopId;
use crate::ingredient::Ingredient;

/// One place things are bought.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shop {
    pub id: ShopId,
    pub name: String,
}

impl Shop {
    pub fn new(id: ShopId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
        }
    }

    /// Does `name` denote this shop? Case-insensitive, and trimmed.
    ///
    /// The one place it is asked is a field where somebody typed a shop's
    /// name: creating "Biocoop" a second time because the first was typed
    /// "biocoop " is the failure this exists to prevent, and it is the same
    /// bargain [`crate::Ingredient::matches`] makes.
    pub fn matches(&self, name: &str) -> bool {
        self.name.trim().eq_ignore_ascii_case(name.trim())
    }
}

/// Which of `shops` an ingredient belongs to — the trips its line appears on
/// (DECISIONS 0071).
///
/// **Two different silences both answer "every shop".** An ingredient that
/// names none has not been classified; one whose every named shop has since
/// been forgotten has been classified and then orphaned. Neither is a
/// statement that the thing is unavailable, and hiding such a line from the
/// only screen that would have prompted somebody to place it is how a
/// shopping list quietly loses an item.
///
/// It lives here rather than as a method on [`crate::Ingredient`] because it
/// needs the shop library to answer at all — which is exactly what makes the
/// orphaned case decidable. The frontend then filters by plain membership and
/// carries no rule of its own (Rule 9).
pub fn sold_at<'a>(ingredient: &Ingredient, shops: &'a [Shop]) -> Vec<&'a Shop> {
    let named: Vec<&Shop> = shops
        .iter()
        .filter(|shop| ingredient.shops.contains(&shop.id))
        .collect();
    if named.is_empty() {
        shops.iter().collect()
    } else {
        named
    }
}

/// Resolves typed text to a shop the group already has.
///
/// Stops at "not found" rather than creating one, like
/// [`crate::ingredient::resolve`]: minting is a write and this crate performs
/// none (Rule 1).
pub fn resolve<'a, I>(candidates: I, text: &str) -> Option<&'a Shop>
where
    I: IntoIterator<Item = &'a Shop>,
{
    candidates.into_iter().find(|shop| shop.matches(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shops() -> Vec<Shop> {
        vec![
            Shop::new(ShopId::from_raw("s1"), "Biocoop"),
            Shop::new(ShopId::from_raw("s2"), "Marché"),
        ]
    }

    #[test]
    fn an_ingredient_belongs_to_the_shops_it_names() {
        use crate::IngredientId;
        use crate::ingredient::Aisle;

        let held = shops();
        let named = |ingredient: &Ingredient| -> Vec<String> {
            sold_at(ingredient, &held)
                .into_iter()
                .map(|shop| shop.id.to_string())
                .collect()
        };

        let placed = Ingredient::new(IngredientId::from_raw("i"), "Beurre", Aisle::Dairy)
            .sold_at([ShopId::from_raw("s1")]);
        assert_eq!(named(&placed), ["s1"]);
    }

    #[test]
    fn an_ingredient_nobody_has_placed_belongs_to_every_trip() {
        use crate::IngredientId;
        use crate::ingredient::Aisle;

        let held = shops();
        let unplaced = Ingredient::new(IngredientId::from_raw("i"), "Beurre", Aisle::Dairy);
        assert_eq!(sold_at(&unplaced, &held).len(), 2);

        // And so does one whose shops have all been forgotten since: it was
        // classified, the classification is gone, and refusing to show it
        // would lose it (DECISIONS 0071).
        let orphaned = unplaced.clone().sold_at([ShopId::from_raw("gone")]);
        assert_eq!(sold_at(&orphaned, &held).len(), 2);

        // With no shops at all there are no trips to belong to.
        assert!(sold_at(&unplaced, &[]).is_empty());
    }

    #[test]
    fn a_shop_is_found_however_it_was_typed() {
        let held = shops();
        assert_eq!(
            resolve(&held, "  biocoop ").map(|s| s.id.clone()),
            Some(ShopId::from_raw("s1"))
        );
        // Unknown text is reported, never minted here (Rule 1).
        assert!(resolve(&held, "Grand Frais").is_none());
    }
}
