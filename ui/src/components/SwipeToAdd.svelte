<script module lang="ts">
  /**
   * The two distances the gesture is made of live in `app.css` like every
   * other measurement (Rule 10), and are read back here because the pointer
   * maths needs numbers. Custom properties are not resolved by
   * `getComputedStyle`, so they are written in `px` and parsed as such —
   * anything else would come back as the literal text.
   *
   * They are declared on `:root`, so they are the same two numbers for every
   * row, and they are read once for the whole app rather than once per
   * instance: a shelf mounts one of these per ingredient, and narrowing a
   * search remounts every row that comes back into it.
   *
   * The fallbacks are what the tokens say today, and are never the only copy.
   */
  let measured: { reach: number; rest: number } | null = null;

  function lengths(): { reach: number; rest: number } {
    if (measured === null) {
      const styles = getComputedStyle(document.documentElement);
      measured = {
        reach: distance(styles.getPropertyValue('--swipe-reach'), 168),
        rest: distance(styles.getPropertyValue('--swipe-rest'), 92),
      };
    }
    return measured;
  }

  function distance(token: string, fallback: number): number {
    const value = Number.parseFloat(token);
    return Number.isFinite(value) && value > 0 ? value : fallback;
  }
</script>

<script lang="ts">
  /**
   * A row that goes on the list by being pushed there (DECISIONS 0067).
   *
   * Wraps one shelf row — an ingredient, a recipe — and gives it a horizontal
   * gesture: drag right to the stop, let go, and the thing is on the list.
   * The row then springs back and parks short of home, leaving the way out
   * uncovered on the left. That parked state is not a memory of the gesture:
   * `entry` is the list entry the caller found in the core's own state, so a
   * row that is already on the list looks the part after a reload, and one
   * the *other* phone took off the list quietly stops offering to undo.
   *
   * # Why the whole width, and why on release
   *
   * Adding to a shopping list is a real edit, made one-handed, in a shop,
   * against a list that is not on screen. A short flick would fire on every
   * scroll that started slightly sideways. So the gesture is long — the row
   * has to reach a hard stop — and it commits when the finger lifts, which
   * leaves the whole drag cancellable by sliding back.
   *
   * # Why the row cannot be scrolled away mid-swipe
   *
   * `touch-action: pan-y` hands vertical scrolling to the browser and keeps
   * horizontal movement here. Without it the browser claims the gesture as a
   * scroll on the first ambiguous frame and the row simply never moves. The
   * axis is decided once, at `SLOP` pixels, and never revisited: a drag that
   * wanders is still the drag it started as.
   *
   * # The tap underneath
   *
   * The row is a button — it opens the ingredient, it opens the recipe. A
   * swipe ends in a click the browser fires anyway, so a gesture that moved
   * anywhere swallows the click that follows it.
   */

  import type { Snippet } from 'svelte';

  let {
    /**
     * The list entry this row already has, or `null`. Read from the core's
     * state by the caller, never remembered here.
     */
    entry = null,
    label,
    onadd,
    onundo,
    children,
  }: {
    entry?: string | null;
    /** Named for a screen reader: "Ajouter Farine T55 à la liste". */
    label: string;
    onadd: () => void;
    onundo: (entry: string) => void;
    children: Snippet;
  } = $props();

  /**
   * Movement below this has not chosen an axis yet. The one distance here
   * that is not a visual value: it is the width of a thumb's wobble, not a
   * measurement of the layout (Rule 10).
   */
  const SLOP = 8;

  const { reach, rest } = lengths();

  type Phase =
    /** Nothing is happening; the row sits at its resting offset. */
    | { at: 'rest' }
    /** A finger is down and the axis is still undecided. */
    | { at: 'weighing'; x: number; y: number }
    /** Committed to the horizontal; the row follows the finger. */
    | { at: 'dragging'; x: number; from: number; offset: number };

  let phase = $state.raw<Phase>({ at: 'rest' });
  /** Set for as long as it takes the click after a real drag to arrive. */
  let moved = $state(false);

  let parked = $derived(entry !== null ? rest : 0);
  let offset = $derived(phase.at === 'dragging' ? phase.offset : parked);
  /** A drag is only animated on the way back — following a finger must not lag. */
  let settling = $derived(phase.at !== 'dragging');
  let armed = $derived(phase.at === 'dragging' && phase.offset >= reach);

  /**
   * Past the stop the row keeps moving, a third as fast. The end of the
   * gesture has to be felt rather than read, and a row that simply freezes
   * feels like a row that has stopped responding.
   */
  function resist(travelled: number): number {
    if (travelled <= reach) return Math.max(0, travelled);
    return reach + (travelled - reach) / 3;
  }

  function down(event: PointerEvent): void {
    // Mouse buttons other than the first, and anything already in flight.
    if (!event.isPrimary || (event.pointerType === 'mouse' && event.button !== 0)) return;
    moved = false;
    phase = { at: 'weighing', x: event.clientX, y: event.clientY };
  }

  function move(event: PointerEvent): void {
    if (phase.at === 'rest') return;

    if (phase.at === 'weighing') {
      const dx = event.clientX - phase.x;
      const dy = event.clientY - phase.y;
      if (Math.abs(dx) < SLOP && Math.abs(dy) < SLOP) return;
      // Vertical wins ties: this list is scrolled far more often than it is
      // swiped, and a scroll that snags is worse than a swipe that misses.
      if (Math.abs(dx) <= Math.abs(dy)) {
        phase = { at: 'rest' };
        return;
      }
      // Captured only now, so a scroll that began here still belongs to the
      // page. From this point every move and the release come to us even if
      // the finger leaves the row.
      try {
        (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
      } catch {
        // A pointer the browser does not consider active — a synthesised
        // event, a mouse released outside the window. Capture is what keeps
        // a drag alive once the finger wanders off the row; without it the
        // ordinary case still works, so this is a loss of polish and not a
        // reason to abandon the gesture.
      }
      moved = true;
      phase = { at: 'dragging', x: event.clientX, from: parked, offset: parked };
      return;
    }

    phase = { ...phase, offset: resist(phase.from + event.clientX - phase.x) };
  }

  function up(): void {
    if (phase.at !== 'dragging') {
      phase = { at: 'rest' };
      return;
    }
    const reached = armed;
    phase = { at: 'rest' };
    // Only ever an add. Undoing is the button, not the gesture: a list entry
    // is worth one deliberate tap to remove, and a leftward flick over a row
    // that is scrolling is not one.
    if (reached && entry === null) onadd();
  }

  /** The click the browser fires at the end of a drag is not a tap. */
  function click(event: MouseEvent): void {
    if (!moved) return;
    event.preventDefault();
    event.stopPropagation();
    moved = false;
  }
</script>

<div class="swipe">
  <div class="behind" class:done={entry !== null}>
    {#if entry !== null}
      <button type="button" class="undo" onclick={() => onundo(entry)}>Annuler</button>
    {:else}
      <span class="hint" class:armed aria-hidden="true">Ajouter à la liste</span>
    {/if}
  </div>

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="front"
    class:settling
    style:transform="translateX({offset}px)"
    role="group"
    aria-label={entry === null ? `Ajouter ${label} à la liste` : `${label}, sur la liste`}
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={up}
    onclickcapture={click}
  >
    {@render children()}
  </div>
</div>

<style>
  .swipe {
    position: relative;
    /* The row slides sideways over this box and must not paint outside it,
       or a half-swiped row overlaps the one below on the way past. */
    overflow: hidden;
  }

  .behind {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    /* What the row uncovers is on the left, because the row leaves to the
       right — the text has to be readable in the strip that opens up, from
       the first few pixels of it. */
    justify-content: flex-start;
    padding-left: var(--space-3);
    background: var(--accent-soft);
  }

  .behind.done {
    background: var(--surface-sunken);
    padding-left: 0;
  }

  .hint {
    color: var(--accent);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    white-space: nowrap;
    opacity: 0.7;
    transition: opacity var(--duration-fast) var(--ease-out);
  }

  /* The one thing that says "let go now". */
  .hint.armed {
    opacity: 1;
  }

  .undo {
    width: var(--swipe-rest);
    align-self: stretch;
    border: 0;
    background: none;
    color: var(--danger);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    cursor: pointer;
  }

  .front {
    position: relative;
    /* Opaque, or the row it is covering reads through it. */
    background: var(--surface);
    /* Vertical scrolling stays the browser's; horizontal is ours. */
    touch-action: pan-y;
  }

  .front.settling {
    transition: transform var(--duration-spring) var(--ease-spring);
  }
</style>
