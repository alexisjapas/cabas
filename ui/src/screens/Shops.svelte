<script lang="ts">
  /**
   * The shops the group buys from — the maintenance door (DECISIONS 0071).
   *
   * Shops are created where they are needed, in the field on an ingredient's
   * form, and that is deliberately the only place they are *born*. This is
   * where a typo is corrected and an abandoned one is forgotten, because a
   * library with no way to fix a name is a library that fills up with
   * "Biocoop", "biocoop " and "Bicoop".
   *
   * Forgetting one changes nothing about the ingredients that named it: the
   * dangling id matches no shop, so it filters nothing, and the line goes
   * back to being sold everywhere. That is the same bargain every delete in
   * this app makes (DECISIONS 0022).
   */
  import Screen from '../components/Screen.svelte';
  import type { ShopView } from '../lib/bindings/ShopView';
  import { byName } from '../lib/format';
  import type { Session } from '../lib/session.svelte';

  let { session, onback }: { session: Session; onback: () => void } = $props();

  let shops = $derived([...session.state.shops].sort(byName));

  /**
   * How many ingredients name each shop, so that forgetting one says what it
   * costs before it happens.
   */
  let counts = $derived.by(() => {
    const found = new Map<string, number>();
    for (const ingredient of session.state.ingredients) {
      for (const shop of ingredient.shops) found.set(shop, (found.get(shop) ?? 0) + 1);
    }
    return found;
  });

  /**
   * The name being typed, and which shop it belongs to.
   *
   * The same shape `Settings.svelte` uses for the person's name, and for the
   * same reason: a `$state` seeded from the view captures the value once and
   * would ignore a rename arriving from the other phone while this screen is
   * open. The draft carries its shop id so that opening another one drops it.
   */
  let edited = $state<{ shop: string; name: string } | null>(null);
  let confirming = $state<string | null>(null);

  function nameOf(shop: ShopView): string {
    return edited !== null && edited.shop === shop.id ? edited.name : shop.name;
  }

  function rename(shop: ShopView): void {
    const name = nameOf(shop).trim();
    if (name === '' || name === shop.name) return;
    if (session.run({ command: 'save_shop', shop: { id: shop.id, name } })) edited = null;
  }

  function forget(shop: ShopView): void {
    if (confirming !== shop.id) {
      confirming = shop.id;
      return;
    }
    if (session.run({ command: 'delete_shop', shop: shop.id })) confirming = null;
  }
</script>

<Screen
  title="Magasins"
  subtitle={shops.length === 0 ? 'Aucun magasin' : `${shops.length} référencés`}
  {onback}
>
  {#if shops.length === 0}
    <p class="empty">
      Les magasins se créent depuis la fiche d'un ingrédient, là où on dit où il s'achète.
    </p>
  {/if}

  <ul>
    {#each shops as shop (shop.id)}
      {@const used = counts.get(shop.id) ?? 0}
      <li>
        <input
          value={nameOf(shop)}
          data-field="shop-name"
          autocomplete="off"
          onblur={() => rename(shop)}
          oninput={(event) => (edited = { shop: shop.id, name: event.currentTarget.value })}
          onkeydown={(event) => {
            if (event.key !== 'Enter') return;
            event.preventDefault();
            rename(shop);
          }}
        />
        <p class="meta">
          {used === 0 ? 'Aucun ingrédient' : used === 1 ? '1 ingrédient' : `${used} ingrédients`}
        </p>
        <button
          type="button"
          class="forget"
          class:confirming={confirming === shop.id}
          onclick={() => forget(shop)}
        >
          {confirming === shop.id ? 'Confirmer' : 'Oublier'}
        </button>
      </li>
    {/each}
  </ul>

  <p class="note">
    Oublier un magasin ne touche à aucun ingrédient : ceux qui le citaient redeviennent
    disponibles partout.
  </p>
</Screen>

<style>
  .empty {
    margin: var(--space-6) 0;
    color: var(--text-muted);
    text-align: center;
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  li {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: var(--space-2) var(--space-3);
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
  }

  input {
    grid-column: 1;
    min-width: 0;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
  }

  .meta {
    grid-column: 1;
    margin: 0;
    color: var(--text-faint);
    font-size: var(--text-xs);
  }

  .forget {
    grid-column: 2;
    grid-row: 1 / span 2;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--danger);
    border-radius: var(--radius-md);
    background: none;
    color: var(--danger);
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .forget.confirming {
    background: var(--danger);
    color: var(--on-danger);
  }

  .note {
    margin-top: var(--space-5);
    color: var(--text-muted);
    font-size: var(--text-sm);
  }
</style>
