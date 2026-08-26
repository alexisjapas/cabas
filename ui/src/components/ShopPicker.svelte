<script lang="ts">
  /**
   * Where a thing can be bought (DECISIONS 0071).
   *
   * A field with the shops already chosen sitting above it as chips, and the
   * matches for what is being typed below it. Typing a name nothing matches
   * offers to create it — which is the whole point: the shop library exists
   * so that "Biocoop" typed on the second ingredient is the same shop as on
   * the first, and nobody is going to visit a settings screen first to make
   * that true.
   *
   * # Why the matches are inline and not a panel
   *
   * `SearchPicker` floats its list, and pays for it with three behaviours
   * that each fail silently on their own — a capture-phase outside-click, a
   * prevented `mousedown`, an Enter that always `preventDefault`s (DECISIONS
   * 0058). All of that exists because the field there *is* the form's value
   * and must read back as a real name when the panel closes. Here the field
   * is a search box and nothing else: the value is the chips, which are
   * always visible. So the list is simply below the field while something is
   * typed, and there is no closing to get wrong.
   *
   * Enter is still caught, because this renders inside a form and would
   * otherwise submit it.
   */

  import type { ShopView } from '../lib/bindings/ShopView';
  import { mintShopId } from '../lib/core';
  import { byName, matches } from '../lib/format';
  import type { Session } from '../lib/session.svelte';

  let {
    session,
    selected = $bindable(),
    onkeydown = undefined,
  }: {
    session: Session;
    /** Shop ids, in the order they are to be read. */
    selected: string[];
    /**
     * Given by a caller that is not a `<form>` and lives inside one —
     * `IngredientForm` (DECISIONS 0056). Enter is handled here either way;
     * this is what lets the panel around it save on the same key.
     */
    onkeydown?: ((event: KeyboardEvent) => void) | undefined;
  } = $props();

  let query = $state('');

  let shops = $derived([...session.state.shops].sort(byName));
  let chosen = $derived(
    selected
      .map((id) => shops.find((shop) => shop.id === id))
      .filter((shop): shop is ShopView => shop !== undefined),
  );

  let offered = $derived(
    query.trim() === ''
      ? []
      : shops.filter((shop) => !selected.includes(shop.id) && matches(shop.name, query)),
  );

  /** Whether what is typed already *is* a shop, however it was capitalised. */
  let exact = $derived(
    shops.some((shop) => shop.name.trim().toLowerCase() === query.trim().toLowerCase()),
  );

  function add(id: string): void {
    if (!selected.includes(id)) selected = [...selected, id];
    query = '';
  }

  function drop(id: string): void {
    selected = selected.filter((held) => held !== id);
  }

  /**
   * Creates the shop and selects it, in that order and in one gesture.
   *
   * The id is minted here rather than left to the core for the reason
   * DECISIONS 0056 gives at the ingredient picker: `SaveShop` hands back a
   * whole state and not the id it wrote, and this field has to put that id on
   * the draft it is sitting in the moment it exists.
   */
  function create(): void {
    const name = query.trim();
    if (name === '') return;
    const id = mintShopId();
    if (session.run({ command: 'save_shop', shop: { id, name } })) add(id);
  }

  /**
   * Enter picks the obvious thing: the single match if there is one, the new
   * shop otherwise. It never submits — the form around this one is the
   * ingredient being written, and saving it half-way through typing a shop
   * name is the failure this guard exists for.
   */
  function key(event: KeyboardEvent): void {
    if (event.key !== 'Enter') {
      onkeydown?.(event);
      return;
    }
    event.preventDefault();
    if (query.trim() === '') return;
    const only = offered.length === 1 ? offered[0] : undefined;
    if (only !== undefined) add(only.id);
    else if (!exact) create();
  }
</script>

<div class="shops">
  <span class="label" id="shops-label">Magasins</span>

  {#if chosen.length > 0}
    <ul class="chosen">
      {#each chosen as shop (shop.id)}
        <li>
          <span>{shop.name}</span>
          <button type="button" aria-label="Retirer {shop.name}" onclick={() => drop(shop.id)}
            >×</button
          >
        </li>
      {/each}
    </ul>
  {/if}

  <input
    bind:value={query}
    data-field="shops"
    placeholder="Biocoop, marché…"
    autocomplete="off"
    aria-labelledby="shops-label"
    onkeydown={key}
  />

  {#if query.trim() !== ''}
    <ul class="offered">
      {#each offered as shop (shop.id)}
        <li>
          <button type="button" onclick={() => add(shop.id)}>{shop.name}</button>
        </li>
      {/each}
      {#if !exact}
        <li>
          <button type="button" class="create" data-field="create-shop" onclick={create}>
            + Nouveau magasin «&nbsp;{query.trim()}&nbsp;»
          </button>
        </li>
      {/if}
    </ul>
  {/if}

  <small>Sert à trier les courses par magasin. Vide : disponible partout.</small>
</div>

<style>
  .shops {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .label {
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .chosen {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .chosen li {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-1) var(--space-1) var(--space-1) var(--space-3);
    border-radius: var(--radius-pill);
    background: var(--accent-soft);
    color: var(--accent);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  .chosen button {
    width: var(--space-5);
    height: var(--space-5);
    display: grid;
    place-items: center;
    border: 0;
    border-radius: var(--radius-pill);
    background: none;
    color: inherit;
    font-size: var(--text-base);
    line-height: 1;
    cursor: pointer;
  }

  input {
    padding: var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
  }

  .offered {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    overflow: hidden;
  }

  .offered li + li {
    border-top: 1px solid var(--border);
  }

  .offered button {
    width: 100%;
    padding: var(--space-3);
    min-height: var(--tapsize);
    border: 0;
    background: none;
    color: inherit;
    font-size: var(--text-sm);
    text-align: left;
    cursor: pointer;
  }

  .offered button:active {
    background: var(--surface-sunken);
  }

  .offered .create {
    color: var(--accent);
    font-weight: var(--weight-medium);
  }

  small {
    color: var(--text-muted);
    font-size: var(--text-xs);
  }
</style>
