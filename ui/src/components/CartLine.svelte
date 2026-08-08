<script lang="ts">
  import type { CartLineView } from '../lib/bindings/CartLineView';
  import { formatAmounts, relativeTime } from '../lib/format';

  /**
   * One line of the cart, in all three sections.
   *
   * The whole row is the target: a checkbox-sized hit area is the wrong shape
   * for a thumb holding a phone and a basket at the same time.
   */
  let { line, ontoggle }: { line: CartLineView; ontoggle: () => void } = $props();

  let settled = $derived(line.state !== 'to_buy');
</script>

<button type="button" class:settled aria-pressed={settled} onclick={ontoggle}>
  <span class="box" aria-hidden="true">
    {#if settled}
      <svg viewBox="0 0 24 24"><path d="m5 12.5 5 5 9-11" /></svg>
    {/if}
  </span>

  <span class="text">
    <span class="name">{line.name}</span>
    {#if line.checked_by !== null && line.checked_at !== null}
      <span class="meta">{line.checked_by} · {relativeTime(line.checked_at)}</span>
    {:else if line.state === 'auto_checked'}
      <span class="meta">Ingrédient de base, supposé présent</span>
    {/if}
  </span>

  {#if line.amounts.length > 0}
    <span class="amount">{formatAmounts(line.amounts)}</span>
  {/if}
</button>

<style>
  button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3);
    min-height: var(--tapsize);
    border: 0;
    border-radius: var(--radius-md);
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
    transition: opacity var(--duration-fast) var(--ease-out);
  }

  button:active {
    background: var(--surface-sunken);
  }

  .settled {
    opacity: 0.55;
  }

  .box {
    flex: none;
    display: grid;
    place-items: center;
    width: 1.5rem;
    height: 1.5rem;
    border: 2px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-raised);
  }

  .settled .box {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--on-accent);
  }

  svg {
    width: 1rem;
    height: 1rem;
    fill: none;
    stroke: currentColor;
    stroke-width: 3;
    stroke-linecap: round;
    stroke-linejoin: round;
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

  .settled .name {
    text-decoration: line-through;
  }

  .meta {
    color: var(--text-muted);
    font-size: var(--text-xs);
  }

  .amount {
    flex: none;
    color: var(--text-muted);
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
  }
</style>
