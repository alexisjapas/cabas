<script module lang="ts">
  /**
   * What the picker's last option is worth. Never an ingredient id — those are
   * `ing_…` — so a select can carry it without ambiguity.
   */
  const NEW_INGREDIENT = '+new';
</script>

<script lang="ts">
  import type { Snippet } from 'svelte';

  import type { IngredientInput } from '../lib/bindings/IngredientInput';
  import { mintIngredientId } from '../lib/core';
  import { byName } from '../lib/format';
  import type { Session } from '../lib/session.svelte';
  import IngredientForm, { blankDraft, type IngredientDraft } from './IngredientForm.svelte';

  /**
   * Choosing an ingredient, and creating the one that is wanted.
   *
   * Wanting something the library has never heard of is the ordinary case —
   * in a shop and in a recipe alike — and sending someone to another tab to
   * say so loses what they were typing (DECISIONS 0056). So the select, the
   * door at the end of it and the panel behind that door are one mechanism,
   * and therefore one component: the list and the recipe editor each held a
   * copy of it, and every defect in one was a defect in both.
   *
   * The panel's state belongs to this component and dies with it. That is what
   * makes a removed recipe line take its own half-typed ingredient with it and
   * leave every other line's alone — the editor keys its lines by `component.id`
   * — and what makes closing the list's add form forget the draft rather than
   * resurrect it.
   */
  let {
    session,
    value = $bindable(),
    label = undefined,
    required = false,
    trailing = undefined,
  }: {
    session: Session;
    /** The chosen ingredient's id, `''` for none. Never `NEW_INGREDIENT`. */
    value: string;
    /** Shown above the select. Without one the select only names itself. */
    label?: string | undefined;
    required?: boolean;
    /** What sits beside the select — a recipe line's remove button. */
    trailing?: Snippet | undefined;
  } = $props();

  /** Sorted for a reader; the core sorts by id, which is stable, not legible. */
  let ingredients = $derived([...session.state.ingredients].sort(byName));

  /**
   * The panel, and the draft it fills in.
   *
   * Seeded without an id and re-seeded whole — id and all — every time the
   * door is opened, so a picker nobody opens mints nothing: a recipe holds one
   * of these per ingredient line. The id-less draft is never rendered, because
   * the only thing that raises `creating` is the same statement that mints one.
   */
  let creating = $state(false);
  let draft = $state<IngredientDraft>(blankDraft(''));

  /**
   * The picker is not bound: its last option is a door rather than a value, so
   * choosing it must leave `value` alone — and put the control back where it
   * was, which a binding that never changed would not do.
   *
   * Choosing a real ingredient closes the panel. Left open it would sit under
   * a picker that no longer refers to it, and « Créer » would then overwrite
   * the choice that was just made.
   */
  function pick(event: Event & { currentTarget: HTMLSelectElement }): void {
    const picked = event.currentTarget.value;
    if (picked !== NEW_INGREDIENT) {
      value = picked;
      creating = false;
      return;
    }
    event.currentTarget.value = value;
    draft = blankDraft(mintIngredientId());
    creating = true;
  }

  /** Created, then chosen: whatever was being written carries on where it was. */
  function create(ingredient: IngredientInput): boolean {
    const accepted = session.run({ command: 'save_ingredient', ingredient });
    if (accepted) {
      // The draft is this component's own and nothing re-seeds it between the
      // panel's save and here, so its id is the one the command was given.
      value = draft.id;
      creating = false;
    }
    return accepted;
  }
</script>

<!-- Not `.picker`: the recipe editor's "@" mention list already owns that name
     in the same document, and `ui-test` counts it globally. -->
<div class="ingredient-picker">
  <div class="row">
    <label>
      {#if label !== undefined}{label}{/if}
      <select {value} onchange={pick} {required} aria-label={label ?? 'Ingrédient'}>
        <option value="" disabled>Choisir…</option>
        {#each ingredients as ingredient (ingredient.id)}
          <option value={ingredient.id}>{ingredient.name}</option>
        {/each}
        <option value={NEW_INGREDIENT}>+ Nouvel ingrédient</option>
      </select>
    </label>
    {@render trailing?.()}
  </div>

  {#if creating}
    <IngredientForm
      bind:draft
      heading="Nouvel ingrédient"
      submitLabel="Créer"
      onsave={create}
      oncancel={() => (creating = false)}
    />
  {/if}
</div>

<style>
  .ingredient-picker {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  label {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  select {
    padding: var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-weight: var(--weight-normal);
  }
</style>
