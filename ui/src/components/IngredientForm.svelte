<script module lang="ts">
  import type { AisleTag } from '../lib/bindings/AisleTag';
  import type { IngredientInput } from '../lib/bindings/IngredientInput';
  import type { IngredientView } from '../lib/bindings/IngredientView';
  import type { KeepingTag } from '../lib/bindings/KeepingTag';
  import type { UnitTag } from '../lib/bindings/UnitTag';

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
    /**
     * The ids of the shops it can be bought at, in the order they are read
     * (DECISIONS 0071). Empty means "nobody has said", which the cart reads
     * as every shop rather than none.
     */
    shops: string[];
    /** Fridge, freezer or a cupboard (DECISIONS 0070). */
    keeping: KeepingTag;
    staple: boolean;
    density: string;
    unitWeight: string;
    /**
     * How much of it one usually buys, as typed (DECISIONS 0066) — an amount
     * **and a unit** (0089). Both fields are empty of an answer only when the
     * amount is blank *and* the dropdown is still on "unité", which is where
     * it starts: naming a unit is saying something, and it used to be thrown
     * away. See `toInput`.
     */
    defaultAmount: string;
    defaultUnit: UnitTag;
    /** The id of a stored photo, or none. Bytes never travel in a draft. */
    photo: string | null;
  };

  /**
   * A new ingredient under an id, named before it exists.
   *
   * The id is minted by the caller rather than left to the core, because the
   * picker that opened this form has to select what it creates the moment it
   * is created, and a command hands back a whole state rather than an id
   * (DECISIONS 0056). `SaveIngredient` cannot tell which side minted it.
   *
   * It is an argument rather than a mint of its own so that a caller holding a
   * closed panel can seed a draft without paying for an id it will discard —
   * `IngredientPicker` has one per recipe line.
   *
   * `name` is what was being searched for when this form was reached, which
   * is the name the thing is about to be given (DECISIONS 0060). Empty is the
   * ordinary case — the "Nouveau" button, which follows no search.
   */
  export function blankDraft(id: string, name = ''): IngredientDraft {
    return {
      id,
      name: name.trim(),
      aliases: '',
      aisle: 'pantry',
      shops: [],
      keeping: 'ambient',
      staple: false,
      density: '',
      unitWeight: '',
      defaultAmount: '',
      defaultUnit: 'piece',
      photo: null,
    };
  }

  export function draftOf(ingredient: IngredientView): IngredientDraft {
    return {
      id: ingredient.id,
      name: ingredient.name,
      aliases: ingredient.aliases.join(', '),
      aisle: ingredient.aisle,
      shops: [...ingredient.shops],
      keeping: ingredient.keeping,
      staple: ingredient.staple,
      density: ingredient.density ?? '',
      unitWeight: ingredient.unit_weight ?? '',
      defaultAmount: ingredient.default_quantity?.amount ?? '',
      defaultUnit: ingredient.default_quantity?.unit ?? 'piece',
      photo: ingredient.photo,
    };
  }

  /** Empty means "not known", which is not the same as zero (Rule 5). */
  function orNull(text: string): string | null {
    const trimmed = text.trim();
    return trimmed === '' ? null : trimmed;
  }

  /**
   * What a usual quantity is worth, when only half of it was filled in
   * (DECISIONS 0089).
   *
   * The amount alone is the ordinary case and reads straight through. The
   * *unit* alone used to be discarded — "farine, kg" saved as nothing at all,
   * which is the whole of the complaint that produced this entry: somebody
   * opened the dropdown, chose a unit, saved, and found "unité" waiting for
   * them the next time. A unit is a deliberate act, so it stands on its own
   * and means one of it.
   *
   * `piece` is the exception and the reason this is not simply "keep whatever
   * is in the two fields": it is where the dropdown starts, so a form nobody
   * touched would otherwise save "1 pièce" — an answer, over an ingredient
   * nobody has said anything about. That is exactly the state `None` exists
   * to hold (0066), and the core substitutes a piece at the moment it is
   * needed anyway.
   */
  function usualQuantity(draft: IngredientDraft): IngredientInput['default_quantity'] {
    const amount = orNull(draft.defaultAmount);
    if (amount !== null) return { amount, unit: draft.defaultUnit };
    return draft.defaultUnit === 'piece' ? null : { amount: '1', unit: draft.defaultUnit };
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
      shops: [...draft.shops],
      keeping: draft.keeping,
      staple: draft.staple,
      density: orNull(draft.density),
      unit_weight: orNull(draft.unitWeight),
      default_quantity: usualQuantity(draft),
      photo: draft.photo,
    };
  }
</script>

<script lang="ts">
  import type { Snippet } from 'svelte';

  import { AISLE_LABEL, AISLES, KEEPING_LABEL, KEEPINGS } from '../lib/labels';
  import type { Session } from '../lib/session.svelte';
  import PhotoField from './PhotoField.svelte';
  import QuantityField from './QuantityField.svelte';
  import ShopPicker from './ShopPicker.svelte';

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
    session,
    draft = $bindable(),
    heading = undefined,
    submitLabel = 'Enregistrer',
    onsave,
    oncancel,
    extra = undefined,
  }: {
    /** Only the photo needs it: storing one is a call on the core. */
    session: Session;
    draft: IngredientDraft;
    /** Titles the panel where it is not obvious what it is — in a picker. */
    heading?: string | undefined;
    submitLabel?: string;
    /**
     * The panel is the caller's to show and the caller's to close, so a
     * refusal needs nothing from here: whoever opened it closes it on success
     * and leaves it standing otherwise. This used to be typed as returning
     * whether it was accepted, which no caller could rely on — nothing here
     * read it.
     */
    onsave: (ingredient: IngredientInput) => void;
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

  <PhotoField {session} bind:photo={draft.photo} />

  <label>
    Rayon
    <select bind:value={draft.aisle} data-field="aisle" {onkeydown}>
      {#each AISLES as aisle (aisle)}
        <option value={aisle}>{AISLE_LABEL[aisle]}</option>
      {/each}
    </select>
  </label>

  <ShopPicker {session} bind:selected={draft.shops} {onkeydown} />

  <label>
    Conservation
    <select bind:value={draft.keeping} data-field="keeping" {onkeydown}>
      {#each KEEPINGS as keeping (keeping)}
        <option value={keeping}>{KEEPING_LABEL[keeping]}</option>
      {/each}
    </select>
    <small>Où ça va une fois rentré.</small>
  </label>

  <label class="check">
    <input type="checkbox" bind:checked={draft.staple} data-field="staple" {onkeydown} />
    <span>
      Ingrédient de base
      <small>Supposé présent à la maison, donc coché d'avance dans le panier.</small>
    </span>
  </label>

  <!-- A shopping amount, not a cooking one: it is what a swipe puts on the
       list, and no recipe reads it (DECISIONS 0066). -->
  <div class="usual">
    <QuantityField
      bind:amount={draft.defaultAmount}
      bind:unit={draft.defaultUnit}
      label="Quantité habituelle"
      field="default-quantity"
      {onkeydown}
    />
    <small>
      Ce qu'on en achète quand on l'ajoute d'un geste — quantité et unité. Une
      unité seule vaut 1. Tout vide : une pièce.
    </small>
  </div>

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
  /* A form is a lilac tile and its fields are cream (DECISIONS 0081). This one
     also renders inside the list's add form and inside the recipe editor, so
     the tile is what tells a reader where it starts and ends. */
  .ingredient-form {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-4);
    border: 2px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
  }

  h2 {
    margin: 0;
    font-size: var(--text-xs);
    font-weight: var(--weight-bold);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.12em;
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
    border: 2px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--bubble);
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

  .usual {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
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
    border-radius: var(--radius-pill);
    background: var(--accent);
    color: var(--on-accent);
    font-weight: var(--weight-bold);
    box-shadow: var(--shadow-sm);
    cursor: pointer;
  }

  /* A disabled primary is a sunken pill in muted ink, not a half-transparent
     orange one: over a lilac tile the alpha turns the label to mud
     (DECISIONS 0081). */
  .submit:disabled {
    background: var(--surface-sunken);
    color: var(--text-muted);
    box-shadow: none;
  }

  .cancel {
    flex: none;
    padding: var(--space-3) var(--space-4);
    border: 2px solid var(--border-strong);
    border-radius: var(--radius-pill);
    background: var(--bubble);
    color: var(--text);
    font-weight: var(--weight-bold);
    cursor: pointer;
  }
</style>

