<script lang="ts">
  import type { CartLineView } from '../lib/bindings/CartLineView';
  import { formatAmounts, relativeTime } from '../lib/format';
  import { KEEPING_LABEL } from '../lib/labels';
  import type { Session } from '../lib/session.svelte';
  import Photo from './Photo.svelte';

  /**
   * One line of the cart, in all three sections.
   *
   * The whole row is the target: a checkbox-sized hit area is the wrong shape
   * for a thumb holding a phone and a basket at the same time.
   *
   * It is a **cream card** rather than a row in a ruled table (DECISIONS
   * 0081) — the screen under it is a tablecloth, and nothing is written
   * straight onto the checks. Two consequences worth keeping:
   *
   * - a settled line **changes surface, it does not fade**. `opacity` over a
   *   checked ground gives grey text on a chequerboard; the sunken cream says
   *   the same thing and stays readable.
   * - the keeping badge sits **under the name, never beside it**. On a 302 px
   *   line a box, a thumbnail, a badge and an amount leave the name about
   *   43 px, and the name is the only thing anybody reads at arm's length in
   *   an aisle.
   */
  let {
    session,
    line,
    ontoggle,
  }: { session: Session; line: CartLineView; ontoggle: () => void } = $props();

  let settled = $derived(line.state !== 'to_buy');
</script>

<button type="button" class:settled aria-pressed={settled} onclick={ontoggle}>
  <span class="box" aria-hidden="true">
    {#if settled}
      <svg viewBox="0 0 24 24"><path d="m5 12.5 5 5 9-11" /></svg>
    {/if}
  </span>

  <Photo {session} photo={line.photo} alt="" />

  <span class="text">
    <span class="name">{line.name}</span>

    <!-- Only the two that are not obvious: almost everything lives in a
         cupboard, and a badge on almost every row says nothing (DECISIONS
         0070). It is on the cart line because this is the row still on screen
         when the bags are being emptied onto a counter. -->
    {#if line.keeping !== 'ambient'}
      <span class="keeping" class:freezer={line.keeping === 'freezer'}>
        {KEEPING_LABEL[line.keeping]}
      </span>
    {/if}

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
    background: var(--bubble);
    color: inherit;
    text-align: left;
    cursor: pointer;
    transition: background-color var(--duration-fast) var(--ease-out);
  }

  button:active {
    background: var(--surface-sunken);
  }

  /* Settled lines change surface rather than fading out. */
  .settled {
    background: var(--surface-sunken);
  }

  /* Round, and big enough to aim at: 1.625rem of ring rather than a 1.5rem
     square, because this is the one control pressed while walking. */
  .box {
    flex: none;
    display: grid;
    place-items: center;
    width: 1.625rem;
    height: 1.625rem;
    border: 3px solid var(--border-strong);
    border-radius: var(--radius-pill);
    background: var(--bubble);
  }

  .settled .box {
    border-color: var(--done);
    background: var(--done);
    color: var(--on-done);
  }

  svg {
    width: 0.875rem;
    height: 0.875rem;
    fill: none;
    stroke: currentColor;
    stroke-width: 3;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  /* The name, and whatever has to go under it. `align-items: flex-start` is
     what keeps a badge the width of its own word rather than the width of the
     column. */
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 3px;
  }

  .name {
    max-width: 100%;
    font-weight: var(--weight-medium);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .settled .name {
    color: var(--text-muted);
    text-decoration: line-through;
  }

  .meta {
    color: var(--text-muted);
    font-size: var(--text-xs);
  }

  /* A quantity is a pink pill, everywhere in the app. */
  .amount {
    flex: none;
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--ring);
    color: var(--on-ring);
    font-family: var(--font-numeric);
    font-size: var(--text-xs);
    font-weight: var(--weight-bold);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  /* Settled: still legible, no longer shouting. */
  .settled .amount {
    background: none;
    color: var(--text-muted);
    font-weight: var(--weight-normal);
  }

  .keeping {
    flex: none;
    max-width: 100%;
    padding: 2px 8px;
    border: 2px solid var(--border-strong);
    border-radius: var(--radius-pill);
    background: var(--bubble);
    color: var(--text);
    /* Below `--text-xs` on purpose: it is a second line under a name, and it
       must not compete with it. Not a field, so the 16px floor does not
       apply. */
    font-size: 11px;
    font-weight: var(--weight-bold);
    white-space: nowrap;
  }

  /* The freezer is the one that costs something to get wrong. */
  .keeping.freezer {
    border-color: var(--done);
    color: var(--done);
  }
</style>
