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
  }: {
    amount: string;
    unit: UnitTag;
    label?: string;
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
  fieldset {
    margin: 0;
    padding: 0;
    border: 0;
  }

  legend {
    padding: 0 0 var(--space-2);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  div {
    display: flex;
    gap: var(--space-2);
  }

  input {
    flex: 1;
    min-width: 0;
    padding: var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
  }

  select {
    flex: none;
    padding: var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
  }
</style>
