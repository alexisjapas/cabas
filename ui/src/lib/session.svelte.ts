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

/**
 * The screens that exist. The current one is persisted, because an iOS cold
 * reload otherwise drops you on the home screen mid-shop — the most visible
 * flaw of an installed PWA, and the cheapest one to fix (DECISIONS 0003).
 */
export type Screen = 'cart' | 'list' | 'ingredients' | 'settings';

const SCREENS: readonly Screen[] = ['cart', 'list', 'ingredients', 'settings'];
const SCREEN_KEY = 'cabas.screen';

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

export class Session {
  readonly #core: Core;
  #flushTimer: ReturnType<typeof setTimeout> | undefined;

  /** The whole of what is on screen. Replaced, never edited. */
  state = $state.raw<StateView>(undefined as unknown as StateView);

  /** Which screen is showing. Device-local, and never synced. */
  screen = $state<Screen>('cart');

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
  }

  static async open(identity: Identity): Promise<Session> {
    const core = await Core.open(identity);
    const session = new Session(core, core.state(), readScreen());
    session.#watchPageLifecycle();
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
      return true;
    } catch (cause) {
      this.error = cause instanceof Error ? cause.message : String(cause);
      return false;
    }
  }

  show(screen: Screen): void {
    this.screen = screen;
    localStorage.setItem(SCREEN_KEY, screen);
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
    const flushNow = (): void => void this.#flush();
    // `pagehide` is the one iOS fires reliably when a PWA is backgrounded;
    // `visibilitychange` covers app switching everywhere else. Both, because
    // neither alone catches every way this app stops being looked at.
    document.addEventListener('visibilitychange', () => {
      if (document.visibilityState === 'hidden') flushNow();
    });
    window.addEventListener('pagehide', flushNow);
  }
}
