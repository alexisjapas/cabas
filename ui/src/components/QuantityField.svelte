<script lang="ts">
  import type { UnitTag } from '../lib/bindings/UnitTag';
  import { UNIT_GROUPS, UNIT_LABEL } from '../lib/labels';

  /**
   * An amount and its unit.
   *
   * The amount stays **text** all the way to the core, which parses it into an
   * exact rational (Rule 4). Nothing here pre-parses it: a `type="number"`
   * input would hand back a float and lose the precision the whole design is
   * built to keep. `inputmode="decimal"` gets the numeric keypad on a phone
   * without any of that, and a comma types through untouched — the core reads
   * "1,5" as happily as "1.5".
   */
  let {
    amount = $bindable(),
    unit = $bindable(),
    label = 'Quantité',
    required = false,
  }: {
    amount: string;
    unit: UnitTag;
    label?: string;
    /** Lets the browser block the submit, rather than the core refusing an
        empty amount after the fact. */
    required?: boolean;
  } = $props();
</script>

<fieldset>
  <legend>{label}</legend>
  <div>
    <input
      bind:value={amount}
      inputmode="decimal"
      placeholder="1,5"
      aria-label="Quantité"
      autocomplete="off"
      {required}
    />
    <select bind:value={unit} aria-label="Unité">
      {#each UNIT_GROUPS as group (group.label)}
        <optgroup label={group.label}>
          {#each group.units as tag (tag)}
            <option value={tag}>{UNIT_LABEL[tag] === '' ? 'unité' : UNIT_LABEL[tag]}</option>
          {/each}
        </optgroup>
      {/each}
    </select>
  </div>
</fieldset>

<style>
  /* A `<fieldset>` is `min-inline-size: min-content` in every browser's own
     stylesheet, which means it refuses to be narrower than its widest option —
     and that is what pushed the unit dropdown past the right edge of a recipe
     line. Nothing else on the page needed telling; this element did. */
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

  /* An amount and a unit, sharing one line and never leaving it. The amount
     takes what is left; the unit takes what it needs and no more. */
  div {
    display: flex;
    align-items: stretch;
    gap: var(--space-2);
    min-width: 0;
  }

  input {
    flex: 1;
    min-width: 0;
    padding: var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
  }

  /* `flex: none` is what made this overflow the line it sits on: a select that
     cannot shrink is as wide as its widest option, and "c. à s. US" inside a
     recipe line on a phone is wider than the room left for it. It shrinks now,
     down to a floor that still shows a unit, and the label ellipsises rather
     than pushing the amount off the screen. */
  select {
    flex: 0 1 auto;
    width: auto;
    min-width: var(--space-7);
    max-width: 45%;
    padding: var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    text-overflow: ellipsis;
  }
</style>
