/**
 * The core, as the frontend is allowed to see it.
 *
 * This file used to *be* the wasm edge. Since M7 it is the **interface** and
 * nothing else: one TypeScript surface with two implementations behind it —
 * `core.wasm.ts` in the PWA and `core.tauri.ts` in the Android app (DECISIONS
 * 0093, and Rule 9's "one TypeScript API surface, two implementations" as
 * literally as it can be written). Which one is imported is decided by the
 * `$core-host` alias in `vite.config.ts`, and that alias is the **only**
 * conditional in `ui/`: no screen, no engine and no component learns which
 * host it is running on.
 *
 * **Everything here is asynchronous, including what wasm answers instantly.**
 * Tauri's IPC is promise-based and has no synchronous form, so a surface whose
 * timing depended on the host would be two surfaces wearing one name — and the
 * bug that shape produces is a screen that works on one phone and races on the
 * other. The price is `Session.run` returning a promise on both platforms,
 * which is 0093's stated cost.
 *
 * It is not a violation of Rule 6 or of DECISIONS 0032. Rule 6 says no user
 * action waits on the **network**; an IPC hop to a core on the same device is
 * not one. 0032 says a render never waits on **storage**; `flush` is still the
 * call that writes.
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

/**
 * The replica, with the types the frontend is written against.
 *
 * Deliberately not reactive: reactivity is `Session`'s business, and mixing
 * the two would put a rune behind an FFI call.
 */
export interface Core {
  state(): Promise<StateView>;

  /** Returns the whole new state (DECISIONS 0033). */
  apply(command: Command): Promise<StateView>;

  /**
   * Who the core now thinks this device is.
   *
   * Several commands change it and the host holds the only durable copy
   * (DECISIONS 0031), so a caller that runs one reads this back and remembers
   * it, or the next launch is a device that has forgotten (0068). `Session`
   * names that set once, in `MOVES_IDENTITY`, rather than leaving it to be
   * remembered per call site.
   */
  identity(): Promise<Identity>;

  /** Resolves to `true` when it actually wrote. Never awaited by a render. */
  flush(): Promise<boolean>;

  /**
   * Whether this replica was built from nothing at launch, because storage
   * held no snapshot. A sync cursor from a previous life must not be resumed
   * on one — it would claim frames this replica never received.
   */
  openedFresh(): Promise<boolean>;

  /**
   * The photo half. The bytes live beside the document, one record per photo,
   * because the document is rewritten whole on every save (DECISIONS 0062).
   * Storing and attaching are two steps: this returns an id, and the id then
   * rides on the ordinary `SaveIngredient` or `SaveRecipe`.
   */
  putPhoto(bytes: Uint8Array): Promise<string>;

  /**
   * The bytes of a photo this device holds, or `undefined`.
   *
   * Absent is an ordinary answer, not a failure: a photo taken on the other
   * phone is named by the document from the moment the replicas merge, and
   * its bytes arrive afterwards. The screen shows a placeholder (Rule 6).
   */
  photo(id: string): Promise<Uint8Array<ArrayBuffer> | undefined>;

  /** What the replica references and this device has not got yet. */
  missingPhotos(): Promise<string[]>;

  /**
   * The library as a file, and back (DECISIONS 0076). The core decides the
   * file's shape and its text; this side does not parse it, does not build it,
   * and does not look inside it.
   *
   * `withPhotos` is the difference between a document and a backup: without
   * them the file is small enough to open in a text editor, which is the point
   * of it being JSON at all.
   */
  exportLibrary(withPhotos: boolean): Promise<string>;

  /**
   * Merges a file in, and returns the receipt together with the new state.
   *
   * Both in one call for the reason every mutation returns a state (DECISIONS
   * 0033): an import changes more of the screen than anything else the app
   * does, and fetching the state separately would leave a window in which the
   * report and the screen disagree.
   */
  importLibrary(json: string): Promise<Imported>;

  /**
   * The sync half. The socket is the frontend's on **both** hosts (DECISIONS
   * 0043, confirmed for Tauri by 0093) and these are the calls that drive it:
   * every `Uint8Array` below is opaque — a sealed frame going out, a wire
   * message coming in — and no plaintext ever crosses. A frame that opens is
   * merged inside the core, and what comes back is a state like any other.
   */

  /**
   * Starts a connection and returns the hello to send on it. `cursor` is what
   * the last connection ended on, or zeros on a device that has never synced.
   */
  syncHello(phrase: string, cursor: SyncCursor): Promise<Uint8Array>;

  /** One message off the socket, applied. */
  syncHandle(wire: Uint8Array): Promise<SyncEvent>;

  /**
   * Seals everything produced since `shadow` — the version returned by
   * {@link version} when the last push was acked, or an empty array on a
   * device that has never pushed.
   */
  syncPush(shadow: Uint8Array): Promise<Uint8Array>;

  /** Seals the whole replica, which lets the relay drop its log (0042). */
  syncSnapshot(): Promise<Uint8Array>;

  /**
   * The replica's version now — the shadow to adopt once the push carrying it
   * is acked. Read it *before* sending, so that an edit made while the push is
   * in flight stays unpushed rather than being counted as sent.
   */
  version(): Promise<Uint8Array>;

  /** The cursor to persist and the counters to show, or `null` between
   * connections. */
  syncStatus(): Promise<SyncStatus | null>;

  /** The socket closed. The next connection derives its key again. */
  syncClose(): Promise<void>;

  /**
   * The photo half, on a socket of its own (DECISIONS 0080, 0092).
   *
   * `photoHandle` and `photoPush` take the session out of the core's cell
   * while they await, so **only one of them may be in flight at a time**. The
   * engine in `photos.svelte.ts` chains them; calling two at once throws
   * rather than corrupting a queue.
   */

  /** Starts a photo connection and returns the hello to send on it. */
  photoHello(phrase: string): Promise<Uint8Array>;

  /** One message off the socket, applied — which for a photo means written. */
  photoHandle(wire: Uint8Array): Promise<PhotoEvent>;

  /** The next photo to ask for, or `undefined` when there is nothing left. */
  photoFetch(): Promise<Uint8Array | undefined>;

  /** The next photo to offer, sealed — or `undefined` when there is none. */
  photoPush(): Promise<Uint8Array | undefined>;

  /** Where the transfer has got to, or `null` between connections. */
  photoStatus(): Promise<PhotoStatus | null>;

  photoClose(): Promise<void>;
}

/**
 * What a host provides around the replica: the device's own memory of itself,
 * the id minters, and the two constants the core owns.
 *
 * The identity half is DECISIONS 0031, and M7 is where its second sentence
 * finally gets built — `localStorage` in the PWA, a file beside the replica
 * under Tauri. It is here rather than in {@link Core} because "where a device
 * remembers things about itself" is the host's business by definition, and
 * because it has to be readable *before* there is a replica to open.
 */
export interface Host {
  /**
   * The identity this device already has, or `null` on a device that has
   * never run.
   *
   * A stored value that does not parse is treated as absent rather than as an
   * error: the recovery is to mint a new identity, and refusing to start would
   * strand the person on a broken screen with no way out. The cost is a new
   * name in the group roster, which is a cosmetic problem (Rule 7).
   */
  readIdentity(): Promise<Identity | null>;

  rememberIdentity(identity: Identity): Promise<void>;

  /**
   * Mints the id of a device that has never run before. Called once, ever —
   * the result is what the host then holds forever.
   *
   * It says nothing about *who* is carrying it: that is a row on the group's
   * roster, and the roster arrives over the network (DECISIONS 0068). The
   * question is asked once the app is open, and the answer comes back through
   * {@link Core.identity} to be written down.
   *
   * The name is a first guess, replaced by whatever is typed on the way in.
   */
  mintDevice(deviceName: string): Promise<Identity>;

  /**
   * The id of a recipe line the editor is about to create.
   *
   * The editor needs it before there is anything to save: a step references a
   * *usage* rather than an ingredient (DECISIONS 0022), so mentioning a line
   * the user has just added means naming it, and `SaveRecipe` does not hand
   * the name back until after the save. Minting it up front is what lets the
   * whole recipe — lines and the prose that points at them — go out in one
   * command instead of a half-finished recipe being written to the library
   * first (DECISIONS 0039).
   *
   * From the core rather than from `crypto.randomUUID`, because two devices
   * adding a line to the same recipe offline must not choose the same id, and
   * because the format is the core's to decide.
   */
  mintUsageId(): Promise<string>;

  /**
   * The id of an ingredient a picker is about to create.
   *
   * The same shape as {@link mintUsageId}, for the same reason one layer up.
   * An ingredient created from the list or from a recipe has to be *selected*
   * in the picker it was created from, and `SaveIngredient` returns the whole
   * new state rather than the id it minted — so the form that will need the id
   * mints it, sends it, and selects it (DECISIONS 0056). Inferring it by
   * diffing the library before and after would be a guess where this is a
   * fact.
   */
  mintIngredientId(): Promise<string>;

  /**
   * The id of a shop the ingredient form is about to create.
   *
   * The third of the same shape, for the same reason one layer up (DECISIONS
   * 0071): the field where a shop's name is typed has to put the new shop on
   * the draft it is sitting in the instant it exists, and `SaveShop` hands
   * back a whole state rather than the id it minted.
   */
  mintShopId(): Promise<string>;

  /**
   * Which build is running, from the core — the workspace version, which is
   * also the add-on's (Rule 15).
   *
   * Worth having on screen because a new build takes over a launch after it
   * arrives (DECISIONS 0038): without this, "did the update land" is answered
   * by counting relaunches.
   */
  buildVersion(): Promise<string>;

  /**
   * The most a photo may weigh, from the core.
   *
   * The encoder targets it and the core refuses anything above it, so there is
   * one number rather than two that drift (DECISIONS 0062).
   */
  maxPhotoBytes(): Promise<number>;

  /**
   * A new group's recovery phrase — twelve words, minted once, on the device
   * that starts the group (DECISIONS 0042). Every other device joins with the
   * same words, scanned or typed (0021).
   */
  mintPhrase(): Promise<string>;

  /**
   * The canonical spelling of a phrase that was typed or scanned. Throws with
   * a message meant to be shown next to the field — wrong word count, a word
   * off the list, a checksum that says one was mistyped.
   *
   * The pairing screen calls this *before* storing anything, so that a bad
   * phrase fails there rather than looking like a relay that is down.
   */
  readPhrase(phrase: string): Promise<string>;

  /** Opens the replica this device holds. */
  openCore(identity: Identity): Promise<Core>;
}

export {
  buildVersion,
  maxPhotoBytes,
  mintDevice,
  mintIngredientId,
  mintPhrase,
  mintShopId,
  mintUsageId,
  openCore,
  readIdentity,
  readPhrase,
  rememberIdentity,
} from '$core-host';
