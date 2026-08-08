<script lang="ts">
  import Screen from '../components/Screen.svelte';
  import type { FocusView } from '../lib/bindings/FocusView';
  import { decimal, formatQuantity } from '../lib/format';
  import type { Session } from '../lib/session.svelte';

  /**
   * A recipe, read at the number of people it is being cooked for.
   *
   * Nothing here scales anything. `focus.recipe` arrives already rendered at
   * `servings` — every quantity in the ingredient list and inside every step
   * — because the arithmetic is exact rationals and exact rationals stay in
   * Rust (Rule 4). Changing the stepper re-issues `OpenRecipe` and the whole
   * view comes back scaled.
   */
  let {
    session,
    focus,
    onedit,
    ondelete,
    onclose,
  }: {
    session: Session;
    focus: FocusView;
    onedit: () => void;
    ondelete: () => void;
    onclose: () => void;
  } = $props();

  let recipe = $derived(focus.recipe);

  let confirmingDelete = $state(false);

  /**
   * The confirmation is tied to what was added, not to a flag: rescaling and
   * adding again is a normal thing to do, and a button still reading "ajoutée"
   * at a different serving count would be claiming something untrue.
   */
  let addedAt = $state<string | null>(null);
  let added = $derived(addedAt === `${recipe.id}:${recipe.servings}`);

  function read(servings: number): void {
    if (servings < 1) return;
    session.run({ command: 'open_recipe', recipe: recipe.id, servings });
  }

  function addToList(): void {
    // At the servings being read, not the servings it was written for: the
    // stepper above is the "we are six tonight" the list entry records.
    if (session.run({ command: 'add_recipe_to_list', recipe: recipe.id, servings: recipe.servings })) {
      addedAt = `${recipe.id}:${recipe.servings}`;
    }
  }

  function remove(): void {
    if (!confirmingDelete) {
      confirmingDelete = true;
      return;
    }
    ondelete();
  }
</script>

<Screen title={recipe.name} subtitle={recipe.yields !== null ? `Donne ${formatQuantity(recipe.yields)}` : undefined}>
  {#snippet actions()}
    <button type="button" class="close" onclick={onclose}>Fermer</button>
  {/snippet}

  <div class="servings">
    <button type="button" aria-label="Moins" onclick={() => read(recipe.servings - 1)}>−</button>
    <span class="count">{recipe.servings} pers.</span>
    <button type="button" aria-label="Plus" onclick={() => read(recipe.servings + 1)}>+</button>
    {#if recipe.servings !== recipe.written_for}
      <small>écrite pour {recipe.written_for}</small>
    {/if}
  </div>

  <button type="button" class="primary" onclick={addToList}>
    {added ? 'Ajoutée à la liste' : 'Ajouter à la liste'}
  </button>

  <h2>Ingrédients</h2>
  {#if recipe.components.length === 0}
    <p class="empty">Aucun ingrédient.</p>
  {/if}
  <ul class="components">
    {#each recipe.components as component (component.usage)}
      <li>
        {#if component.name === null}
          <span class="gone">
            {component.kind === 'ingredient' ? 'Ingrédient supprimé' : 'Sous-recette supprimée'}
          </span>
        {:else}
          <span class="what">{component.name}</span>
        {/if}
        <span class="amount">
          {#if component.kind === 'ingredient'}
            {formatQuantity(component.quantity)}
          {:else if component.amount.kind === 'factor'}
            ×{decimal(component.amount.factor)}
          {:else}
            {formatQuantity(component.amount.quantity)}
          {/if}
        </span>
      </li>
    {/each}
  </ul>

  {#if recipe.steps.length > 0}
    <h2>Préparation</h2>
    <ol class="steps">
      {#each recipe.steps as step, index (index)}
        <li><p class="prose">{#each step.segments as segment, position (position)}{#if segment.kind === 'text'}{segment.text}{:else if segment.kind === 'missing'}<span class="gone">ligne supprimée</span>{:else}<span class="ref">{#if segment.name !== null}{segment.name}{/if}{#if segment.name !== null && segment.quantity !== null}&nbsp;{/if}{#if segment.quantity !== null}<span class="refamount">{formatQuantity(segment.quantity)}</span>{/if}</span>{/if}{/each}</p></li>
      {/each}
    </ol>
  {/if}

  <div class="buttons">
    <button type="button" class="edit" onclick={onedit}>Modifier</button>
    <button type="button" class="delete" class:confirming={confirmingDelete} onclick={remove}>
      {confirmingDelete ? 'Confirmer la suppression' : 'Supprimer'}
    </button>
  </div>
</Screen>

<style>
  .close {
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-pill);
    background: var(--surface-raised);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    cursor: pointer;
  }

  .servings {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-4);
  }

  .servings button {
    width: var(--tapsize);
    height: var(--space-6);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-raised);
    cursor: pointer;
  }

  .count {
    font-variant-numeric: tabular-nums;
    font-weight: var(--weight-medium);
  }

  .servings small {
    color: var(--text-faint);
    font-size: var(--text-xs);
  }

  .primary {
    width: 100%;
    padding: var(--space-3);
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--on-accent);
    font-size: var(--text-base);
    font-weight: var(--weight-semibold);
    cursor: pointer;
  }

  h2 {
    margin: var(--space-5) 0 var(--space-2);
    font-size: var(--text-sm);
    font-weight: var(--weight-semibold);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .empty {
    margin: 0;
    color: var(--text-muted);
  }

  .components {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .components li {
    display: flex;
    align-items: baseline;
    gap: var(--space-3);
    padding: var(--space-2) 0;
  }

  .components li + li {
    border-top: 1px solid var(--border);
  }

  .what {
    flex: 1;
    min-width: 0;
  }

  .amount {
    flex: none;
    color: var(--text-muted);
    font-family: var(--font-numeric);
    font-size: var(--text-sm);
  }

  .steps {
    margin: 0;
    padding-left: var(--space-5);
  }

  .steps li {
    margin-bottom: var(--space-3);
  }

  .steps li::marker {
    color: var(--text-faint);
    font-size: var(--text-sm);
  }

  /* The authored spacing is the cook's: a step is a run of segments and the
     spaces live inside the text ones (DECISIONS 0022). */
  .prose {
    margin: 0;
    line-height: var(--leading-normal);
    white-space: pre-wrap;
  }

  .ref {
    padding: 0 var(--space-1);
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: var(--weight-medium);
    white-space: normal;
  }

  .refamount {
    font-family: var(--font-numeric);
    font-size: var(--text-sm);
  }

  .gone {
    color: var(--danger);
    font-size: var(--text-sm);
  }

  .buttons {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-6);
  }

  .edit {
    flex: 1;
    padding: var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    font-weight: var(--weight-medium);
    cursor: pointer;
  }

  .delete {
    flex: none;
    padding: var(--space-3);
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
</style>
