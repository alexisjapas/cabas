/**
 * The reactive layer: one state, one way to change it, and a save policy.
 *
 * # Why the state is `$state.raw`
 *
 * Every command returns a complete new `StateView` (DECISIONS 0033), so the
 * object is always replaced and never mutated in place. A deep `$state` proxy
 * would pay for tracking mutations that cannot happen, on a tree that arrives
 * fresh from wasm each time. `$state.raw` tracks the assignment, which is the
 * only thing that ever occurs.
 *
 * # Why saving is debounced, and why that is safe
 *
 * `apply` is synchronous and the render happens off its return value, so no
 * user action waits on IndexedDB (Rule 6, DECISIONS 0032). `flush` is left to
 * settle on its own a moment later; ticking five items in a row costs one
 * write rather than five. It is safe to coalesce because `flush` saves the
 * revision that was current when it started, so a command applied mid-write
 * stays pending rather than being counted as saved.
 *
 * The debounce is also why the page lifecycle listeners exist. iOS evicts a
 * backgrounded PWA whenever it likes (DECISIONS 0003), and a pending timer
 * dies with the page — so hiding the app flushes it immediately.
 */

import type { Command } from './bindings/Command';
import type { Identity } from './bindings/Identity';
import type { StateView } from './bindings/StateView';
import { Core } from './core';
import { Sync } from './sync.svelte';

/**
 * The screens that exist. The current one is persisted, because an iOS cold
 * reload otherwise drops you on the home screen mid-shop — the most visible
 * flaw of an installed PWA, and the cheapest one to fix (DECISIONS 0003).
 */
export type Screen = 'cart' | 'list' | 'recipes' | 'ingredients' | 'settings';

const SCREENS: readonly Screen[] = ['cart', 'list', 'recipes', 'ingredients', 'settings'];
const SCREEN_KEY = 'cabas.screen';
const SCROLL_KEY = 'cabas.scroll';

/**
 * Long enough that a burst of taps coalesces, short enough that the write has
 * landed before a thumb can background the app. The lifecycle listeners cover
 * the case where it does not.
 */
const FLUSH_DELAY_MS = 400;

function readScreen(): Screen {
  const stored = localStorage.getItem(SCREEN_KEY);
  return SCREENS.find((screen) => screen === stored) ?? 'cart';
}

/**
 * How far down each screen was left.
 *
 * Validated key by key rather than trusted: this is the one thing the app reads
 * back that no schema covers, and a hand-edited or half-written entry must cost
 * a screen its offset, not the launch.
 */
function readOffsets(): Partial<Record<Screen, number>> {
  const offsets: Partial<Record<Screen, number>> = {};
  try {
    const stored: unknown = JSON.parse(localStorage.getItem(SCROLL_KEY) ?? '{}');
    if (stored === null || typeof stored !== 'object') return offsets;
    for (const screen of SCREENS) {
      const offset: unknown = (stored as Record<string, unknown>)[screen];
      if (typeof offset === 'number' && Number.isFinite(offset) && offset > 0) {
        offsets[screen] = offset;
      }
    }
  } catch {
    // Not JSON. Every screen starts at the top, which is where it started
    // before any of this existed.
  }
  return offsets;
}

export class Session {
  readonly #core: Core;
  #flushTimer: ReturnType<typeof setTimeout> | undefined;

  /**
   * The socket and its policy. Public because a settings screen will want to
   * show what it is doing; it holds no business state — a frame that opens
   * arrives here as a whole `StateView`, like every other change.
   */
  readonly sync: Sync;

  /** The whole of what is on screen. Replaced, never edited. */
  state = $state.raw<StateView>(undefined as unknown as StateView);

  /** Which screen is showing. Device-local, and never synced. */
  screen = $state<Screen>('cart');

  /**
   * How far down each screen was left. Device-local like the screen itself, and
   * deliberately not reactive: it is written on every scroll event and read
   * once per screen change, so tracking it would invalidate a render per frame
   * to no end.
   */
  #offsets: Partial<Record<Screen, number>> = readOffsets();

  /**
   * Set from the moment a screen changes until its offset has been put back.
   *
   * A `scroll` event is delivered a frame after the scrolling, so the ones left
   * over from the outgoing screen arrive when `screen` already names the
   * incoming one — and recording those would overwrite the very offset about to
   * be restored with the outgoing screen's last position. Which is exactly what
   * happened: switching tabs and switching straight back landed at the top,
   * about half the time.
   */
  #settling = true;

  /**
   * The last command that was refused, in the app's own words. English, and
   * a diagnostic: it means another device deleted something under us, or a
   * typed quantity did not parse. Cleared by the next command that works.
   */
  error = $state<string | null>(null);

  private constructor(core: Core, state: StateView, screen: Screen) {
    this.#core = core;
    this.state = state;
    this.screen = screen;
    // A merged frame is a state change like any other, and the replica it
    // came from now differs from what is on disk — so it renders and it saves,
    // through exactly the paths a command uses.
    this.sync = new Sync(core, (state) => {
      this.state = state;
      this.#scheduleFlush();
    });
  }

  static async open(identity: Identity): Promise<Session> {
    const core = await Core.open(identity);
    const session = new Session(core, core.state(), readScreen());
    session.#watchPageLifecycle();
    // Does nothing on a device with no phrase, which is every device until
    // the pairing screens land.
    session.sync.start();
    return session;
  }

  /**
   * Applies one intent. Returns whether it was accepted, for the callers that
   * close a form on success and keep it open on failure.
   */
  run(command: Command): boolean {
    try {
      this.state = this.#core.apply(command);
      this.error = null;
      this.#scheduleFlush();
      this.sync.localChange();
      return true;
    } catch (cause) {
      this.error = cause instanceof Error ? cause.message : String(cause);
      return false;
    }
  }

  show(screen: Screen): void {
    // Read here rather than trusted from the last scroll event: this is the
    // last instant at which `window.scrollY` still belongs to the screen being
    // left.
    this.#offsets[this.screen] = window.scrollY;
    this.#settling = true;
    this.screen = screen;
    localStorage.setItem(SCREEN_KEY, screen);
    this.#saveOffsets();
  }

  /**
   * Puts the screen back where it was left. Called by `App.svelte` once the
   * screen has rendered — an offset means nothing before there is something to
   * scroll.
   *
   * Takes the screen it was queued for, because a second tap can land while the
   * first is still waiting for the DOM, and scrolling the new screen to the old
   * one's offset is worse than not scrolling at all.
   */
  restoreScroll(screen: Screen): void {
    if (screen !== this.screen) return;
    window.scrollTo(0, this.#offsets[screen] ?? 0);
    this.#settling = false;
  }

  #saveOffsets(): void {
    localStorage.setItem(SCROLL_KEY, JSON.stringify(this.#offsets));
  }

  dismissError(): void {
    this.error = null;
  }

  #scheduleFlush(): void {
    clearTimeout(this.#flushTimer);
    this.#flushTimer = setTimeout(() => void this.#flush(), FLUSH_DELAY_MS);
  }

  async #flush(): Promise<void> {
    clearTimeout(this.#flushTimer);
    this.#flushTimer = undefined;
    try {
      await this.#core.flush();
    } catch (cause) {
      // A failed write is the one error worth showing unprompted: everything
      // on screen is correct, and none of it is on disk.
      this.error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  #watchPageLifecycle(): void {
    const settle = (): void => {
      void this.#flush();
      this.#saveOffsets();
    };
    // `pagehide` is the one iOS fires reliably when a PWA is backgrounded;
    // `visibilitychange` covers app switching everywhere else. Both, because
    // neither alone catches every way this app stops being looked at.
    document.addEventListener('visibilitychange', () => {
      if (document.visibilityState === 'hidden') settle();
    });
    window.addEventListener('pagehide', settle);

    // Kept in memory on every frame and written down only when a screen is
    // left or the app is: a `localStorage` write per scroll event is a
    // synchronous disk touch per frame, on the one device that cannot spare it.
    window.addEventListener(
      'scroll',
      () => {
        if (this.#settling) return;
        this.#offsets[this.screen] = window.scrollY;
      },
      { passive: true },
    );

    // The browser's own scroll restoration aims at a document that does not
    // exist yet — this one renders after the wasm core has loaded. Ours runs
    // when there is something to scroll; theirs would only be a jump.
    history.scrollRestoration = 'manual';
  }
}
