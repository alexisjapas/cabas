<script lang="ts">
  import { tick } from 'svelte';

  import AmountDialog, {
    quantityDraft,
    type AmountDraft,
  } from '../components/AmountDialog.svelte';
  import IngredientForm, {
    blankDraft,
    draftOf,
    type IngredientDraft,
  } from '../components/IngredientForm.svelte';
  import Photo from '../components/Photo.svelte';
  import Screen from '../components/Screen.svelte';
  import SearchField from '../components/SearchField.svelte';
  import SwipeToAdd from '../components/SwipeToAdd.svelte';
  import type { IngredientInput } from '../lib/bindings/IngredientInput';
  import type { IngredientView } from '../lib/bindings/IngredientView';
  import { mintIngredientId } from '../lib/core';
  import { byName, matches } from '../lib/format';
  import { badgeOf, entriesBySource } from '../lib/list';
  import { AISLE_LABEL, KEEPING_LABEL } from '../lib/labels';
  import type { Session } from '../lib/session.svelte';

  /**
   * The library: the canonical ingredients everything else refers to.
   *
   * The form itself lives in `IngredientForm`, because the same one opens from
   * the list and from a recipe being written (DECISIONS 0056). What is left
   * here is the shelf, and the one thing only this screen offers: deleting.
   *
   * **The editor opens under the row it belongs to** (DECISIONS 0073). One at
   * a time: opening another closes the first, because two forms over one
   * library are two answers to "what am I editing".
   *
   * The two coefficient fields are the interesting part of that form. Without
   * a density, mass and volume of the same ingredient stay on separate cart
   * lines forever — that is the rule, not a bug (Rule 5). Filling one in is
   * how you tell the app that 300 g of flour and 2 tablespoons of it can be
   * added up.
   */
  let { session }: { session: Session } = $props();

  let ingredients = $derived([...session.state.ingredients].sort(byName));

  /**
   * The filter, and what survives it. Aliases count: a library is searched for
   * the word somebody has in mind, which is exactly what an alias records.
   */
  let query = $state('');
  let shown = $derived(
    ingredients.filter((ingredient) =>
      matches([ingredient.name, ...ingredient.aliases].join(' '), query),
    ),
  );

  /** The list entry each row already has, if it has one (DECISIONS 0067). */
  let onList = $derived(entriesBySource(session.state.list));

  /**
   * The draft is always a whole one; `writing` says *where* the form is —
   * under a row, at the top for a new one, or nowhere. The form binds to the
   * draft, and a binding to something that may be `null` is a different type
   * on each side, which is the shape `Recipes.svelte` uses for the same
   * reason.
   *
   * A single id rather than a set: opening one closes the other, which is the
   * whole of DECISIONS 0073.
   */
  let draft = $state<IngredientDraft>(blankDraft(''));
  let writing = $state<{ under: string | null } | null>(null);
  let confirmingDelete = $state(false);

  /**
   * Only an ingredient the library already holds can be deleted — derived
   * rather than decided when the panel opened, because the library is synced.
   * The other device deleting this one mid-edit takes the button away instead
   * of leaving one that reports "not found" when it is pressed.
   */
  let editing = $derived(session.state.ingredients.some((held) => held.id === draft.id));

  /**
   * `wanted` is the shelf's search, when the creation starts from one that
   * found nothing — the same continuity the picker's door has, at the other
   * place an ingredient is born (DECISIONS 0060).
   */
  async function create(wanted = ''): Promise<void> {
    draft = blankDraft(await mintIngredientId(), wanted);
    writing = { under: null };
    confirmingDelete = false;
  }

  /** A tap on a row: open its editor under it, or close the one that is. */
  function toggle(ingredient: IngredientView): void {
    if (writing !== null && writing.under === ingredient.id) {
      writing = null;
      return;
    }
    draft = draftOf(ingredient);
    writing = { under: ingredient.id };
    confirmingDelete = false;
  }

  /**
   * The row that was just saved, so it can say where it went (DECISIONS
   * 0087).
   *
   * The editor opens *under* its row (0073), so saving one halfway down a
   * long shelf collapses a tall panel and leaves the eye on whatever slid up
   * to fill the gap — and a new ingredient lands wherever the alphabet puts
   * it, which is nowhere near the form that made it. Both are answered by the
   * same two acts: put the row on screen, and flash it.
   *
   * Cleared by the animation's own `animationend` rather than by a timer, so
   * the duration lives in `app.css` and is read from nowhere else — a token
   * parsed back out of `getComputedStyle` is the trap `--press-delay` already
   * carries.
   */
  let flashing = $state<string | null>(null);

  async function save(ingredient: IngredientInput): Promise<void> {
    if (!(await session.run({ command: 'save_ingredient', ingredient }))) return;
    writing = null;
    // Always a string in practice — `blankDraft` mints one and `draftOf`
    // carries one — but `IngredientInput` lets the core name its own, and a
    // row nobody can name is a row nobody can scroll to.
    if (ingredient.id !== null) void reveal(ingredient.id);
  }

  /**
   * Scrolls a row into view and marks it for the flash.
   *
   * Two `tick()`s and a search that may be cleared in between: a *new*
   * ingredient is filed by the alphabet and can perfectly well fall outside
   * the query that was on screen when "Nouveau" was pressed, in which case
   * there is no row to scroll to. Dropping the search is the only way to show
   * the thing that was just created, and it is what somebody who has just
   * created it wants to see.
   */
  async function reveal(id: string): Promise<void> {
    await tick();
    if (document.querySelector(`[data-ingredient="${id}"]`) === null) {
      query = '';
      await tick();
    }
    const row = document.querySelector(`[data-ingredient="${id}"]`);
    if (row === null) return;
    row.scrollIntoView({
      block: 'center',
      behavior: matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth',
    });
    flashing = id;
  }

  async function remove(): Promise<void> {
    if (!confirmingDelete) {
      confirmingDelete = true;
      return;
    }
    // Recipes still using it are not rewritten — the dangling reference is
    // reported and rendered as a warning, never a reason to refuse
    // (DECISIONS 0022).
    if (await session.run({ command: 'delete_ingredient', ingredient: draft.id })) writing = null;
  }

  // --- the amount behind a row (DECISIONS 0072) ------------------------------

  /**
   * The row whose amount is being typed, and the amount itself.
   *
   * Two pieces of state rather than one, because the draft is `$bindable` and
   * the dialog mutates it: `pressed` says which row, and it is what decides
   * whether confirming edits the entry or creates one.
   */
  let pressed = $state<IngredientView | null>(null);
  let amount = $state<AmountDraft>(quantityDraft({ amount: '1', unit: 'piece' }));

  function press(ingredient: IngredientView): void {
    const entry = onList.get(ingredient.id);
    // What is already on the list, or what one usually buys — the same amount
    // the swipe would have put there, so the dialog opens on the answer it is
    // offering to change rather than on an empty field.
    amount = quantityDraft(
      entry !== undefined && entry.item.kind === 'ingredient'
        ? entry.item.edit
        : (ingredient.default_quantity ?? { amount: '1', unit: 'piece' }),
    );
    pressed = ingredient;
  }

  async function confirm(chosen: AmountDraft): Promise<void> {
    if (pressed === null) return;
    const quantity = { amount: chosen.amount, unit: chosen.unit };
    const entry = onList.get(pressed.id);
    const accepted = await (entry === undefined
      ? session.run({
          command: 'add_ingredient_to_list',
          ingredient: pressed.id,
          quantity,
        })
      : session.run({ command: 'set_entry_quantity', entry: entry.id, quantity }));
    if (accepted) pressed = null;
  }
</script>

<Screen
  title="Ingrédients"
  subtitle={ingredients.length === 0 ? 'Bibliothèque vide' : `${ingredients.length} référencés`}
>
  {#snippet actions()}
    <button type="button" class="add" onclick={() => create()}>Nouveau</button>
  {/snippet}

  {#if writing !== null && writing.under === null}
    <div class="panel">
      <IngredientForm {session} bind:draft onsave={save} oncancel={() => (writing = null)} />
    </div>
  {/if}

  {#if ingredients.length === 0 && writing === null}
    <p class="empty bubble">Aucun ingrédient. Créez le premier avec « Nouveau ».</p>
  {:else if ingredients.length > 0}
    <SearchField bind:value={query} placeholder="Chercher un ingrédient…" />
    {#if shown.length === 0}
      <div class="nothing">
        <p class="empty bubble">Aucun ingrédient ne correspond.</p>
        <button type="button" class="create" onclick={() => create(query)}>
          + Nouvel ingrédient «&nbsp;{query.trim()}&nbsp;»
        </button>
      </div>
    {/if}
  {/if}

  <ul>
    {#each shown as ingredient (ingredient.id)}
      {@const entry = onList.get(ingredient.id)}
      <li
        data-ingredient={ingredient.id}
        class:open={writing !== null && writing.under === ingredient.id}
        class:flashing={flashing === ingredient.id}
        onanimationend={() => {
          if (flashing === ingredient.id) flashing = null;
        }}
      >
        <SwipeToAdd
          label={ingredient.name}
          entry={entry?.id ?? null}
          badge={badgeOf(entry)}
          onadd={() =>
            session.run({
              command: 'add_ingredient_to_list',
              ingredient: ingredient.id,
              // No amount: the gesture has nowhere to put one, and the core
              // knows what this ingredient is usually bought by (0066).
              quantity: null,
            })}
          onnudge={(steps) =>
            entry !== undefined &&
            session.run({ command: 'nudge_list_entry', entry: entry.id, steps })}
          onundo={(id) => session.run({ command: 'remove_list_entry', entry: id })}
          onpress={() => press(ingredient)}
        >
          <button type="button" onclick={() => toggle(ingredient)}>
            <Photo {session} photo={ingredient.photo} alt="" />
            <span class="text">
              <span class="name">{ingredient.name}</span>
              <span class="meta">
                {AISLE_LABEL[ingredient.aisle]}
                {#if ingredient.keeping !== 'ambient'}· {KEEPING_LABEL[ingredient.keeping]}{/if}
                {#if ingredient.aliases.length > 0}· {ingredient.aliases.join(', ')}{/if}
              </span>
            </span>
            {#if ingredient.staple}<span class="badge">base</span>{/if}
          </button>
        </SwipeToAdd>

        <!-- Under the row it belongs to, and outside the swipe: a form that
             slid sideways with the row would be unusable the moment a finger
             wandered (DECISIONS 0073). -->
        {#if writing !== null && writing.under === ingredient.id}
          <div class="panel under">
            <IngredientForm {session} bind:draft onsave={save} oncancel={() => (writing = null)}>
              {#snippet extra()}
                {#if editing}
                  <button
                    type="button"
                    class="delete"
                    class:confirming={confirmingDelete}
                    onclick={remove}
                  >
                    {confirmingDelete ? 'Confirmer la suppression' : 'Supprimer'}
                  </button>
                {/if}
              {/snippet}
            </IngredientForm>
          </div>
        {/if}
      </li>
    {/each}
  </ul>
</Screen>

{#if pressed !== null}
  {@const row = pressed}
  <AmountDialog
    title={row.name}
    bind:draft={amount}
    onconfirm={confirm}
    oncancel={() => (pressed = null)}
  />
{/if}

<style>
  .add {
    padding: var(--space-2) var(--space-4);
    border: 0;
    border-radius: var(--radius-pill);
    background: var(--accent);
    color: var(--on-accent);
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
    box-shadow: var(--shadow-sm);
    cursor: pointer;
  }

  /* The form's own styles live with it in `IngredientForm.svelte`; what is
     left here is the shelf and the delete button below the panel. */

  .panel {
    margin-bottom: var(--space-5);
  }

  /* Under a row rather than above the shelf: it belongs to the row, and the
     margin is what stops it from reading as part of the next one. */
  .panel.under {
    margin: var(--space-3) 0 0;
  }

  .delete {
    padding: var(--space-2) var(--space-4);
    border: 2px solid var(--danger);
    border-radius: var(--radius-pill);
    background: var(--bubble);
    color: var(--danger);
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
    cursor: pointer;
  }

  .delete.confirming {
    background: var(--danger);
    color: var(--on-danger);
  }

  /* A search that found nothing, and the way out of it. */
  .nothing {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
    margin: var(--space-5) 0;
  }

  .nothing .empty {
    margin: 0;
  }

  .create {
    padding: var(--space-2) var(--space-4);
    border: 2px solid var(--border-strong);
    border-radius: var(--radius-pill);
    background: var(--bubble);
    color: var(--accent-strong);
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
    cursor: pointer;
  }

  .empty {
    margin: var(--space-5) 0;
    color: var(--text-muted);
    text-align: center;
  }

  /* Cards, spaced, with the cloth between them. The cream belongs to the row
     that slides (`SwipeToAdd`'s `.front`), not to the item. */
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  /* The row whose editor is showing reads as one block with it: a sunken
     tray around both, rather than a tint on the row alone. */
  li.open {
    padding: var(--space-2);
    border-radius: var(--radius-lg);
    background: var(--surface-sunken);
  }

  /*
   * "Here I am", once, after a save (DECISIONS 0087).
   *
   * A pink ring rather than a tint: the row's cream belongs to `SwipeToAdd`'s
   * `.front`, which slides, so anything painted on the `<li>` would be a
   * colour behind a row that can move away from it. An outline is drawn
   * outside the box and needs no room reserved for it, which is what keeps
   * the shelf from shifting by two pixels as the flash starts and stops.
   */
  li.flashing {
    border-radius: var(--radius-md);
    animation: flash var(--duration-flash) var(--ease-out);
  }

  @keyframes flash {
    from {
      outline: var(--photo-ring) solid var(--ring);
      outline-offset: var(--space-1);
    }
    to {
      outline: var(--photo-ring) solid transparent;
      outline-offset: var(--space-1);
    }
  }

  li button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3);
    min-height: var(--tapsize);
    border: 0;
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }

  li button:active {
    background: var(--surface-sunken);
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .name {
    font-weight: var(--weight-medium);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    color: var(--text-muted);
    font-size: var(--text-xs);
    font-weight: var(--weight-bold);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Anis is a surface: the word on it takes the dark ink, never the orange. */
  .badge {
    flex: none;
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--accent-soft);
    color: var(--on-accent);
    font-size: var(--text-xs);
    font-weight: var(--weight-bold);
  }
</style>

