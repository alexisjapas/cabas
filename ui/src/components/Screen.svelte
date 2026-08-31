<script lang="ts">
  import type { Snippet } from 'svelte';

  /**
   * The frame every screen sits in: a title that stays put, and a body that
   * scrolls under it. Shared so the header spacing is decided once rather
   * than re-invented per screen (Rule 10).
   *
   * The title is an **anis band** (DECISIONS 0081), not a bare heading: the
   * body scrolls a checked tablecloth underneath, and a title set straight on
   * the checks cannot be read. Two things follow and neither is decoration:
   *
   * - the `header` stays sticky and **opaque** — the least alpha and the
   *   title becomes unreadable while a list scrolls past behind it;
   * - the band has horizontal padding, which is what catches the last letter
   *   of a display face leaning out of its own layout box.
   */
  let {
    title,
    subtitle,
    onback,
    actions,
    children,
  }: {
    title: string;
    subtitle?: string;
    /**
     * Given by a screen that sits over another one. Rendered here rather than
     * passed in through `actions`, because Svelte scopes a snippet's styles
     * to the component that wrote it — a shared look for a button the caller
     * supplies is a rule that cannot match it.
     */
    onback?: () => void;
    actions?: Snippet;
    children: Snippet;
  } = $props();
</script>

<header>
  <div class="band">
    <div class="titles">
      <h1 class="display">{title}</h1>
      {#if subtitle}<p>{subtitle}</p>{/if}
    </div>
    {#if onback || actions}
      <div class="actions">
        {#if onback}<button type="button" class="back" onclick={onback}>Retour</button>{/if}
        {#if actions}{@render actions()}{/if}
      </div>
    {/if}
  </div>
</header>

<div class="body">
  {@render children()}
</div>

<style>
  header {
    position: sticky;
    top: 0;
    z-index: var(--layer-header);
    padding: var(--space-4);
    padding-top: calc(var(--safe-top) + var(--space-4));
    /* Opaque, and the flat shell colour rather than the cloth: this is the one
       strip the checks must not run under. */
    background: var(--surface);
  }

  /* The band itself. No rule under the header — this is the separation. */
  .band {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-4);
    border-radius: var(--radius-md);
    background: var(--accent-soft);
    box-shadow: var(--shadow-md);
  }

  .titles {
    flex: 1;
    min-width: 0;
  }

  h1 {
    font-size: var(--text-2xl);
    /* Not `--check-a`: the cloth's olive falls to 2.14:1 on the anis. */
    color: var(--display-ink);
  }

  p {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--text-xs);
    font-weight: var(--weight-bold);
  }

  .actions {
    flex: none;
    display: flex;
    gap: var(--space-2);
  }

  .back {
    padding: var(--space-2) var(--space-3);
    border: 2px solid var(--border-strong);
    border-radius: var(--radius-pill);
    background: var(--surface-raised);
    color: var(--text);
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
    white-space: nowrap;
    cursor: pointer;
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
