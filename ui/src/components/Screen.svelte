<script lang="ts">
  import type { Snippet } from 'svelte';

  /**
   * The frame every screen sits in: a title that stays put, and a body that
   * scrolls under it. Shared so the header spacing is decided once rather
   * than re-invented per screen (Rule 10).
   */
  let {
    title,
    subtitle,
    actions,
    children,
  }: {
    title: string;
    subtitle?: string;
    actions?: Snippet;
    children: Snippet;
  } = $props();
</script>

<header>
  <div class="titles">
    <h1>{title}</h1>
    {#if subtitle}<p>{subtitle}</p>{/if}
  </div>
  {#if actions}
    <div class="actions">{@render actions()}</div>
  {/if}
</header>

<div class="body">
  {@render children()}
</div>

<style>
  header {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: flex-end;
    gap: var(--space-3);
    padding: var(--space-4);
    padding-top: calc(var(--safe-top) + var(--space-4));
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }

  .titles {
    flex: 1;
    min-width: 0;
  }

  h1 {
    font-size: var(--text-2xl);
  }

  p {
    margin: var(--space-1) 0 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .actions {
    display: flex;
    gap: var(--space-2);
  }

  .body {
    padding: var(--space-4);
    /* Clear of the tab bar, which floats over the scrolling body — or of the
       keyboard, which covers more and takes the bar with it. Without the second
       term the last field of a form is unreachable on iOS: the document simply
       ends behind the keys, and no amount of scrolling brings it out. */
    padding-bottom: calc(
      var(--space-7) + max(var(--tapsize) + var(--safe-bottom), var(--keyboard-inset))
    );
  }
</style>
