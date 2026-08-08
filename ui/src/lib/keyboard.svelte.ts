/**
 * The soft keyboard, as a number the layout can use.
 *
 * # Why this has to exist at all
 *
 * iOS does not resize the page when the keyboard opens. The layout viewport
 * keeps its full height and the keys are drawn over the bottom of it, so
 * `100dvh`, `position: fixed` and `env(safe-area-inset-bottom)` all go on
 * describing a viewport whose bottom third has quietly become unreachable.
 * Nothing in CSS can see that. `visualViewport` is the only API that reports
 * it, which makes a keyboard the one piece of layout this app has to compute
 * in script.
 *
 * # What it publishes
 *
 * One number, in two forms.
 *
 * `--keyboard-inset` on the root element is what the layout reads: a screen's
 * body pads by it, the tab bar slides down by it. Both go through `max()`, and
 * both get `0px` when nothing is covering anything — so the closed state is the
 * layout that was there before this file existed, and there is no mode to
 * leave. That is the whole reason it is a length and not a class.
 *
 * `keyboard.inset` is the same value as reactive state, for the one thing CSS
 * cannot do: scrolling something out from under the keyboard *after* it has
 * opened (`reveal`).
 *
 * Watched from `App.svelte` rather than from `Session`, because the first
 * screen this app ever shows is a form — onboarding runs before there is a
 * session to hang anything off (DECISIONS 0031).
 */

/**
 * Below this, whatever moved was not a keyboard.
 *
 * A collapsing address bar and an iPad's accessory strip are tens of pixels;
 * every soft keyboard is hundreds. Sliding the tab bar off the screen because
 * Safari's chrome moved would be a glitch on a screen nobody is typing into.
 */
const MIN_KEYBOARD_PX = 80;

/**
 * A pinched-in page reports a smaller visual viewport for a reason that has
 * nothing to do with a keyboard, and padding the document by the difference
 * would be nonsense. The tolerance is there because a scale that has been
 * touched does not reliably come back to exactly 1.
 */
const MAX_UNZOOMED_SCALE = 1.05;

/**
 * How much of the layout viewport is covered, from the only two numbers that
 * know: the visible band's height and where it starts.
 *
 * `offsetTop` is the visual viewport's own displacement inside the layout one —
 * iOS scrolls it up to keep the caret above the keys rather than resizing
 * anything — so the two together are where the band the user can see ends.
 */
function measureInset(viewport: VisualViewport): number {
  if (viewport.scale > MAX_UNZOOMED_SCALE) return 0;
  const covered = document.documentElement.clientHeight - (viewport.height + viewport.offsetTop);
  const inset = Math.max(0, Math.round(covered));
  return inset < MIN_KEYBOARD_PX ? 0 : inset;
}

class Keyboard {
  /**
   * How much of the layout viewport the keyboard is covering, in CSS pixels;
   * zero when nothing is.
   *
   * Reactive, but it moves at human speed: it is assigned only when the rounded
   * value actually changes, so the stream of events a pan or a rubber-band
   * fires on the visual viewport costs nothing. (The scroll offsets in
   * `session.svelte.ts` reach the same place from the other side — written
   * every frame and read once, so they are deliberately not state at all.)
   */
  inset = $state(0);

  /** Called once, from the app's `onMount`. */
  watch(): void {
    const viewport = window.visualViewport;
    // A browser without it keeps the layout that was there before any of this,
    // which is the correct one everywhere a keyboard does not overlap the page.
    if (viewport === null) return;

    const measure = (): void => this.#publish(measureInset(viewport));

    viewport.addEventListener('resize', measure);
    // `scroll`, because the keyboard also moves the visual viewport without
    // resizing it, and `offsetTop` is half of what is being measured.
    viewport.addEventListener('scroll', measure);
    measure();
  }

  #publish(inset: number): void {
    if (inset === this.inset) return;
    this.inset = inset;
    document.documentElement.style.setProperty('--keyboard-inset', `${inset}px`);
  }
}

export const keyboard = new Keyboard();

/**
 * Scrolls `element` out from under the keyboard, if it is under it.
 *
 * `scrollIntoView` cannot do this. To the browser the element is already
 * visible, because visible means inside the layout viewport — and on iOS the
 * layout viewport is precisely what the keyboard is covering. The distance to
 * travel is the element's bottom minus the last row of pixels still on screen.
 *
 * How much room to leave above the keys is a visual decision, so it stays in
 * CSS: the element's own `scroll-margin-bottom`, which is what that property
 * already means (Rule 10).
 *
 * The inset is a parameter rather than a read, so that a caller scheduling this
 * behind a `tick()` reads it — and therefore depends on it — while what the
 * scroll acts on is the geometry that scheduled the call.
 */
export function reveal(element: HTMLElement, inset: number): void {
  const visibleBottom = document.documentElement.clientHeight - inset;
  const margin = Number.parseFloat(getComputedStyle(element).scrollMarginBottom);
  const overflow =
    element.getBoundingClientRect().bottom + (Number.isNaN(margin) ? 0 : margin) - visibleBottom;
  if (overflow <= 0) return;

  window.scrollBy({
    top: overflow,
    // Animated everywhere the app animates, and not where the system asked for
    // no motion (app.css turns its own durations off the same way).
    behavior: window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth',
  });
}
