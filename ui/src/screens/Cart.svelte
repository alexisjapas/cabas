<script lang="ts">
  import { flip } from 'svelte/animate';

  import CartLine from '../components/CartLine.svelte';
  import Screen from '../components/Screen.svelte';
  import type { AisleTag } from '../lib/bindings/AisleTag';
  import type { CartLineView } from '../lib/bindings/CartLineView';
  import { byName } from '../lib/format';
  import { AISLE_LABEL, CHECK_STATE_LABEL } from '../lib/labels';
  import type { Session } from '../lib/session.svelte';

  /**
   * The home screen, because it is what you open in a shop.
   *
   * Everything here is derived by the core and merely arranged: the lines
   * arrive already aggregated, converted and sorted into walking order, and
   * this screen never decides what a quantity is or whether something counts
   * as bought (Rule 9).
   *
   * The three sections are deliberate. "Acheté" is something you picked up;
   * "Déjà à la maison" is a staple you never needed to — merging them would
   * make unchecking one hard to discover (DECISIONS 0023).
   */
  let { session }: { session: Session } = $props();

  let cart = $derived(session.state.cart);
  let picked = $derived(cart.total - cart.remaining);

  /**
   * Which shop this trip is (DECISIONS 0071). `null` is "Tous" — one list,
   * the way it was before shops existed.
   *
   * Device-local and deliberately not remembered: a tab opens cold (0074),
   * and "which shop am I standing in" is the most perishable fact this app
   * holds. It dies with the component, which is what switching tabs does to
   * it.
   */
  let shop = $state<string | null>(null);

  /**
   * The shops worth offering: the ones something in the cart names. A shop
   * nothing on the list belongs to is a chip that filters to an empty screen.
   *
   * Read across the whole cart rather than only what is left to buy, so the
   * chips do not rearrange themselves under a thumb as things are ticked off.
   */
  let offered = $derived.by(() => {
    const named = new Set(
      [...cart.to_buy, ...cart.bought, ...cart.at_home].flatMap((line) => line.shops),
    );
    return session.state.shops.filter((held) => named.has(held.id)).sort(byName);
  });

  /**
   * Whether a line belongs to the trip being shown.
   *
   * Plain membership, and deliberately nothing more: `line.shops` arrives
   * already resolved, so an ingredient nobody has placed already names every
   * shop by the time it gets here (DECISIONS 0071). Writing the "empty means
   * everywhere" rule out again on this side would be a second copy of it, and
   * the two would drift.
   */
  function here(line: CartLineView): boolean {
    return shop === null || line.shops.includes(shop);
  }

  let inThisShop = $derived(cart.to_buy.filter(here));
  let elsewhere = $derived(cart.to_buy.filter((line) => !here(line)));

  type Group = { aisle: AisleTag; lines: CartLineView[] };

  /**
   * Aisle headings. The order is the core's — it sorted the lines into the
   * order you walk the shop in — so this only has to notice where it changes.
   *
   * Within an aisle the names are put back in *French* order: the core sorts
   * by code point, where "Œufs" and "Épinards" land after "Z". Which of the
   * two orders you want depends on whether the reader is a person
   * (`format.byName`), and here they are.
   */
  let groups = $derived.by(() => {
    const built: Group[] = [];
    for (const line of inThisShop) {
      const last = built[built.length - 1];
      if (last !== undefined && last.aisle === line.aisle) {
        last.lines.push(line);
      } else {
        built.push({ aisle: line.aisle, lines: [line] });
      }
    }
    for (const group of built) group.lines.sort(byName);
    return built;
  });

  let bought = $derived([...cart.bought].sort(byName));
  let atHome = $derived([...cart.at_home].sort(byName));

  function toggle(ingredient: string): void {
    session.run({ command: 'toggle_cart_item', ingredient });
  }

  /**
   * Two taps rather than a dialog. Finishing prunes the list and there is no
   * undo, but a native confirm in the middle of a shop is worse — this keeps
   * the guard and stays on the same screen.
   */
  let confirming = $state(false);

  function finish(): void {
    if (!confirming) {
      confirming = true;
      return;
    }
    session.run({ command: 'finish_shopping' });
    confirming = false;
  }
</script>

<Screen
  title="Courses"
  subtitle={cart.total === 0 ? 'Rien à acheter' : `${cart.remaining} à prendre sur ${cart.total}`}
>
  {#if cart.total > 0}
    <div class="progress" style="--picked: {(picked / cart.total) * 100}%">
      <div class="fill"></div>
    </div>
  {/if}

  {#if offered.length > 0}
    <div class="shops" role="group" aria-label="Magasin">
      <button type="button" class:on={shop === null} onclick={() => (shop = null)}>Tous</button>
      {#each offered as held (held.id)}
        <button type="button" class:on={shop === held.id} onclick={() => (shop = held.id)}>
          {held.name}
        </button>
      {/each}
    </div>
  {/if}

  {#if cart.total === 0}
    <p class="empty">
      Ajoutez une recette ou un ingrédient à la liste : le panier se remplit tout seul.
    </p>
  {:else if inThisShop.length === 0 && elsewhere.length > 0}
    <p class="empty">Rien à prendre ici. Le reste est plus bas.</p>
  {/if}

  {#each groups as group (group.aisle)}
    <section>
      <h2>{AISLE_LABEL[group.aisle]}</h2>
      <ul>
        {#each group.lines as line (line.ingredient)}
          <li animate:flip={{ duration: 180 }}>
            <CartLine {session} {line} ontoggle={() => toggle(line.ingredient)} />
          </li>
        {/each}
      </ul>
    </section>
  {/each}

  {#if elsewhere.length > 0}
    <details>
      <summary>Ailleurs ({elsewhere.length})</summary>
      <p class="hint">Ce que ce magasin ne vend pas. Toujours à prendre, mais pas ici.</p>
      <ul>
        {#each elsewhere as line (line.ingredient)}
          <li><CartLine {session} {line} ontoggle={() => toggle(line.ingredient)} /></li>
        {/each}
      </ul>
    </details>
  {/if}

  {#if cart.bought.length > 0}
    <details>
      <summary>{CHECK_STATE_LABEL.checked} ({cart.bought.length})</summary>
      <ul>
        {#each bought as line (line.ingredient)}
          <li><CartLine {session} {line} ontoggle={() => toggle(line.ingredient)} /></li>
        {/each}
      </ul>
    </details>
  {/if}

  {#if cart.at_home.length > 0}
    <details>
      <summary>{CHECK_STATE_LABEL.auto_checked} ({cart.at_home.length})</summary>
      <p class="hint">
        Des ingrédients de base, décochés d'un geste s'il en manque un.
      </p>
      <ul>
        {#each atHome as line (line.ingredient)}
          <li><CartLine {session} {line} ontoggle={() => toggle(line.ingredient)} /></li>
        {/each}
      </ul>
    </details>
  {/if}

  {#if cart.bought.length > 0}
    <button type="button" class="finish" class:confirming onclick={finish}>
      {confirming ? 'Confirmer : vider la liste ?' : 'Terminer les courses'}
    </button>
    {#if confirming}
      <button type="button" class="cancel" onclick={() => (confirming = false)}>Annuler</button>
    {/if}
  {/if}
</Screen>

<style>
  .progress {
    height: var(--space-2);
    margin-bottom: var(--space-4);
    border-radius: var(--radius-pill);
    /* The border token, not the sunken surface: a groove one shade off the
       background disappears in dark mode, and this is the one thing on the
       screen a person reads without stopping to look. */
    background: var(--border);
    overflow: hidden;
  }

  .fill {
    width: var(--picked);
    height: 100%;
    border-radius: var(--radius-pill);
    background: var(--accent);
    transition: width var(--duration-base) var(--ease-out);
  }

  .empty {
    margin: var(--space-6) 0;
    color: var(--text-muted);
    text-align: center;
  }

  /* One row of chips, scrolled sideways rather than wrapped: the number of
     shops is small but not bounded, and a second row would push the first
     aisle off a phone screen. */
  .shops {
    display: flex;
    gap: var(--space-2);
    margin-bottom: var(--space-4);
    overflow-x: auto;
    /* The scroll is inside this strip; the page itself never moves sideways. */
    scrollbar-width: none;
  }

  .shops button {
    flex: none;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-pill);
    background: var(--surface-raised);
    color: inherit;
    font-size: var(--text-sm);
    white-space: nowrap;
    cursor: pointer;
  }

  .shops button.on {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: var(--weight-medium);
  }

  section {
    margin-bottom: var(--space-5);
  }

  h2 {
    margin-bottom: var(--space-2);
    color: var(--text-muted);
    font-size: var(--text-xs);
    font-weight: var(--weight-semibold);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  details {
    margin-top: var(--space-4);
    padding: var(--space-2) 0;
    border-top: 1px solid var(--border);
  }

  summary {
    padding: var(--space-2) var(--space-1);
    color: var(--text-muted);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    cursor: pointer;
  }

  .hint {
    margin: 0 var(--space-3) var(--space-2);
    color: var(--text-faint);
    font-size: var(--text-xs);
  }

  .finish {
    width: 100%;
    margin-top: var(--space-6);
    padding: var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font-weight: var(--weight-semibold);
    cursor: pointer;
  }

  .finish.confirming {
    border-color: var(--danger);
    background: var(--danger);
    color: var(--on-danger);
  }

  .cancel {
    width: 100%;
    margin-top: var(--space-2);
    padding: var(--space-2);
    border: 0;
    background: none;
    color: var(--text-muted);
    font-size: var(--text-sm);
    cursor: pointer;
  }
</style>
