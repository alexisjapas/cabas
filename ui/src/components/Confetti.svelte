<script lang="ts">
  /**
   * One burst of confetti, over the thing that earned it (DECISIONS 0088).
   *
   * Mounted for a burst and unmounted after it: the pieces are built once, in
   * the initialiser, because a burst is not a state that changes — it is a
   * thing that happens. The caller renders it inside an `{#if}` and takes it
   * away again when `oncomplete` fires.
   *
   * # Why it is `position: fixed`, and why the origin is measured
   *
   * Confetti falls a long way, and an absolutely positioned box that long
   * contributes to the page's scrollable overflow — so a cart that fitted on
   * one screen would grow a scrollbar for a second and a half and then lose
   * it again. A fixed box contributes to nothing, at the price of having to
   * be told where the thing it is celebrating currently is. `origin` is that
   * measurement, taken by the caller at the moment of the burst.
   *
   * # Where the randomness is allowed to live
   *
   * Rule 10 says no component writes a visual value, and none is written
   * here: the colours are token *names*, the fall and the duration are tokens,
   * and the piece size is a token. What this file decides is where each piece
   * starts across the width, how far it drifts sideways, how long it waits and
   * how much it spins — none of which is a value anybody would want to set
   * from a stylesheet, and all of which is what stops twenty-four identical
   * squares from falling in a rank.
   */

  /** Where the burst comes from, in viewport coordinates. */
  export type Origin = { top: number; left: number; width: number };

  let {
    origin,
    oncomplete,
  }: {
    origin: Origin;
    /** Every piece has landed; the caller may unmount this. */
    oncomplete?: () => void;
  } = $props();

  /**
   * The palette, as token names. Everything that is a surface in this app is
   * fair game here — a piece of confetti is the one place a colour carries no
   * word and so has nothing to be legible against (DECISIONS 0081).
   */
  const COLOURS = ['--accent', '--ring', '--accent-soft', '--brand', '--check-a', '--surface'];

  /**
   * How many of each colour. Six colours, four each: enough to read as a
   * celebration on a phone, few enough that the whole burst is two dozen
   * absolutely positioned spans that exist for a second and a half.
   *
   * Counted this way round — a handful of passes over the palette rather than
   * a length and a modulo — so every piece provably has a colour. Indexing a
   * list by `i % length` is the same thing with an `undefined` in it.
   */
  const PER_COLOUR = 4;

  type Piece = {
    /** Across the bar, as a percentage of its width. */
    left: number;
    /** Sideways travel, as a fraction of the fall. Either way. */
    drift: number;
    /** As a fraction of the duration, so the burst arrives as a shower. */
    delay: number;
    /** Degrees, over the whole fall. */
    spin: number;
    /** Multiplies the size token; the token still owns the size. */
    scale: number;
    colour: string;
    round: boolean;
  };

  const pieces: Piece[] = Array.from({ length: PER_COLOUR }).flatMap((_, pass) =>
    COLOURS.map((colour, index) => ({
      colour,
      left: Math.random() * 100,
      drift: (Math.random() - 0.5) * 1.2,
      delay: Math.random() * 0.35,
      spin: 360 + Math.random() * 720,
      scale: 0.7 + Math.random() * 0.6,
      round: (pass + index) % 3 === 0,
    })),
  );

  /**
   * Counted rather than timed. The duration is a token and reading a token
   * back out of `getComputedStyle` is the trap `--press-delay` already carries
   * — the minifier ships `500ms` as `.5s` — so the animation says when it is
   * over and nothing here has to know how long it was.
   */
  let landed = 0;

  function settle(): void {
    landed += 1;
    if (landed >= pieces.length) oncomplete?.();
  }
</script>

<div
  class="confetti"
  aria-hidden="true"
  style="--burst-top: {origin.top}px; --burst-left: {origin.left}px; --burst-width: {origin.width}px"
  onanimationend={settle}
>
  {#each pieces as piece, index (index)}
    <span
      class="piece"
      class:round={piece.round}
      style="
        left: {piece.left}%;
        background: var({piece.colour});
        --piece-drift: {piece.drift};
        --piece-delay: {piece.delay};
        --piece-spin: {piece.spin}deg;
        --piece-scale: {piece.scale};
      "
    ></span>
  {/each}
</div>

<style>
  /*
   * Fixed, over the bar that earned it, and untouchable: a burst must never
   * eat the tap that follows it — somebody who has just ticked the last line
   * is reaching for "Terminer les courses".
   */
  .confetti {
    position: fixed;
    top: var(--burst-top);
    left: var(--burst-left);
    width: var(--burst-width);
    height: var(--confetti-fall);
    z-index: var(--layer-alert);
    pointer-events: none;
    /* The pieces drift sideways as they fall and are clipped when they leave;
       without this a phone gets a horizontal scrollbar for a second. */
    overflow: hidden;
  }

  .piece {
    position: absolute;
    top: 0;
    width: var(--confetti-size);
    height: var(--confetti-size);
    border-radius: var(--radius-sm);
    animation: confetti var(--duration-confetti) var(--ease-out) forwards;
    animation-delay: calc(var(--piece-delay) * var(--duration-confetti));
  }

  .round {
    border-radius: var(--radius-pill);
  }

  @keyframes confetti {
    from {
      transform: translate(0, 0) rotate(0deg) scale(var(--piece-scale));
      opacity: 1;
    }
    to {
      transform: translate(calc(var(--confetti-fall) * var(--piece-drift)), var(--confetti-fall))
        rotate(var(--piece-spin)) scale(var(--piece-scale));
      opacity: 0;
    }
  }
</style>
