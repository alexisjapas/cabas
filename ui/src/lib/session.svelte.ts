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
import type { ImportReport } from './bindings/ImportReport';
import type { StateView } from './bindings/StateView';
import { Core, rememberIdentity } from './core';
import { Sync } from './sync.svelte';

/**
 * The screens that exist. The current one is persisted, because an iOS cold
 * reload otherwise drops you on the home screen mid-shop — the most visible
 * flaw of an installed PWA, and the cheapest one to fix (DECISIONS 0003).
 *
 * *Which* screen is remembered; nothing about what it was showing is
 * (DECISIONS 0074). A tab opens cold.
 */
export type Screen = 'cart' | 'list' | 'recipes' | 'ingredients' | 'settings';

const SCREENS: readonly Screen[] = ['cart', 'list', 'recipes', 'ingredients', 'settings'];
const SCREEN_KEY = 'cabas.screen';

/**
 * The commands that change who this device says it is (DECISIONS 0068).
 *
 * The core's copy of the identity dies with the page; the durable one is the
 * host's (0031), so every one of these has to be written back to
 * `localStorage` after it applies. Running one and forgetting that works
 * perfectly until the next launch, which is the worst shape a bug can have —
 * so the set is enumerated here, once, and `run` consults it. A door beside
 * `run` was the earlier answer and it had the flaw every parallel door has:
 * `rename_user` moves the identity too, and went through `run`.
 */
const MOVES_IDENTITY: ReadonlySet<Command['command']> = new Set([
  'choose_user',
  'create_user',
  'name_device',
  'rename_user',
]);

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
      if (MOVES_IDENTITY.has(command.command)) rememberIdentity(this.#core.identity());
      this.#scheduleFlush();
      this.sync.localChange();
      return true;
    } catch (cause) {
      this.error = cause instanceof Error ? cause.message : String(cause);
      return false;
    }
  }

  /**
   * Stores a photo and returns the id to put on the draft being edited.
   *
   * Two steps rather than one, and the split is the core's (DECISIONS 0062):
   * the bytes go to a store of their own on a call that is awaited, and the
   * id then rides on the ordinary save. Nothing about a photo makes `run`
   * asynchronous.
   */
  putPhoto(bytes: Uint8Array): Promise<string> {
    return this.#core.putPhoto(bytes);
  }

  /**
   * The bytes of a photo, or `undefined` when this device has not got them.
   *
   * Absent is ordinary: a photo taken on the other phone is named by the
   * document as soon as the replicas merge, and arrives afterwards.
   */
  photo(id: string): Promise<Uint8Array<ArrayBuffer> | undefined> {
    return this.#core.photo(id);
  }

  /**
   * The whole library as a file (DECISIONS 0076). The core decides its shape
   * and its text; this hands back the string and nothing more.
   */
  exportLibrary(withPhotos: boolean): Promise<string> {
    return this.#core.exportLibrary(withPhotos);
  }

  /**
   * Merges a file in, and returns the receipt.
   *
   * Not `run`, because it is asynchronous — the photos in the file are a
   * browser transaction each — but it ends the same way a command does: the
   * new state renders, the write is scheduled, and the other phone is told
   * there is something to pull. An import is the largest single change this
   * app can make, and the one most worth pushing promptly.
   *
   * It throws rather than returning a boolean, unlike `run`: an import fails
   * for reasons that need saying — the wrong file, a newer format, a picture
   * that does not decode — where a refused command is nearly always a
   * concurrent edit.
   */
  async importLibrary(json: string): Promise<ImportReport> {
    try {
      const { report, state } = await this.#core.importLibrary(json);
      this.state = state;
      this.error = null;
      this.#scheduleFlush();
      this.sync.localChange();
      return report;
    } catch (cause) {
      this.error = cause instanceof Error ? cause.message : String(cause);
      throw cause;
    }
  }

  /**
   * Goes to a tab, and opens it cold (DECISIONS 0074).
   *
   * Everything a screen was in the middle of is dropped: the open recipe, the
   * editor under an ingredient, the search that narrowed the shelf, and how
   * far down it had been scrolled. Most of that is free — each screen lives
   * inside an `{#if}` in `App.svelte`, so switching away destroys the
   * component and its local state with it. The two that are not are here: the
   * open recipe is *core* state (`OpenRecipe`, device-local but persisted),
   * and the scroll offset belongs to the window.
   *
   * Tapping the tab you are already on does the same thing, deliberately: it
   * is the way back out of a recipe without hunting for the close button.
   */
  show(screen: Screen): void {
    // Only when there is one, so that changing tabs does not push a state and
    // schedule a write for a command that changes nothing.
    if (this.state.focus !== null) this.run({ command: 'close_recipe' });
    this.screen = screen;
    localStorage.setItem(SCREEN_KEY, screen);
    window.scrollTo(0, 0);
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
    };
    // `pagehide` is the one iOS fires reliably when a PWA is backgrounded;
    // `visibilitychange` covers app switching everywhere else. Both, because
    // neither alone catches every way this app stops being looked at.
    document.addEventListener('visibilitychange', () => {
      if (document.visibilityState === 'hidden') settle();
    });
    window.addEventListener('pagehide', settle);

    // The browser's own scroll restoration aims at a document that does not
    // exist yet — this one renders after the wasm core has loaded, so what it
    // would restore is an offset into a page that was empty at the time. A
    // launch lands at the top of the last tab, which is what `show` does too
    // (DECISIONS 0074).
    history.scrollRestoration = 'manual';
  }
}
