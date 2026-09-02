/**
 * The Tauri host: the Android and Linux implementation of `core.ts`
 * (DECISIONS 0093).
 *
 * Every export here is one `invoke` and nothing else. The counterpart of
 * `core.wasm.ts`, and thin for the same reason that file is thin: the
 * decisions live in `cabas-app`, and a host that decided anything would be a
 * second place to keep the two platforms in step.
 *
 * # Why there are no casts
 *
 * `core.wasm.ts` is full of them, because `wasm-bindgen` declares everything
 * as `any` and that file is where the generated types are tied back on.
 * Tauri's `invoke` is generic — `invoke<StateView>(…)` returns a
 * `Promise<StateView>` — so the same tying-on is a type argument. It is no
 * more checked than the casts are: the guarantee on both hosts is that
 * `crates/app` serialises what `ui/src/lib/bindings/` declares, and CI
 * regenerates those bindings from the Rust types (DECISIONS 0036).
 *
 * # Names
 *
 * Tauri derives a command's name from the Rust function, so these are the
 * `snake_case` names in `src-tauri/src/lib.rs` — spelled here once, next to
 * the call, rather than renamed on the Rust side to please JavaScript.
 *
 * # The identity is a file here
 *
 * `localStorage` in the PWA, a JSON file beside the replica under Tauri
 * (DECISIONS 0031, whose second half was written and never built until M7).
 * Nothing above this module can tell which it is.
 */

import { invoke } from '@tauri-apps/api/core';

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

export function readIdentity(): Promise<Identity | null> {
  return invoke<Identity | null>('read_identity');
}

export function rememberIdentity(identity: Identity): Promise<void> {
  return invoke('remember_identity', { identity });
}

export function mintDevice(deviceName: string): Promise<Identity> {
  return invoke<Identity>('mint_device', { deviceName });
}

export function mintUsageId(): Promise<string> {
  return invoke<string>('mint_usage_id');
}

export function mintIngredientId(): Promise<string> {
  return invoke<string>('mint_ingredient_id');
}

export function mintShopId(): Promise<string> {
  return invoke<string>('mint_shop_id');
}

export function buildVersion(): Promise<string> {
  return invoke<string>('build_version');
}

export function maxPhotoBytes(): Promise<number> {
  return invoke<number>('max_photo_bytes');
}

export function mintPhrase(): Promise<string> {
  return invoke<string>('mint_phrase');
}

export function readPhrase(phrase: string): Promise<string> {
  return invoke<string>('read_phrase', { phrase });
}

/**
 * Opens the replica and hands back the surface over it.
 *
 * On the PWA this constructs an object; here it fills state that already
 * exists on the Rust side, because Tauri sets its state up before there is a
 * window. Either way the frontend decides *when* — there is nothing to open
 * until the host says who this device is.
 */
export async function openCore(identity: Identity): Promise<Core> {
  await invoke('open', { identity });
  return new TauriCore();
}

/**
 * Bytes cross the IPC boundary as arrays of numbers, not as `Uint8Array`.
 *
 * Tauri serialises a command's arguments as JSON, so a `Uint8Array` would
 * arrive at a `Vec<u8>` as an object with numeric keys and fail to
 * deserialise. Going out, `serde` writes a `Vec<u8>` as a JSON array and it
 * arrives here as `number[]`. Both directions are converted **here**, once,
 * so that everything above this file speaks `Uint8Array` on both hosts —
 * which is what lets `sync.svelte.ts` and `photos.svelte.ts` be the same
 * files on the PWA and in the APK.
 */
function bytesOut(bytes: Uint8Array): number[] {
  return Array.from(bytes);
}

function bytesIn(bytes: number[]): Uint8Array<ArrayBuffer> {
  return new Uint8Array(bytes);
}

class TauriCore implements Core {
  state(): Promise<StateView> {
    return invoke<StateView>('state');
  }

  apply(command: Command): Promise<StateView> {
    return invoke<StateView>('apply', { command });
  }

  identity(): Promise<Identity> {
    return invoke<Identity>('identity');
  }

  flush(): Promise<boolean> {
    return invoke<boolean>('flush');
  }

  openedFresh(): Promise<boolean> {
    return invoke<boolean>('opened_fresh');
  }

  putPhoto(bytes: Uint8Array): Promise<string> {
    return invoke<string>('put_photo', { bytes: bytesOut(bytes) });
  }

  async photo(id: string): Promise<Uint8Array<ArrayBuffer> | undefined> {
    const bytes = await invoke<number[] | null>('photo', { id });
    // `null` is an ordinary answer, and the surface says `undefined`: a photo
    // taken on the other phone is named by the document before its bytes
    // arrive, and the screen shows a placeholder (Rule 6).
    return bytes === null ? undefined : bytesIn(bytes);
  }

  missingPhotos(): Promise<string[]> {
    return invoke<string[]>('missing_photos');
  }

  exportLibrary(withPhotos: boolean): Promise<string> {
    return invoke<string>('export_library', { withPhotos });
  }

  importLibrary(json: string): Promise<Imported> {
    return invoke<Imported>('import_library', { json });
  }

  async syncHello(phrase: string, cursor: SyncCursor): Promise<Uint8Array> {
    return bytesIn(await invoke<number[]>('sync_hello', { phrase, cursor }));
  }

  syncHandle(wire: Uint8Array): Promise<SyncEvent> {
    return invoke<SyncEvent>('sync_handle', { wire: bytesOut(wire) });
  }

  async syncPush(shadow: Uint8Array): Promise<Uint8Array> {
    return bytesIn(await invoke<number[]>('sync_push', { shadow: bytesOut(shadow) }));
  }

  async syncSnapshot(): Promise<Uint8Array> {
    return bytesIn(await invoke<number[]>('sync_snapshot'));
  }

  async version(): Promise<Uint8Array> {
    return bytesIn(await invoke<number[]>('sync_version'));
  }

  syncStatus(): Promise<SyncStatus | null> {
    return invoke<SyncStatus | null>('sync_status');
  }

  syncClose(): Promise<void> {
    return invoke('sync_close');
  }

  async photoHello(phrase: string): Promise<Uint8Array> {
    return bytesIn(await invoke<number[]>('photo_hello', { phrase }));
  }

  photoHandle(wire: Uint8Array): Promise<PhotoEvent> {
    return invoke<PhotoEvent>('photo_handle', { wire: bytesOut(wire) });
  }

  async photoFetch(): Promise<Uint8Array | undefined> {
    const bytes = await invoke<number[] | null>('photo_fetch');
    return bytes === null ? undefined : bytesIn(bytes);
  }

  async photoPush(): Promise<Uint8Array | undefined> {
    const bytes = await invoke<number[] | null>('photo_push');
    return bytes === null ? undefined : bytesIn(bytes);
  }

  photoStatus(): Promise<PhotoStatus | null> {
    return invoke<PhotoStatus | null>('photo_status');
  }

  photoClose(): Promise<void> {
    return invoke('photo_close');
  }
}
