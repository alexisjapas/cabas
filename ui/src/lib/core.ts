/**
 * The typed edge of the wasm core.
 *
 * `wasm-bindgen` declares `apply`, `state` and `mintIdentity` as returning
 * `any`, because what actually crosses is a `serde` value and the glue has no
 * idea what shape it has. The shape is known — it is generated into
 * `./bindings/` from the Rust types (DECISIONS 0036) — so **this file is the
 * one place the two are tied together**, and the only place a cast is
 * allowed. Everything above it is checked.
 *
 * It also owns the other half of DECISIONS 0031: the device identity lives in
 * `localStorage` here, because where a device remembers things about itself
 * is the host's business, and this is the host.
 */

import type { Command } from './bindings/Command';
import type { Identity } from './bindings/Identity';
import type { StateView } from './bindings/StateView';
import type { SyncCursor } from './bindings/SyncCursor';
import type { SyncEvent } from './bindings/SyncEvent';
import type { SyncStatus } from './bindings/SyncStatus';
import initWasm, { CabasApp } from './wasm/cabas';

/** Namespaced, because `localStorage` is shared by everything on the origin. */
const IDENTITY_KEY = 'cabas.identity';

/**
 * The module is instantiated once per page load, and `open` may be reached
 * twice — the onboarding path calls `mintIdentity` first. Caching the promise
 * rather than a boolean means a second caller awaits the first fetch instead
 * of starting its own.
 */
let instantiated: Promise<unknown> | undefined;

function wasmReady(): Promise<unknown> {
  return (instantiated ??= initWasm());
}

/** Narrow enough to reject a value written by an older or broken build. */
function isIdentity(value: unknown): value is Identity {
  if (typeof value !== 'object' || value === null) return false;
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.user === 'string' &&
    typeof candidate.user_name === 'string' &&
    typeof candidate.device === 'string' &&
    typeof candidate.device_name === 'string'
  );
}

/**
 * The identity this device already has, or `null` on a device that has never
 * run.
 *
 * A stored value that does not parse is treated as absent rather than as an
 * error: the recovery is to mint a new identity, and refusing to start would
 * strand the person on a broken screen with no way out. The cost is a new
 * name in the family roster, which is a cosmetic problem (Rule 7).
 */
export function readIdentity(): Identity | null {
  const stored = localStorage.getItem(IDENTITY_KEY);
  if (stored === null) return null;
  try {
    const parsed: unknown = JSON.parse(stored);
    return isIdentity(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

export function rememberIdentity(identity: Identity): void {
  localStorage.setItem(IDENTITY_KEY, JSON.stringify(identity));
}

/**
 * Mints ids for a device that has never run before. Called once, ever — the
 * result is what `localStorage` then holds forever.
 */
export async function mintIdentity(userName: string, deviceName: string): Promise<Identity> {
  await wasmReady();
  return CabasApp.mintIdentity(userName, deviceName) as Identity;
}

/**
 * The id of a recipe line the editor is about to create.
 *
 * The editor needs it before there is anything to save: a step references a
 * *usage* rather than an ingredient (DECISIONS 0022), so mentioning a line the
 * user has just added means naming it, and `SaveRecipe` does not hand the name
 * back until after the save. Minting it up front is what lets the whole recipe
 * — lines and the prose that points at them — go out in one command instead of
 * a half-finished recipe being written to the library first (DECISIONS 0039).
 *
 * From the core rather than from `crypto.randomUUID`, because two devices
 * adding a line to the same recipe offline must not choose the same id, and
 * because the format is the core's to decide. Synchronous, and safe to be:
 * the editor is only reachable once `Core.open` has instantiated the module.
 */
export function mintUsageId(): string {
  return CabasApp.mintUsageId();
}

/**
 * A new family's recovery phrase — twelve words, minted once, on the device
 * that starts the family (DECISIONS 0042). Every other device joins with the
 * same words, scanned or typed (0021).
 */
export async function mintPhrase(): Promise<string> {
  await wasmReady();
  return CabasApp.mintPhrase();
}

/**
 * The canonical spelling of a phrase that was typed or scanned. Throws with a
 * message meant to be shown next to the field — wrong word count, a word off
 * the list, a checksum that says one was mistyped.
 *
 * The pairing screen calls this *before* storing anything, so that a bad
 * phrase fails there rather than looking like a relay that is down.
 */
export async function readPhrase(phrase: string): Promise<string> {
  await wasmReady();
  return CabasApp.readPhrase(phrase);
}

/**
 * The replica, with the types the frontend is written against.
 *
 * Deliberately not reactive: reactivity is `Session`'s business, and mixing
 * the two would put a rune behind an FFI call.
 */
export class Core {
  readonly #app: CabasApp;

  private constructor(app: CabasApp) {
    this.#app = app;
  }

  static async open(identity: Identity): Promise<Core> {
    await wasmReady();
    return new Core(await CabasApp.open(identity));
  }

  state(): StateView {
    return this.#app.state() as StateView;
  }

  /** Synchronous, and returns the whole new state (DECISIONS 0032, 0033). */
  apply(command: Command): StateView {
    return this.#app.apply(command) as StateView;
  }

  /** Resolves to `true` when it actually wrote. Never awaited by a render. */
  flush(): Promise<boolean> {
    return this.#app.flush();
  }

  /**
   * Whether this replica was built from nothing at launch, because storage
   * held no snapshot. A sync cursor from a previous life must not be resumed
   * on one — it would claim frames this replica never received.
   */
  openedFresh(): boolean {
    return this.#app.openedFresh();
  }

  /**
   * The sync half. The socket is ours (DECISIONS 0043) and these are the calls
   * that drive it: every `Uint8Array` below is opaque — a sealed frame going
   * out, a wire message coming in — and no plaintext ever crosses. A frame
   * that opens is merged inside the core, and what comes back is a state like
   * any other.
   */

  /**
   * Starts a connection and returns the hello to send on it. `cursor` is what
   * the last connection ended on, or zeros on a device that has never synced.
   */
  syncHello(phrase: string, cursor: SyncCursor): Uint8Array {
    return this.#app.syncHello(phrase, cursor);
  }

  /** One message off the socket, applied. */
  syncHandle(wire: Uint8Array): SyncEvent {
    return this.#app.syncHandle(wire) as SyncEvent;
  }

  /**
   * Seals everything produced since `shadow` — the version returned by
   * {@link version} when the last push was acked, or an empty array on a
   * device that has never pushed.
   */
  syncPush(shadow: Uint8Array): Uint8Array {
    return this.#app.syncPush(shadow);
  }

  /** Seals the whole replica, which lets the relay drop its log (0042). */
  syncSnapshot(): Uint8Array {
    return this.#app.syncSnapshot();
  }

  /**
   * The replica's version now — the shadow to adopt once the push carrying it
   * is acked. Read it *before* sending, so that an edit made while the push is
   * in flight stays unpushed rather than being counted as sent.
   */
  version(): Uint8Array {
    return this.#app.syncVersion();
  }

  /** The cursor to persist and the counters to show, or `null` between
   * connections. */
  syncStatus(): SyncStatus | null {
    return this.#app.syncStatus() as SyncStatus | null;
  }

  /** The socket closed. The next connection derives its key again. */
  syncClose(): void {
    this.#app.syncClose();
  }
}
