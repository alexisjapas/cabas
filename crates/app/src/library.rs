//! The whole document, read into plain domain values.
//!
//! Every command and every render starts by materialising this. That is a
//! deliberate choice of clarity over cleverness: the alternative is an
//! in-memory model kept in step with the document by hand, which is two
//! representations of the same truth and therefore two chances to disagree —
//! and the one that disagrees would be the one on screen.
//!
//! It is affordable because M2 measured it: a 200-recipe library is 154 kB of
//! CRDT and reads back in about 10 ms natively. The wasm figure is some
//! multiple of that and gets measured on the actual phone at M4, which is the
//! only place the number means anything. If it turns out to be too slow
//! there, the fix is an incremental read *behind this same seam* — which is
//! the other reason for putting it behind one.

use cabas_domain::{
    Device, IngredientId, IngredientIndex, Overlay, RecipeIndex, ShoppingList, User, UserId,
};
use cabas_store::Document;

use crate::error::Result;

pub(crate) struct Library {
    pub recipes: RecipeIndex,
    pub ingredients: IngredientIndex,
    pub list: ShoppingList,
    pub overlay: Overlay,
    pub users: Vec<User>,
    /// The devices those people carry. Read for the device screen, which is
    /// where Rule 7 has to be said out loud.
    pub devices: Vec<Device>,
}

impl Library {
    pub(crate) fn read(document: &Document) -> Result<Self> {
        Ok(Self {
            recipes: document
                .recipes()?
                .into_iter()
                .map(|recipe| (recipe.id.clone(), recipe))
                .collect(),
            ingredients: document
                .ingredients()?
                .into_iter()
                .map(|ingredient| (ingredient.id.clone(), ingredient))
                .collect(),
            list: document.list()?,
            overlay: document.overlay()?,
            users: document.users()?,
            devices: document.devices()?,
        })
    }

    /// The name to show for a person, or `None` if that user is not in the
    /// document — which a concurrent delete can perfectly well arrange.
    pub(crate) fn user_name(&self, id: &UserId) -> Option<&str> {
        self.users
            .iter()
            .find(|user| &user.id == id)
            .map(|user| user.name.as_str())
    }

    pub(crate) fn ingredient_name(&self, id: &IngredientId) -> Option<&str> {
        self.ingredients
            .get(id)
            .map(|ingredient| ingredient.name.as_str())
    }
}
