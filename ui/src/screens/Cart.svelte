<script lang="ts">
  import { flip } from 'svelte/animate';

  import CartLine from '../components/CartLine.svelte';
  import Screen from '../components/Screen.svelte';
  import type { AisleTag } from '../lib/bindings/AisleTag';
  import type { CartLineView } from '../lib/bindings/CartLineView';
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

  type Group = { aisle: AisleTag; lines: CartLineView[] };

  /**
   * Aisle headings. The order is the core's — it sorted the lines into the
   * order you walk the shop in — so this only has to notice where it changes.
   */
  let groups = $derived.by(() => {
    const built: Group[] = [];
    for (const line of cart.to_buy) {
      const last = built[built.length - 1];
      if (last !== undefined && last.aisle === line.aisle) {
        last.lines.push(line);
      } else {
        built.push({ aisle: line.aisle, lines: [line] });
      }
    }
    return built;
  });

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

  {#if cart.total === 0}
    <p class="empty">
      Ajoutez une recette ou un ingrédient à la liste : le panier se remplit tout seul.
    </p>
  {/if}

  {#each groups as group (group.aisle)}
    <section>
      <h2>{AISLE_LABEL[group.aisle]}</h2>
      <ul>
        {#each group.lines as line (line.ingredient)}
          <li animate:flip={{ duration: 180 }}>
            <CartLine {line} ontoggle={() => toggle(line.ingredient)} />
          </li>
        {/each}
      </ul>
    </section>
  {/each}

  {#if cart.bought.length > 0}
    <details>
      <summary>{CHECK_STATE_LABEL.checked} ({cart.bought.length})</summary>
      <ul>
        {#each cart.bought as line (line.ingredient)}
          <li><CartLine {line} ontoggle={() => toggle(line.ingredient)} /></li>
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
        {#each cart.at_home as line (line.ingredient)}
          <li><CartLine {line} ontoggle={() => toggle(line.ingredient)} /></li>
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
