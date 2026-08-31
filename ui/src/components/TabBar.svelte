<script lang="ts">
  import type { Screen } from '../lib/session.svelte';

  let {
    current,
    onselect,
  }: {
    current: Screen;
    onselect: (screen: Screen) => void;
  } = $props();

  /**
   * Icons are inline paths rather than a font or a sprite: five shapes do not
   * justify a dependency, and an icon that fails to load in a shop is worse
   * than no icon.
   *
   * The order is the order the app is *used* in, left to right (DECISIONS
   * 0064): the shelves an ingredient or a recipe comes off, then the list it
   * is asked for on, then the trip it is bought on. The tabs used to run the
   * other way, which put the end of the story under the left thumb and read
   * backwards to everyone who was not the person who wrote it. Réglages sits
   * at the far end because it is where a flow starts least often.
   */
  const TABS: readonly { id: Screen; label: string; path: string }[] = [
    { id: 'settings', label: 'Réglages', path: 'M4 8h16M4 16h16M9 8a2 2 0 1 0 0 .01M15 16a2 2 0 1 0 0 .01' },
    { id: 'ingredients', label: 'Ingrédients', path: 'M3 11.5 11.5 3H20a1 1 0 0 1 1 1v8.5L12.5 21 3 11.5Zm13-3.5h.01' },
    {
      id: 'recipes',
      label: 'Recettes',
      path: 'M5 4.5A1.5 1.5 0 0 1 6.5 3H19v18H6.5A1.5 1.5 0 0 1 5 19.5v-15ZM5 17.5h14M9 7.5h6',
    },
    { id: 'list', label: 'Liste', path: 'M9 6h11M9 12h11M9 18h11M4.5 6h.01M4.5 12h.01M4.5 18h.01' },
    {
      id: 'cart',
      label: 'Courses',
      path: 'M4 9h16l-1.4 10.2a2 2 0 0 1-2 1.8H7.4a2 2 0 0 1-2-1.8L4 9Zm5 0V6a3 3 0 0 1 6 0v3',
    },
  ];
</script>

<nav aria-label="Sections">
  {#each TABS as tab (tab.id)}
    <button
      type="button"
      class:current={current === tab.id}
      aria-current={current === tab.id ? 'page' : undefined}
      onclick={() => onselect(tab.id)}
    >
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d={tab.path} />
      </svg>
      <span>{tab.label}</span>
    </button>
  {/each}
</nav>

<style>
  nav {
    position: fixed;
    inset: auto 0 0;
    z-index: var(--layer-tabbar);
    display: flex;
    justify-content: center;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-2) calc(var(--safe-bottom) + var(--space-2));
    /* Opaque, the flat shell colour, and no rule on top: the pills are the
       separation (DECISIONS 0081). */
    background: var(--surface);
    /* The bar belongs at the bottom of the layout viewport, and the keyboard
       covers the bottom of the layout viewport — so it goes down with it. That
       holds whichever viewport a browser anchors a fixed element to: where the
       bar is already hidden behind the keys this changes nothing, and where it
       would float above them it stops five buttons from sitting on top of the
       row being typed into. Nobody switches tabs mid-word. */
    transform: translateY(var(--keyboard-inset));
    transition: transform var(--duration-base) var(--ease-out);
  }

  /* A lilac pill at rest, an anis one with an orange rim when it is the screen
     you are on — read at a metre, with a basket on the arm. The border is
     already there when it is transparent, so activation moves nothing. */
  button {
    flex: 1;
    /* Five tabs across the narrowest phone: the labels ellipsize rather than
       wrap the bar onto two lines. */
    min-width: 0;
    max-width: calc(var(--content-width) / 5);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-1);
    min-height: var(--tapsize);
    border: 3px solid transparent;
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font-size: var(--text-tab);
    font-weight: var(--weight-bold);
    cursor: pointer;
    transition:
      background-color var(--duration-fast) var(--ease-out),
      border-color var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out);
  }

  button.current {
    border-color: var(--accent);
    background: var(--accent-soft);
    /* The glyph inherits `color`, so this carries the icon as well as the
       word — which is the second reason the cloth's olive would be wrong
       here and not merely unreadable. */
    color: var(--display-ink);
  }

  svg {
    width: 1.25rem;
    height: 1.25rem;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.75;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  span {
    /* "Ingrédients" must not wrap the bar onto two lines. */
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>

