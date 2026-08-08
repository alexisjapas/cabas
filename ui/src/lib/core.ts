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
}
