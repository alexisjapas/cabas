<script module lang="ts">
  import type { AisleTag } from '../lib/bindings/AisleTag';
  import type { IngredientInput } from '../lib/bindings/IngredientInput';
  import type { IngredientView } from '../lib/bindings/IngredientView';
  import { mintIngredientId } from '../lib/core';

  /**
   * The library form, wherever it is needed — the Ingredients tab, the list,
   * and a recipe being written (DECISIONS 0056).
   *
   * The whole form travels, not a reduced version of it: an ingredient created
   * mid-shop is the same object as one created in the library, and a second,
   * shorter form would be a second place to add a field to.
   */

  /**
   * The form's own shape. Not `IngredientInput`: the aliases are one line of
   * comma-separated text while they are being typed, and the coefficients are
   * the characters in the field — empty meaning "not known", which is not the
   * same as zero (Rule 5).
   */
  export type IngredientDraft = {
    /** Present from the start, minted or carried — see `blankDraft`. */
    id: string;
    name: string;
    aliases: string;
    aisle: AisleTag;
    staple: boolean;
    density: string;
    unitWeight: string;
  };

  /**
   * What a picker's extra option is worth. Never an ingredient id — those are
   * `ing_…` — so a select can carry it without ambiguity.
   */
  export const NEW_INGREDIENT = '+new';

  /**
   * A new ingredient, named before it exists.
   *
   * The id is minted here rather than left to the core, because the picker
   * that opened this form has to select what it creates the moment it is
   * created, and a command hands back a whole state rather than an id
   * (DECISIONS 0056). `SaveIngredient` cannot tell which side minted it.
   */
  export function blankDraft(): IngredientDraft {
    return {
      id: mintIngredientId(),
      name: '',
      aliases: '',
      aisle: 'grocery',
      staple: false,
      density: '',
      unitWeight: '',
    };
  }

  export function draftOf(ingredient: IngredientView): IngredientDraft {
    return {
      id: ingredient.id,
      name: ingredient.name,
      aliases: ingredient.aliases.join(', '),
      aisle: ingredient.aisle,
      staple: ingredient.staple,
      density: ingredient.density ?? '',
      unitWeight: ingredient.unit_weight ?? '',
    };
  }

  /** Empty means "not known", which is not the same as zero (Rule 5). */
  function orNull(text: string): string | null {
    const trimmed = text.trim();
    return trimmed === '' ? null : trimmed;
  }

  export function toInput(draft: IngredientDraft): IngredientInput {
    return {
      id: draft.id,
      name: draft.name.trim(),
      aliases: draft.aliases
        .split(',')
        .map((alias) => alias.trim())
        .filter((alias) => alias !== ''),
      aisle: draft.aisle,
      staple: draft.staple,
      density: orNull(draft.density),
      unit_weight: orNull(draft.unitWeight),
    };
  }
</script>

<script lang="ts">
  import type { Snippet } from 'svelte';

  import { AISLE_LABEL, AISLES } from '../lib/labels';

  /**
   * # Why this is not a `<form>`
   *
   * It renders inside one. The list's add panel is a form and the recipe
   * editor is one big form, and a nested `<form>` is not parsed — the browser
   * drops it. So the panel is a plain block: every button says
   * `type="button"`, no field is `required`, and Enter is caught here rather
   * than left to submit whatever form happens to be around it.
   *
   * The draft is `$bindable` and belongs to the caller, like `RecipeEditor`'s:
   * a form seeded from a prop captures the value once and ignores the next
   * one, which is the wrong behaviour for a panel that is opened again for a
   * different ingredient.
   */
  let {
    draft = $bindable(),
    heading = undefined,
    submitLabel = 'Enregistrer',
    onsave,
    oncancel,
    extra = undefined,
  }: {
    draft: IngredientDraft;
    /** Titles the panel where it is not obvious what it is — in a picker. */
    heading?: string | undefined;
    submitLabel?: string;
    /** Returns whether it was accepted; a refusal keeps the panel open. */
    onsave: (ingredient: IngredientInput) => boolean;
    oncancel: () => void;
    /** The library's delete button. Nothing else has one. */
    extra?: Snippet | undefined;
  } = $props();

  let complete = $derived(draft.name.trim() !== '');

  function save(): void {
    if (!complete) return;
    onsave(toInput(draft));
  }

  /**
   * Enter saves this panel, and stops there.
   *
   * Left alone it would submit the form this panel is sitting in — adding the
   * half-typed line to the list, or saving the recipe being written. It is on
   * each control rather than on the panel, because a keyboard handler on a
   * `<div>` is an accessibility warning, and warnings are errors here.
   */
  function onkeydown(event: KeyboardEvent): void {
    if (event.key !== 'Enter') return;
    event.preventDefault();
    save();
  }
</script>

<div class="ingredient-form" role="group" aria-label={heading ?? 'Ingrédient'}>
  {#if heading !== undefined}<h2>{heading}</h2>{/if}

  <label>
    Nom
    <input
      bind:value={draft.name}
      data-field="name"
      placeholder="Farine T55"
      autocomplete="off"
      {onkeydown}
    />
  </label>

  <label>
    Autres noms
    <input
      bind:value={draft.aliases}
      data-field="aliases"
      placeholder="farine, farine de blé"
      autocomplete="off"
      {onkeydown}
    />
    <small>Séparés par des virgules.</small>
  </label>

  <label>
    Rayon
    <select bind:value={draft.aisle} data-field="aisle" {onkeydown}>
      {#each AISLES as aisle (aisle)}
        <option value={aisle}>{AISLE_LABEL[aisle]}</option>
      {/each}
    </select>
  </label>

  <label class="check">
    <input type="checkbox" bind:checked={draft.staple} data-field="staple" />
    <span>
      Ingrédient de base
      <small>Supposé présent à la maison, donc coché d'avance dans le panier.</small>
    </span>
  </label>

  <div class="pair">
    <label>
      Densité
      <input
        bind:value={draft.density}
        data-field="density"
        inputmode="decimal"
        placeholder="0,6"
        autocomplete="off"
        {onkeydown}
      />
      <small>g/ml — permet de convertir volume et masse.</small>
    </label>
    <label>
      Poids unitaire
      <input
        bind:value={draft.unitWeight}
        data-field="unit-weight"
        inputmode="decimal"
        placeholder="120"
        autocomplete="off"
        {onkeydown}
      />
      <small>g/pièce — permet de convertir pièces et masse.</small>
    </label>
  </div>

  <div class="buttons">
    <button type="button" class="submit" disabled={!complete} onclick={save}>{submitLabel}</button>
    <button type="button" class="cancel" onclick={oncancel}>Annuler</button>
  </div>

  {@render extra?.()}
</div>

<style>
  .ingredient-form {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
  }

  h2 {
    margin: 0;
    font-size: var(--text-sm);
    font-weight: var(--weight-semibold);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  input,
  select {
    padding: var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-weight: var(--weight-normal);
  }

  small {
    color: var(--text-muted);
    font-weight: var(--weight-normal);
  }

  .check {
    flex-direction: row;
    align-items: flex-start;
    gap: var(--space-3);
  }

  .check input {
    flex: none;
    width: var(--space-5);
    height: var(--space-5);
    margin: 0;
    accent-color: var(--accent);
  }

  .check span {
    display: flex;
    flex-direction: column;
  }

  .pair {
    display: flex;
    gap: var(--space-3);
  }

  .pair label {
    flex: 1;
    min-width: 0;
  }

  .pair input {
    min-width: 0;
  }

  .buttons {
    display: flex;
    gap: var(--space-2);
  }

  .submit {
    flex: 1;
    padding: var(--space-3);
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--on-accent);
    font-weight: var(--weight-semibold);
    cursor: pointer;
  }

  .submit:disabled {
    opacity: 0.5;
  }

  .cancel {
    flex: none;
    padding: var(--space-3) var(--space-4);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    cursor: pointer;
  }
</style>
