<script lang="ts">
  import type { Snippet } from 'svelte';

  import type { IngredientInput } from '../lib/bindings/IngredientInput';
  import { mintIngredientId } from '../lib/core';
  import { byName } from '../lib/format';
  import { AISLE_LABEL } from '../lib/labels';
  import type { Session } from '../lib/session.svelte';
  import IngredientForm, { blankDraft, type IngredientDraft } from './IngredientForm.svelte';
  import SearchPicker, { type PickerOption } from './SearchPicker.svelte';

  /**
   * Choosing an ingredient, and creating the one that is wanted.
   *
   * Wanting something the library has never heard of is the ordinary case —
   * in a shop and in a recipe alike — and sending someone to another tab to
   * say so loses what they were typing (DECISIONS 0056). So the field, the
   * door at the end of its list and the panel behind that door are one
   * mechanism, and therefore one component: the list and the recipe editor
   * each held a copy of it, and every defect in one was a defect in both.
   *
   * What is left here is the ingredient half. The choosing itself — the
   * search, the panel, the keyboard — is `SearchPicker`, which the recipes
   * are chosen through too (DECISIONS 0058).
   *
   * The form's state belongs to this component and dies with it. That is what
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
    /** The chosen ingredient's id, `''` for none. */
    value: string;
    /** Shown above the field. Without one the field only names itself. */
    label?: string | undefined;
    required?: boolean;
    /** What sits beside the field — a recipe line's remove button. */
    trailing?: Snippet | undefined;
  } = $props();

  /**
   * Sorted for a reader; the core sorts by id, which is stable, not legible.
   *
   * The aliases go along as search terms rather than as anything on screen:
   * somebody looking for "farine de blé" should find "Farine T55", which is
   * exactly what an alias is for.
   */
  let options = $derived<PickerOption[]>(
    [...session.state.ingredients].sort(byName).map((ingredient) => ({
      id: ingredient.id,
      name: ingredient.name,
      hint: AISLE_LABEL[ingredient.aisle],
      terms: ingredient.aliases,
    })),
  );

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

  function open(): void {
    draft = blankDraft(mintIngredientId());
    creating = true;
  }

  /** Created, then chosen: whatever was being written carries on where it was. */
  function create(ingredient: IngredientInput): void {
    // The id the command carries, not a re-read of the form behind it: this is
    // the value the document received. `null` would mean an ingredient the
    // core names itself, which is precisely the one this picker could not then
    // select — so there is nothing to select and nothing to save.
    const created = ingredient.id;
    if (created === null) return;
    if (session.run({ command: 'save_ingredient', ingredient })) {
      value = created;
      creating = false;
    }
  }
</script>

<div class="ingredient-picker">
  <SearchPicker
    {options}
    bind:value
    {label}
    {required}
    {trailing}
    name="Ingrédient"
    placeholder="Chercher un ingrédient…"
    empty="Aucun ingrédient ne correspond."
    doorLabel="+ Nouvel ingrédient"
    ondoor={open}
  />

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
    min-width: 0;
  }
</style>
