import type { ListEntryView } from './bindings/ListEntryView';
import { formatQuantity } from './format';

/**
 * What the shopping list already holds, indexed by the thing it came from.
 *
 * Every shelf a row can be pushed onto the list from asks the same questions
 * — is this one already there, which entry does an undo remove, and how much
 * of it did we say (DECISIONS 0067, 0072) — so the answer is derived once
 * here rather than per screen. Read from the core's own list rather than
 * remembered from the gesture, which is what makes a swiped row still offer
 * its way out after a reload, and makes it stop offering one the moment the
 * other phone takes the entry off the list.
 *
 * The whole entry rather than its id, since 0072: the row now shows what it
 * asks for beside "Annuler", and the long press opens that amount in a field.
 * Both are on the entry the core already sent, so neither is a second thing
 * to keep in step.
 *
 * One map serves both shelves: an ingredient id is never a recipe id, so
 * there is nothing to tell apart. The newest entry wins, because it is the
 * one an undo means.
 */
export function entriesBySource(list: ListEntryView[]): Map<string, ListEntryView> {
  const found = new Map<string, ListEntryView>();
  for (const entry of list) {
    const source = entry.item.kind === 'ingredient' ? entry.item.ingredient : entry.item.recipe;
    found.set(source, entry);
  }
  return found;
}

/**
 * What a swiped row says it holds, in the strip beside "Annuler"
 * (DECISIONS 0072).
 *
 * The two shelves word it differently because the two entries measure
 * different things — an amount of a thing, a number of people — and both
 * arrive already rendered by the core (Rule 4: the arithmetic happened in
 * Rust; the comma and the word are this side's). `null` for a row that is not
 * on the list, which is the strip that says "Ajouter à la liste" instead.
 */
export function badgeOf(entry: ListEntryView | undefined): string | null {
  if (entry === undefined) return null;
  return entry.item.kind === 'ingredient'
    ? formatQuantity(entry.item.quantity)
    : `${entry.item.servings} pers.`;
}
