/**
 * The wasm host: the PWA's implementation of `core.ts` (DECISIONS 0093).
 *
 * `wasm-bindgen` declares `apply`, `state` and `mintDevice` as returning
 * `any`, because what actually crosses is a `serde` value and the glue has no
 * idea what shape it has. The shape is known — it is generated into
 * `./bindings/` from the Rust types (DECISIONS 0036) — so **this file is the
 * one place the two are tied together**, and the only place a cast is
 * allowed. Everything above it is checked.
 *
 * It also owns the PWA's half of DECISIONS 0031: the device identity lives in
 * `localStorage` here, because where a device remembers things about itself is
 * the host's business, and this is the host. The Android host writes a file
 * instead, and nothing above either of them can tell.
 *
 * **Every export is asynchronous even where wasm answers instantly**, and that
 * is the interface's rule rather than this file's taste — see `core.ts`. The
 * `async` keyword on a call that does no waiting costs a microtask and buys
 * one surface instead of two.
 */

import type { Command } from './bindings/Command';
import type { Identity } from './bindings/Identity';
import type { Imported } from './bindings/Imported';
import type { PhotoEvent } from './bindings/PhotoEvent';
import type { PhotoStatus } from './bindings/PhotoStatus';
import type { StateView } from './bindings/StateView';
import type { SyncCursor } from './bindings/SyncCursor';
import type { SyncEvent } from './bindings/SyncEvent';
import type { SyncStatus } from './bindings/SyncStatus';
import type { Core } from './core';
import initWasm, { CabasApp } from './wasm/cabas';

/** Namespaced, because `localStorage` is shared by everything on the origin. */
const IDENTITY_KEY = 'cabas.identity';

/**
 * The module is instantiated once per page load, and `openCore` may be reached
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

export async function readIdentity(): Promise<Identity | null> {
  const stored = localStorage.getItem(IDENTITY_KEY);
  if (stored === null) return null;
  try {
    const parsed: unknown = JSON.parse(stored);
    return isIdentity(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

export async function rememberIdentity(identity: Identity): Promise<void> {
  localStorage.setItem(IDENTITY_KEY, JSON.stringify(identity));
}

export async function mintDevice(deviceName: string): Promise<Identity> {
  await wasmReady();
  return CabasApp.mintDevice(deviceName) as Identity;
}

export async function mintUsageId(): Promise<string> {
  await wasmReady();
  return CabasApp.mintUsageId();
}

export async function mintIngredientId(): Promise<string> {
  await wasmReady();
  return CabasApp.mintIngredientId();
}

export async function mintShopId(): Promise<string> {
  await wasmReady();
  return CabasApp.mintShopId();
}

export async function buildVersion(): Promise<string> {
  await wasmReady();
  return CabasApp.buildVersion();
}

export async function maxPhotoBytes(): Promise<number> {
  await wasmReady();
  return CabasApp.maxPhotoBytes();
}

export async function mintPhrase(): Promise<string> {
  await wasmReady();
  return CabasApp.mintPhrase();
}

export async function readPhrase(phrase: string): Promise<string> {
  await wasmReady();
  return CabasApp.readPhrase(phrase);
}

/** The relay serves the bundle and `/sync` on one origin, so there is
 *  nothing to configure: `relayUrl` derives it from `location` (0012, 0048). */
export async function defaultRelay(): Promise<string | null> {
  return null;
}

export async function openCore(identity: Identity): Promise<Core> {
  await wasmReady();
  return new WasmCore(await CabasApp.open(identity));
}

class WasmCore implements Core {
  readonly #app: CabasApp;

  constructor(app: CabasApp) {
    this.#app = app;
  }

  async state(): Promise<StateView> {
    return this.#app.state() as StateView;
  }

  async apply(command: Command): Promise<StateView> {
    return this.#app.apply(command) as StateView;
  }

  async identity(): Promise<Identity> {
    return this.#app.identity() as Identity;
  }

  flush(): Promise<boolean> {
    return this.#app.flush();
  }

  async openedFresh(): Promise<boolean> {
    return this.#app.openedFresh();
  }

  putPhoto(bytes: Uint8Array): Promise<string> {
    return this.#app.putPhoto(bytes);
  }

  photo(id: string): Promise<Uint8Array<ArrayBuffer> | undefined> {
    // The one cast this needs, and it belongs here: `wasm-bindgen` declares
    // `Uint8Array<ArrayBufferLike>`, which `Blob` refuses because a
    // `SharedArrayBuffer` cannot back one. Nothing shared ever crosses this
    // boundary, and saying so once keeps every caller cast-free.
    return this.#app.photo(id) as Promise<Uint8Array<ArrayBuffer> | undefined>;
  }

  missingPhotos(): Promise<string[]> {
    return this.#app.missingPhotos() as Promise<string[]>;
  }

  exportLibrary(withPhotos: boolean): Promise<string> {
    return this.#app.exportLibrary(withPhotos) as Promise<string>;
  }

  importLibrary(json: string): Promise<Imported> {
    return this.#app.importLibrary(json) as Promise<Imported>;
  }

  async syncHello(phrase: string, cursor: SyncCursor): Promise<Uint8Array> {
    return this.#app.syncHello(phrase, cursor);
  }

  async syncHandle(wire: Uint8Array): Promise<SyncEvent> {
    return this.#app.syncHandle(wire) as SyncEvent;
  }

  async syncPush(shadow: Uint8Array): Promise<Uint8Array> {
    return this.#app.syncPush(shadow);
  }

  async syncSnapshot(): Promise<Uint8Array> {
    return this.#app.syncSnapshot();
  }

  async version(): Promise<Uint8Array> {
    return this.#app.syncVersion();
  }

  async syncStatus(): Promise<SyncStatus | null> {
    return this.#app.syncStatus() as SyncStatus | null;
  }

  async syncClose(): Promise<void> {
    this.#app.syncClose();
  }

  photoHello(phrase: string): Promise<Uint8Array> {
    return this.#app.photoHello(phrase);
  }

  photoHandle(wire: Uint8Array): Promise<PhotoEvent> {
    return this.#app.photoHandle(wire) as Promise<PhotoEvent>;
  }

  async photoFetch(): Promise<Uint8Array | undefined> {
    return this.#app.photoFetch();
  }

  photoPush(): Promise<Uint8Array | undefined> {
    return this.#app.photoPush();
  }

  async photoStatus(): Promise<PhotoStatus | null> {
    return this.#app.photoStatus() as PhotoStatus | null;
  }

  async photoClose(): Promise<void> {
    this.#app.photoClose();
  }
}
