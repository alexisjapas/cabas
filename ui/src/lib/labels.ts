/**
 * The French. All of it, and only here.
 *
 * The core sends tags — `"kg"`, `"produce"`, `"missing_recipe"` — and never a
 * word a person reads (DECISIONS 0035). That is what keeps every quantity,
 * conversion and rounding in Rust where the tests are, and what makes a
 * translation one day a change to this file rather than to the core.
 *
 * Each table is a total `Record` over its tag union on purpose: adding a
 * variant in Rust regenerates the type under `./bindings/`, and this file
 * stops compiling until somebody writes the word for it.
 */

import type { AisleTag } from './bindings/AisleTag';
import type { CheckStateTag } from './bindings/CheckStateTag';
import type { ProblemKind } from './bindings/ProblemKind';
import type { UnitTag } from './bindings/UnitTag';

/**
 * Units as they are written on a French recipe.
 *
 * `piece` is empty: "5 tomates" is what a person writes, not "5 pièces de
 * tomates". `to_taste` carries no number at all and is handled as a whole
 * phrase by `formatQuantity`.
 */
export const UNIT_LABEL: Record<UnitTag, string> = {
  mg: 'mg',
  g: 'g',
  kg: 'kg',
  oz: 'oz',
  lb: 'lb',
  ml: 'ml',
  cl: 'cl',
  dl: 'dl',
  l: 'l',
  tsp_fr: 'c. à c.',
  tbsp_fr: 'c. à s.',
  cup_metric: 'tasse',
  // The US measures keep their marker: a recipe mixing the two is exactly the
  // case where dropping it silently changes the amount (DECISIONS 0016).
  tsp_us: 'c. à c. US',
  tbsp_us: 'c. à s. US',
  cup_us: 'cup US',
  floz_us: 'fl oz US',
  piece: '',
  pinch: 'pincée',
  to_taste: 'au goût',
};

/** Only the units that are words. Symbols are invariable. */
export const UNIT_PLURAL: Partial<Record<UnitTag, string>> = {
  cup_metric: 'tasses',
  cup_us: 'cups US',
  pinch: 'pincées',
};

/**
 * Aisles, in the words of a French supermarket. The order they are shown in
 * is the walking order, and it comes from the core already sorted — this
 * table names them and nothing more.
 */
export const AISLE_LABEL: Record<AisleTag, string> = {
  produce: 'Fruits et légumes',
  butcher: 'Boucherie',
  fish: 'Poissonnerie',
  deli: 'Charcuterie, traiteur',
  dairy: 'Crèmerie',
  bakery: 'Boulangerie',
  grocery: 'Épicerie',
  frozen: 'Surgelés',
  beverages: 'Boissons',
  household: 'Entretien',
  other: 'Autres',
};

/**
 * The three cart sections. "Acheté" and "Déjà à la maison" are kept apart
 * because they do not make the same statement: one is something you picked
 * up, the other something you never needed to (DECISIONS 0023).
 */
export const CHECK_STATE_LABEL: Record<CheckStateTag, string> = {
  to_buy: 'À prendre',
  checked: 'Acheté',
  auto_checked: 'Déjà à la maison',
};

/**
 * What a problem means, in a sentence. Always the shape of a concurrent edit,
 * so each one says what happened rather than what is broken — the row stays
 * on screen and the rest of the cart still works (DECISIONS 0034).
 */
export const PROBLEM_LABEL: Record<ProblemKind, string> = {
  missing_recipe: 'Cette recette a été supprimée sur un autre appareil.',
  missing_ingredient: 'Un ingrédient de cette recette a été supprimé.',
  broken_graph: 'Des sous-recettes se référencent en boucle.',
  broken_yield: "Une sous-recette n'indique pas de rendement utilisable.",
};

/** Aisles in the order the pickers offer them — the shop's walking order. */
export const AISLES: readonly AisleTag[] = [
  'produce',
  'butcher',
  'fish',
  'deli',
  'dairy',
  'bakery',
  'grocery',
  'frozen',
  'beverages',
  'household',
  'other',
];

/**
 * The units a picker offers, grouped the way a person thinks about them.
 * `to_taste` and `pinch` sit with the counts: they answer "how much" without
 * measuring anything.
 */
export const UNIT_GROUPS: readonly { label: string; units: readonly UnitTag[] }[] = [
  { label: 'Masse', units: ['g', 'kg', 'mg', 'oz', 'lb'] },
  { label: 'Volume', units: ['ml', 'cl', 'dl', 'l'] },
  { label: 'Cuillères et tasses', units: ['tsp_fr', 'tbsp_fr', 'cup_metric'] },
  { label: 'Mesures US', units: ['tsp_us', 'tbsp_us', 'cup_us', 'floz_us'] },
  { label: 'Sans mesure', units: ['piece', 'pinch', 'to_taste'] },
];
