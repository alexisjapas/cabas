<script lang="ts">
  import IngredientForm, {
    blankDraft,
    draftOf,
    type IngredientDraft,
  } from '../components/IngredientForm.svelte';
  import Screen from '../components/Screen.svelte';
  import SearchField from '../components/SearchField.svelte';
  import type { IngredientInput } from '../lib/bindings/IngredientInput';
  import type { IngredientView } from '../lib/bindings/IngredientView';
  import { mintIngredientId } from '../lib/core';
  import { byName, matches } from '../lib/format';
  import { AISLE_LABEL } from '../lib/labels';
  import type { Session } from '../lib/session.svelte';

  /**
   * The library: the canonical ingredients everything else refers to.
   *
   * The form itself lives in `IngredientForm`, because the same one opens from
   * the list and from a recipe being written (DECISIONS 0056). What is left
   * here is the shelf, and the one thing only this screen offers: deleting.
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

  /**
   * The draft is always a whole one and `writing` says whether it is on
   * screen — the shape `Recipes.svelte` uses, and for the same reason: the
   * form binds to it, and a binding to something that may be `null` is a
   * different type on each side. The seed carries no id because it is never
   * rendered; `open` is what mints one.
   */
  let draft = $state<IngredientDraft>(blankDraft(''));
  let writing = $state(false);
  let confirmingDelete = $state(false);

  /**
   * Only an ingredient the library already holds can be deleted — derived
   * rather than decided when the panel opened, because the library is synced.
   * The other device deleting this one mid-edit takes the button away instead
   * of leaving one that reports "not found" when it is pressed.
   */
  let editing = $derived(session.state.ingredients.some((held) => held.id === draft.id));

  function open(ingredient: IngredientView | null): void {
    draft = ingredient === null ? blankDraft(mintIngredientId()) : draftOf(ingredient);
    writing = true;
    confirmingDelete = false;
  }

  function save(ingredient: IngredientInput): void {
    if (session.run({ command: 'save_ingredient', ingredient })) writing = false;
  }

  function remove(): void {
    if (!confirmingDelete) {
      confirmingDelete = true;
      return;
    }
    // Recipes still using it are not rewritten — the dangling reference is
    // reported and rendered as a warning, never a reason to refuse
    // (DECISIONS 0022).
    if (session.run({ command: 'delete_ingredient', ingredient: draft.id })) writing = false;
  }
</script>

<Screen
  title="Ingrédients"
  subtitle={ingredients.length === 0 ? 'Bibliothèque vide' : `${ingredients.length} référencés`}
>
  {#snippet actions()}
    <button type="button" class="add" onclick={() => open(null)}>Nouveau</button>
  {/snippet}

  {#if writing}
    <div class="panel">
      <IngredientForm bind:draft onsave={save} oncancel={() => (writing = false)}>
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

  {#if ingredients.length === 0 && !writing}
    <p class="empty">Aucun ingrédient. Créez le premier avec « Nouveau ».</p>
  {:else if ingredients.length > 0}
    <SearchField bind:value={query} placeholder="Chercher un ingrédient…" />
    {#if shown.length === 0}
      <p class="empty">Aucun ingrédient ne correspond.</p>
    {/if}
  {/if}

  <ul>
    {#each shown as ingredient (ingredient.id)}
      <li>
        <button type="button" onclick={() => open(ingredient)}>
          <span class="text">
            <span class="name">{ingredient.name}</span>
            <span class="meta">
              {AISLE_LABEL[ingredient.aisle]}
              {#if ingredient.aliases.length > 0}· {ingredient.aliases.join(', ')}{/if}
            </span>
          </span>
          {#if ingredient.staple}<span class="badge">base</span>{/if}
        </button>
      </li>
    {/each}
  </ul>
</Screen>

<style>
  .add {
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-pill);
    background: var(--surface-raised);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    cursor: pointer;
  }

  /* The form's own styles live with it in `IngredientForm.svelte`; what is
     left here is the shelf and the delete button below the panel. */

  .panel {
    margin-bottom: var(--space-5);
  }

  .delete {
    padding: var(--space-2);
    border: 1px solid var(--danger);
    border-radius: var(--radius-md);
    background: none;
    color: var(--danger);
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .delete.confirming {
    background: var(--danger);
    color: var(--on-danger);
  }

  .empty {
    margin: var(--space-6) 0;
    color: var(--text-muted);
    text-align: center;
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li + li {
    border-top: 1px solid var(--border);
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
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    color: var(--text-muted);
    font-size: var(--text-xs);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .badge {
    flex: none;
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--accent-soft);
    color: var(--accent);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
  }
</style>
