/**
 * The typed edge of the wasm core.
 *
 * `wasm-bindgen` declares `apply`, `state` and `mintDevice` as returning
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
 * twice — a device that has just been paired calls `mintDevice` first. Caching
 * the promise rather than a boolean means a second caller awaits the first
 * fetch instead of starting its own.
 */
let instantiated: Promise<unknown> | undefined;

function wasmReady(): Promise<unknown> {
  return (instantiated ??= initWasm());
}

/**
 * Narrow enough to reject a value written by an older or broken build.
 *
 * The two user fields accept `null` as well as a string, and that is not
 * laxity — it is the state a device is in between the twelve words and the
 * moment somebody is picked off the roster (DECISIONS 0068). Requiring
 * strings here would make a phone closed on the "Qui êtes-vous ?" screen come
 * back as a device that has never run: pairing again, a new device id, and a
 * dead peer left in the replica's history.
 *
 * The device fields stay mandatory, because a device with no id has nothing
 * to be.
 */
function isIdentity(value: unknown): value is Identity {
  if (typeof value !== 'object' || value === null) return false;
  const candidate = value as Record<string, unknown>;
  const optional = (field: unknown): boolean => field === null || typeof field === 'string';
  return (
    optional(candidate.user) &&
    optional(candidate.user_name) &&
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
 * name in the group roster, which is a cosmetic problem (Rule 7).
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
 * Mints the id of a device that has never run before. Called once, ever — the
 * result is what `localStorage` then holds forever.
 *
 * It says nothing about *who* is carrying it: that is a row on the group's
 * roster, and the roster arrives over the network (DECISIONS 0068). The
 * question is asked once the app is open, and the answer comes back through
 * `Core.identity` to be written down.
 *
 * The name is a first guess, replaced by whatever is typed on the way in.
 */
export async function mintDevice(deviceName: string): Promise<Identity> {
  await wasmReady();
  return CabasApp.mintDevice(deviceName) as Identity;
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
 * The id of an ingredient a picker is about to create.
 *
 * The same shape as `mintUsageId`, for the same reason one layer up. An
 * ingredient created from the list or from a recipe has to be *selected* in
 * the picker it was created from, and `SaveIngredient` returns the whole new
 * state rather than the id it minted — so the form that will need the id mints
 * it, sends it, and selects it (DECISIONS 0056). Inferring it by diffing the
 * library before and after would be a guess where this is a fact.
 */
export function mintIngredientId(): string {
  return CabasApp.mintIngredientId();
}

/**
 * Which build is running, from the core — the workspace version, which is
 * also the add-on's (Rule 15).
 *
 * Worth having on screen because a new build takes over a launch after it
 * arrives (DECISIONS 0038): without this, "did the update land" is answered by
 * counting relaunches. Synchronous for the same reason `mintUsageId` is — it
 * is only reachable once `Core.open` has instantiated the module.
 */
export function buildVersion(): string {
  return CabasApp.buildVersion();
}

/**
 * The most a photo may weigh, from the core.
 *
 * The encoder targets it and the core refuses anything above it, so there is
 * one number rather than two that drift (DECISIONS 0062). Synchronous for the
 * same reason `mintUsageId` is.
 */
export function maxPhotoBytes(): number {
  return CabasApp.maxPhotoBytes();
}

/**
 * A new group's recovery phrase — twelve words, minted once, on the device
 * that starts the group (DECISIONS 0042). Every other device joins with the
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

  /**
   * Who the core now thinks this device is.
   *
   * Several commands change it and `localStorage` holds the only durable
   * copy (DECISIONS 0031), so a caller that runs one reads this back and
   * remembers it, or the next launch is a device that has forgotten (0068).
   * `Session` names that set once, in `MOVES_IDENTITY`, rather than leaving
   * it to be remembered per call site.
   */
  identity(): Identity {
    return this.#app.identity() as Identity;
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
   * The photo half. The bytes live beside the document, one record per photo,
   * because the document is rewritten whole on every save (DECISIONS 0062).
   * Storing and attaching are two steps: this returns an id, and the id then
   * rides on the ordinary `SaveIngredient` or `SaveRecipe`.
   */
  putPhoto(bytes: Uint8Array): Promise<string> {
    return this.#app.putPhoto(bytes);
  }

  /**
   * The bytes of a photo this device holds, or `undefined`.
   *
   * Absent is an ordinary answer, not a failure: a photo taken on the other
   * phone is named by the document from the moment the replicas merge, and
   * its bytes arrive afterwards. The screen shows a placeholder (Rule 6).
   */
  photo(id: string): Promise<Uint8Array<ArrayBuffer> | undefined> {
    // The one cast this needs, and it belongs here: `wasm-bindgen` declares
    // `Uint8Array<ArrayBufferLike>`, which `Blob` refuses because a
    // `SharedArrayBuffer` cannot back one. Nothing shared ever crosses this
    // boundary, and saying so once keeps every caller cast-free.
    return this.#app.photo(id) as Promise<Uint8Array<ArrayBuffer> | undefined>;
  }

  /** What the replica references and this device has not got yet. */
  missingPhotos(): Promise<string[]> {
    return this.#app.missingPhotos() as Promise<string[]>;
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
