import type { ListEntryView } from './bindings/ListEntryView';

/**
 * What the shopping list already holds, indexed by the thing it came from.
 *
 * Every shelf a row can be pushed onto the list from asks the same two
 * questions — is this one already there, and which entry does an undo remove
 * (DECISIONS 0067) — so the answer is derived once here rather than per
 * screen. Read from the core's own list rather than remembered from the
 * gesture, which is what makes a swiped row still offer its way out after a
 * reload, and makes it stop offering one the moment the other phone takes
 * the entry off the list.
 *
 * One map serves both shelves: an ingredient id is never a recipe id, so
 * there is nothing to tell apart. The newest entry wins, because it is the
 * one an undo means.
 */
export function entriesBySource(list: ListEntryView[]): Map<string, string> {
  const found = new Map<string, string>();
  for (const entry of list) {
    const source = entry.item.kind === 'ingredient' ? entry.item.ingredient : entry.item.recipe;
    found.set(source, entry.id);
  }
  return found;
}
