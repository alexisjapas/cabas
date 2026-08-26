<script module lang="ts">
  import type { QuantityInput } from '../lib/bindings/QuantityInput';
  import type { UnitTag } from '../lib/bindings/UnitTag';

  /**
   * What the dialog is editing.
   *
   * One flat shape holding both halves rather than a union, and the reason is
   * `bind:`: a union would make `bind:amount={draft.amount}` a type error on
   * the branch where the field does not exist, and the workaround is a cast
   * at every binding. `kind` says which half is real; the other half is a
   * value nobody reads.
   */
  export type AmountDraft = {
    kind: 'quantity' | 'servings';
    amount: string;
    unit: UnitTag;
    servings: number;
  };

  /** The amount of a bare ingredient, seeded from what the list already holds. */
  export function quantityDraft(quantity: QuantityInput): AmountDraft {
    return { kind: 'quantity', amount: quantity.amount, unit: quantity.unit, servings: 1 };
  }

  export function servingsDraft(servings: number): AmountDraft {
    return { kind: 'servings', amount: '', unit: 'piece', servings };
  }
</script>

<script lang="ts">
  /**
   * The amount behind a shelf row, opened by holding it (DECISIONS 0072).
   *
   * The swipe is deliberately coarse — it counts notches and cannot express
   * "350 g" or change a unit — so there has to be somewhere the exact answer
   * is typed. Holding the row is where, because it is the same thumb in the
   * same place, and because a row that is already on the list has no other
   * spare gesture: a tap opens the ingredient.
   *
   * The draft belongs to the caller and is `$bindable`, the shape
   * `IngredientForm` and `RecipeEditor` both use: a form seeded from a prop
   * captures the value once and quietly ignores the next one, which is the
   * wrong behaviour for a panel that opens again over a different row.
   *
   * # Why this is a panel and not a `<dialog>`
   *
   * `showModal()` puts the element in the top layer, where it is positioned
   * against the *layout* viewport — the one iOS does not shrink for the
   * keyboard (DECISIONS 0040). The panel would then sit under the keys with
   * no way to read `--keyboard-inset` out of it. This is an ordinary fixed
   * overlay that reads the measured inset like everything else that sits at
   * the bottom of the screen.
   */

  import QuantityField from './QuantityField.svelte';

  let {
    title,
    draft = $bindable(),
    onconfirm,
    oncancel,
  }: {
    /** The row this is about — the name, so the panel says what it edits. */
    title: string;
    draft: AmountDraft;
    onconfirm: (draft: AmountDraft) => void;
    oncancel: () => void;
  } = $props();

  let complete = $derived(
    draft.kind === 'servings' ? draft.servings >= 1 : draft.amount.trim() !== '',
  );

  function confirm(event: SubmitEvent): void {
    event.preventDefault();
    if (!complete) return;
    onconfirm(draft);
  }

  function rescale(by: number): void {
    const next = draft.servings + by;
    if (next < 1) return;
    draft = { ...draft, servings: next };
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="scrim" onclick={oncancel}></div>

<!-- `.amount-dialog` and not `.panel`: `ui-test` queries the DOM globally
     while Svelte scopes styles per component, so two components naming their
     root the same thing silently change what a selector counts. The shelf
     that opens this one already has a `.panel`. -->
<div class="amount-dialog" role="dialog" aria-modal="true" aria-label={title}>
  <form onsubmit={confirm}>
    <h2>{title}</h2>

    {#if draft.kind === 'servings'}
      <fieldset>
        <legend>Pour</legend>
        <div class="stepper">
          <button type="button" aria-label="Moins" onclick={() => rescale(-1)}>−</button>
          <span>{draft.servings} pers.</span>
          <button type="button" aria-label="Plus" onclick={() => rescale(1)}>+</button>
        </div>
      </fieldset>
    {:else}
      <QuantityField bind:amount={draft.amount} bind:unit={draft.unit} field="entry-amount" required />
    {/if}

    <div class="buttons">
      <button type="submit" class="submit" disabled={!complete}>Valider</button>
      <button type="button" class="cancel" onclick={oncancel}>Annuler</button>
    </div>
  </form>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 20;
    background: var(--scrim);
  }

  /* Anchored to the bottom, because that is where the thumb that opened it
     already is — and because the keyboard comes up under it. */
  .amount-dialog {
    position: fixed;
    z-index: 21;
    left: 0;
    right: 0;
    bottom: 0;
    /* `max()` so that `0px` — no keyboard — is the layout that existed before
       any of this did (DECISIONS 0040). */
    padding-bottom: max(var(--space-5), var(--keyboard-inset));
  }

  form {
    max-width: var(--content-width);
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg) var(--radius-lg) 0 0;
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

  fieldset {
    margin: 0;
    padding: 0;
    min-width: 0;
    border: 0;
  }

  legend {
    padding: 0 0 var(--space-2);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  .stepper {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .stepper button {
    width: var(--tapsize);
    height: var(--tapsize);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    font-size: var(--text-lg);
    cursor: pointer;
  }

  .stepper span {
    font-variant-numeric: tabular-nums;
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
