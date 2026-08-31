<script module lang="ts">
  /**
   * The distances the gesture is made of live in `app.css` like every other
   * measurement (Rule 10), and are read back here because the pointer maths
   * needs numbers. Custom properties are not resolved by `getComputedStyle`,
   * so they are written in `px` and parsed as such — anything else would come
   * back as the literal text.
   *
   * They are declared on `:root`, so they are the same numbers for every row,
   * and they are read once for the whole app rather than once per instance: a
   * shelf mounts one of these per ingredient, and narrowing a search remounts
   * every row that comes back into it.
   *
   * The fallbacks are what the tokens say today, and are never the only copy.
   */
  let measured: { reach: number; rest: number; press: number } | null = null;

  function lengths(): { reach: number; rest: number; press: number } {
    if (measured === null) {
      const styles = getComputedStyle(document.documentElement);
      measured = {
        reach: distance(styles.getPropertyValue('--swipe-reach'), 168),
        rest: distance(styles.getPropertyValue('--swipe-rest'), 104),
        press: milliseconds(styles.getPropertyValue('--press-delay'), 500),
      };
    }
    return measured;
  }

  function distance(token: string, fallback: number): number {
    const value = Number.parseFloat(token);
    return Number.isFinite(value) && value > 0 ? value : fallback;
  }

  /**
   * `--press-delay` is a duration, and its unit is not decoration: the token
   * is authored as `500ms` and the CSS minifier ships it as `.5s`, so a bare
   * `parseFloat` gives 0.5 — a press that fires after half a millisecond,
   * which in a *test* looks exactly like one that never fires at all. That is
   * how this was found.
   */
  function milliseconds(token: string, fallback: number): number {
    const value = Number.parseFloat(token);
    if (!Number.isFinite(value) || value <= 0) return fallback;
    return token.trim().endsWith('ms') ? value : value * 1000;
  }
</script>

<script lang="ts">
  /**
   * A row that goes on the list by being pushed there, and stays under the
   * thumb afterwards (DECISIONS 0067, 0072).
   *
   * Wraps one shelf row — an ingredient, a recipe — and gives it a horizontal
   * gesture. Drag right to the stop and let go: the thing is on the list. The
   * row then springs back and parks short of home, leaving the way out
   * uncovered on the left, **with what it asks for written on it**. From
   * there the same gesture keeps working: right for one more notch, left for
   * one less, and left past the last one takes the row off the list.
   *
   * That parked state is not a memory of the gesture: `entry` is the list
   * entry the caller found in the core's own state, so a row already on the
   * list looks the part after a reload, and one the *other* phone took off
   * the list quietly stops offering to undo.
   *
   * # What a notch is worth is not decided here
   *
   * This component counts notches and nothing else — `onnudge(+1)`,
   * `onnudge(-1)`. A kilo of flour, another whole tart for four: that rule is
   * the core's, because it is business logic and there is one of it (Rule 9).
   * The same goes for `badge`, which arrives already written.
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
   * # The two taps underneath
   *
   * The row is a button — it opens the ingredient, it opens the recipe — and
   * holding it opens the amount instead. A swipe ends in a click the browser
   * fires anyway, so a gesture that moved anywhere swallows the click that
   * follows it; so does a press that has already fired.
   */

  import type { Snippet } from 'svelte';

  let {
    /**
     * The id of the list entry this row already has, or `null`. Read from the
     * core's state by the caller, never remembered here.
     */
    entry = null,
    /**
     * What that entry asks for, already written — "1 kg", "4 pers.". `null`
     * when the row is not on the list, and when the caller has nothing
     * sensible to say.
     */
    badge = null,
    label,
    onadd,
    onnudge,
    onundo,
    onpress,
    children,
  }: {
    entry?: string | null;
    badge?: string | null;
    /** Named for a screen reader: "Ajouter Farine T55 à la liste". */
    label: string;
    onadd: () => void;
    /** One notch, in either direction. Only ever called with ±1. */
    onnudge: (steps: number) => void;
    onundo: (entry: string) => void;
    /** A finger held still on the row: open the amount for editing. */
    onpress: () => void;
    children: Snippet;
  } = $props();

  /**
   * Movement below this has not chosen an axis yet, and is also what a press
   * is allowed to wander without becoming a drag. The one distance here that
   * is not a visual value: it is the width of a thumb's wobble, not a
   * measurement of the layout (Rule 10).
   */
  const SLOP = 8;

  const { reach, rest, press } = lengths();

  type Phase =
    /** Nothing is happening; the row sits at its resting offset. */
    | { at: 'rest' }
    /** A finger is down and the axis is still undecided. */
    | { at: 'weighing'; x: number; y: number }
    /** Committed to the horizontal; the row follows the finger. */
    | { at: 'dragging'; x: number; from: number; travel: number };

  let phase = $state.raw<Phase>({ at: 'rest' });
  /** Set for as long as it takes the click after a real gesture to arrive. */
  let spent = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  let parked = $derived(entry !== null ? rest : 0);
  let travel = $derived(phase.at === 'dragging' ? phase.travel : 0);
  let offset = $derived(parked + travel);
  /** A drag is only animated on the way back — following a finger must not lag. */
  let settling = $derived(phase.at !== 'dragging');
  let armed = $derived(travel >= reach);
  /** Only ever true on a row that is already on the list; see `resist`. */
  let disarming = $derived(travel <= -reach);

  /**
   * Past the stop the row keeps moving, a third as fast. The end of the
   * gesture has to be felt rather than read, and a row that simply freezes
   * feels like a row that has stopped responding.
   *
   * Leftward travel is refused outright on a row that is not on the list:
   * there is nothing there to take one away from, and sliding it left would
   * uncover a strip with nothing in it.
   */
  function resist(raw: number): number {
    if (entry === null && raw < 0) return 0;
    const past = Math.abs(raw) - reach;
    if (past <= 0) return raw;
    return Math.sign(raw) * (reach + past / 3);
  }

  function cancelPress(): void {
    clearTimeout(timer);
    timer = undefined;
  }

  function down(event: PointerEvent): void {
    // Mouse buttons other than the first, and anything already in flight.
    if (!event.isPrimary || (event.pointerType === 'mouse' && event.button !== 0)) return;
    spent = false;
    phase = { at: 'weighing', x: event.clientX, y: event.clientY };
    cancelPress();
    timer = setTimeout(() => {
      timer = undefined;
      // Still weighing: the finger never committed to an axis, so this was a
      // press. The gesture is spent, which is what stops the release and the
      // click that follows it from also doing something.
      if (phase.at !== 'weighing') return;
      phase = { at: 'rest' };
      spent = true;
      onpress();
    }, press);
  }

  function move(event: PointerEvent): void {
    if (phase.at === 'rest') return;

    if (phase.at === 'weighing') {
      const dx = event.clientX - phase.x;
      const dy = event.clientY - phase.y;
      if (Math.abs(dx) < SLOP && Math.abs(dy) < SLOP) return;
      cancelPress();
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
      spent = true;
      phase = { at: 'dragging', x: event.clientX, from: 0, travel: 0 };
      return;
    }

    phase = { ...phase, travel: resist(phase.from + event.clientX - phase.x) };
  }

  function up(): void {
    cancelPress();
    if (phase.at !== 'dragging') {
      phase = { at: 'rest' };
      return;
    }
    const forward = armed;
    const back = disarming;
    phase = { at: 'rest' };
    // Right is always "one more of this", and on a row that is not on the
    // list yet the first one is the add. Left is always "one less", which the
    // core turns into a removal when it was the last one — so this component
    // never has to know which of the two just happened.
    if (forward) {
      if (entry === null) onadd();
      else onnudge(1);
    } else if (back && entry !== null) {
      onnudge(-1);
    }
  }

  /** The click at the end of a gesture is not a tap. */
  function click(event: MouseEvent): void {
    if (!spent) return;
    event.preventDefault();
    event.stopPropagation();
    spent = false;
  }
</script>

<div class="swipe">
  <div class="behind" class:done={entry !== null}>
    {#if entry !== null}
      <button type="button" class="undo" onclick={() => onundo(entry)}>
        <span>Annuler</span>
        {#if badge !== null}<small>{badge}</small>{/if}
      </button>
      <span class="less" class:armed={disarming} aria-hidden="true">− 1</span>
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
    oncontextmenu={(event) => event.preventDefault()}
  >
    {@render children()}
  </div>
</div>

<style>
  .swipe {
    position: relative;
    /* The row slides sideways over this box and must not paint outside it,
       or a half-swiped row overlaps the one below on the way past. The radius
       is here rather than only on the card, so the strip that is uncovered is
       clipped to the same shape. */
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  /* Two strips, one at each end, and **two grounds** (DECISIONS 0081): anis
     while the row is not on the list yet, sunken cream once it is. The colour
     is the answer to "is this already on the list", read before the words
     are. */
  .behind {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    /* The way out and the amount on the left, because the row leaves to the
       right; "one less" on the right, because that is what the row uncovers on
       its way back past home. */
    justify-content: space-between;
    padding-left: var(--space-4);
    background: var(--accent-soft);
  }

  .behind.done {
    background: var(--surface-sunken);
    padding-left: 0;
  }

  /* Orange as an ink, which is the darker one: `--accent` on anis is 2:1. */
  .hint {
    color: var(--accent-strong);
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
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
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-1);
    border: 0;
    background: none;
    color: var(--danger);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    cursor: pointer;
  }

  /* What the list holds, under the word that takes it back off. Read far
     more often than "Annuler" is pressed, which is why it is here at all —
     and under it rather than over it, because the word is the action and the
     number is the information. */
  .undo small {
    color: var(--text-muted);
    font-family: var(--font-numeric);
    font-size: var(--text-xs);
    font-variant-numeric: tabular-nums;
    font-weight: var(--weight-normal);
    white-space: nowrap;
  }

  .less {
    padding-right: var(--space-4);
    color: var(--text-muted);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    opacity: 0.5;
    transition: opacity var(--duration-fast) var(--ease-out);
  }

  .less.armed {
    color: var(--danger);
    opacity: 1;
  }

  .front {
    position: relative;
    /* The cream card that slides. Opaque, or the strip it is covering reads
       through it — and the radius travels with it, so the row still looks
       like a card while it is parked. */
    background: var(--bubble);
    border-radius: var(--radius-md);
    /* Vertical scrolling stays the browser's; horizontal is ours. */
    touch-action: pan-y;
    /* A press is held, and holding a row is how a phone offers to select its
       text or show a callout menu — both of which land on top of the amount
       this press is meant to open. */
    user-select: none;
    -webkit-touch-callout: none;
  }

  .front.settling {
    transition: transform var(--duration-spring) var(--ease-spring);
  }
</style>

