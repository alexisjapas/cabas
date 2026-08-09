/**
 * The sync engine: one socket, and the policy around it.
 *
 * The core owns the protocol and the crypto; this file owns the WebSocket, the
 * reconnection, and *when* to sync (DECISIONS 0043). It sits next to
 * `session.svelte.ts` because that is where the page lifecycle already lives,
 * and the foreground rule is a page-lifecycle rule.
 *
 * # No background sync (DECISIONS 0011)
 *
 * The socket is open while the app is on screen and closed the moment it is
 * not. There is no service-worker sync, no push, and no timer running behind a
 * backgrounded PWA — iOS would kill it anyway, and pretending otherwise costs
 * battery to be wrong. Coming back to the app reconnects and replays, which on
 * a family library is a few frames.
 *
 * # What is persisted, and why it is two keys
 *
 * The phrase and the relay URL are written once, when pairing. The cursor and
 * the shadow move constantly. They live in **separate** `localStorage` keys so
 * that the hot path never rewrites the secret: a half-written cursor costs a
 * replay, a half-written phrase would cost the family.
 *
 * # Everything here is opaque
 *
 * Every `Uint8Array` below is either a sealed frame or a wire message. This
 * file never looks inside one: a frame that opens is merged by the core, which
 * hands back the same whole `StateView` a command would (Rule 9).
 */

import type { StateView } from './bindings/StateView';
import type { SyncCursor } from './bindings/SyncCursor';
import type { Core } from './core';

/** Written at pairing time, read at every connection. */
const FAMILY_KEY = 'cabas.family';
/** Written constantly; holds nothing secret. */
const CURSOR_KEY = 'cabas.sync';

/**
 * Long enough for a burst of taps in a shop to leave as one frame, short
 * enough that the other phone sees it while you are still standing there. The
 * relay's log grows by one entry per push, so coalescing is also what keeps it
 * short between compactions (DECISIONS 0042).
 */
const PUSH_DELAY_MS = 700;

/** First backoff step, then doubling, capped. Jittered so two phones waking
 *  together do not retry in lockstep forever. */
const RETRY_MIN_MS = 1_000;
const RETRY_MAX_MS = 30_000;

/** What the UI may show about the connection. Nothing here is business state:
 *  it describes a socket, not the library. */
export type SyncPhase =
  /** No phrase on this device — nothing to connect to. The state until the
   *  pairing screens exist. */
  | 'unpaired'
  /** Paired, but the app is not on screen. Deliberate (0011), not a failure. */
  | 'idle'
  | 'connecting'
  | 'online'
  /** The socket dropped; a retry is scheduled. */
  | 'retrying'
  /** The relay said no, or the stored phrase does not decode. Retrying would
   *  not help, so nothing is scheduled. */
  | 'refused';

/** The family this device belongs to. The phrase *is* the key (0042). */
export type Family = {
  phrase: string;
  /** `null` means the app's own origin, which is what production serves the
   *  socket from (0012). A value here is the development override. */
  relay: string | null;
};

function isFamily(value: unknown): value is Family {
  if (typeof value !== 'object' || value === null) return false;
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.phrase === 'string' &&
    (candidate.relay === null || typeof candidate.relay === 'string')
  );
}

/** The family, or `null` on a device that has not been paired. */
export function readFamily(): Family | null {
  const stored = localStorage.getItem(FAMILY_KEY);
  if (stored === null) return null;
  try {
    const parsed: unknown = JSON.parse(stored);
    return isFamily(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

export function rememberFamily(family: Family): void {
  localStorage.setItem(FAMILY_KEY, JSON.stringify(family));
}

/**
 * Where the socket goes: the override if there is one, otherwise this app's
 * own origin. `wss:` follows the page, so the one case that must work — an
 * installed PWA over TLS — cannot end up asking for a `ws:` the browser will
 * refuse as mixed content (DECISIONS 0044).
 */
export function relayUrl(family: Family): string {
  if (family.relay !== null && family.relay !== '') return family.relay;
  const scheme = location.protocol === 'https:' ? 'wss:' : 'ws:';
  return `${scheme}//${location.host}/sync`;
}

/** Base64, by hand: the shadow is a version vector of a few dozen bytes, and
 *  spreading a `Uint8Array` into `String.fromCharCode` is a stack overflow
 *  waiting for a bigger one. */
function encode(bytes: Uint8Array): string {
  let binary = '';
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary);
}

function decode(text: string): Uint8Array {
  const binary = atob(text);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

function sameBytes(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  return a.every((byte, i) => byte === b[i]);
}

type Progress = { cursor: SyncCursor; shadow: Uint8Array };

/**
 * Where this device got to. Zeros and an empty shadow on a device that has
 * never synced — which is also what a corrupt entry becomes, because replaying
 * from the beginning is correct and merges are idempotent (0042). Nothing here
 * is trusted enough to break a launch over.
 */
function readProgress(): Progress {
  const fresh: Progress = { cursor: { epoch: '0', since: 0 }, shadow: new Uint8Array() };
  const stored = localStorage.getItem(CURSOR_KEY);
  if (stored === null) return fresh;
  try {
    const parsed: unknown = JSON.parse(stored);
    if (typeof parsed !== 'object' || parsed === null) return fresh;
    const { epoch, since, shadow } = parsed as Record<string, unknown>;
    // The epoch is text on purpose (DECISIONS 0046) — the relay mints it from
    // 64 random bits, which is more than a JS number holds exactly. Nothing
    // here reads it; it goes back to the core exactly as it came.
    if (typeof epoch !== 'string' || !Number.isInteger(since)) return fresh;
    return {
      cursor: { epoch, since: since as number },
      shadow: typeof shadow === 'string' ? decode(shadow) : new Uint8Array(),
    };
  } catch {
    return fresh;
  }
}

export class Sync {
  readonly #core: Core;
  /** Hands a merged state to the reactive layer, which renders it and saves
   *  the replica on its usual schedule. */
  readonly #adopt: (state: StateView) => void;

  #family: Family | null = null;
  #socket: WebSocket | null = null;
  #retryTimer: ReturnType<typeof setTimeout> | undefined;
  #pushTimer: ReturnType<typeof setTimeout> | undefined;
  #cursorTimer: ReturnType<typeof setTimeout> | undefined;
  #attempt = 0;

  #cursor: SyncCursor;
  #shadow: Uint8Array;
  /** The version a push in flight would advance the shadow to, held until the
   *  relay says the frame is durable. `null` when nothing is in flight. */
  #inFlight: Uint8Array | null = null;

  /** For a settings or diagnostics screen. Not business state: it describes
   *  the socket. */
  phase = $state<SyncPhase>('unpaired');
  /** Frames opened and frames refused on the current connection. A nonzero
   *  `dropped` is either corruption or company (0042). */
  replayed = $state(0);
  dropped = $state(0);

  constructor(core: Core, adopt: (state: StateView) => void) {
    this.#core = core;
    this.#adopt = adopt;
    // A cursor is only meaningful for the replica that consumed those frames
    // (DECISIONS 0045). These are two different files on the device and can be
    // lost separately — and a cursor that outlives its replica fails silently
    // and permanently: it tells the relay "I have everything up to frame N",
    // the relay honestly replays nothing, and the library stays empty until
    // somebody else happens to push. Starting over costs one replay and is
    // always correct.
    const progress = core.openedFresh()
      ? { cursor: { epoch: '0', since: 0 }, shadow: new Uint8Array() }
      : readProgress();
    this.#cursor = progress.cursor;
    this.#shadow = progress.shadow;
  }

  /**
   * Starts listening to the page lifecycle, and connects if the app is on
   * screen and paired. Safe to call on a device with no family: it does
   * nothing until one is written.
   */
  start(): void {
    this.#family = readFamily();
    document.addEventListener('visibilitychange', () => {
      if (document.visibilityState === 'visible') this.#connect();
      else this.#park();
    });
    // The socket dies with the page anyway; closing it first means the relay
    // sees a close rather than a timeout, and this device's own `pagehide`
    // does not race the next launch's hello.
    window.addEventListener('pagehide', () => this.#park());
    // Back on the wifi at the door of the shop: reconnect now rather than at
    // the end of whatever backoff step is pending.
    window.addEventListener('online', () => {
      if (document.visibilityState === 'visible') this.#connect();
    });
    if (document.visibilityState === 'visible') this.#connect();
  }

  /**
   * Called after a command was accepted. Coalesces: a burst of taps leaves as
   * one frame, and nothing is sent at all when the socket is down — the next
   * connection pushes everything since the shadow.
   */
  localChange(): void {
    if (this.#socket === null) return;
    clearTimeout(this.#pushTimer);
    this.#pushTimer = setTimeout(() => this.#push(), PUSH_DELAY_MS);
  }

  /**
   * Pairs this device and connects. The phrase has already been validated by
   * whoever called `readPhrase` — this only stores what it is given.
   */
  pair(family: Family): void {
    rememberFamily(family);
    this.#family = family;
    // A different family means a different log: the old cursor points into
    // something this device may never see again.
    this.#cursor = { epoch: '0', since: 0 };
    this.#shadow = new Uint8Array();
    this.#rememberProgress();
    this.#connect();
  }

  // --- the socket ---------------------------------------------------------

  #connect(): void {
    if (this.#family === null) {
      this.phase = 'unpaired';
      return;
    }
    if (this.#socket !== null) return;
    clearTimeout(this.#retryTimer);

    this.phase = 'connecting';
    const socket = new WebSocket(relayUrl(this.#family));
    socket.binaryType = 'arraybuffer';
    this.#socket = socket;

    socket.onopen = () => {
      const family = this.#family;
      if (family === null) return;
      try {
        socket.send(this.#core.syncHello(family.phrase, this.#cursor));
      } catch (cause) {
        // A stored phrase that does not decode. Reconnecting would produce
        // the same failure for as long as the phone is on, so it stops here
        // and waits for someone to pair again.
        this.#refuse(cause);
      }
    };

    socket.onmessage = (message: MessageEvent<unknown>) => {
      if (!(message.data instanceof ArrayBuffer)) return;
      this.#receive(new Uint8Array(message.data));
    };

    // A refused connection and a dropped one arrive as the same pair of
    // events; only `close` is guaranteed, so the retry hangs off it alone.
    socket.onerror = () => {};
    socket.onclose = () => {
      if (this.#socket !== socket) return;
      this.#drop();
      if (this.phase === 'refused') return;
      this.#scheduleRetry();
    };
  }

  #receive(wire: Uint8Array): void {
    let event;
    try {
      event = this.#core.syncHandle(wire);
    } catch (cause) {
      // Not something this build can parse. Dropping the connection is the
      // honest answer: the alternative is applying half a conversation.
      this.#refuse(cause);
      this.#socket?.close();
      return;
    }

    switch (event.event) {
      case 'connected':
        this.phase = 'online';
        this.#attempt = 0;
        // The relay may have reset the cursor — a restored backup is a new
        // log — so it is read back rather than assumed.
        this.#readCursor();
        break;

      case 'merged':
        this.#adopt(event.state);
        this.#readCursor();
        break;

      case 'dropped':
        this.#readCursor();
        break;

      case 'caught_up':
        this.#readCursor();
        this.#push();
        break;

      case 'acked':
        if (this.#inFlight !== null) {
          this.#shadow = this.#inFlight;
          this.#inFlight = null;
          this.#rememberProgress(true);
          // Anything applied while that push was in flight is still local.
          this.#push();
        }
        break;

      case 'refused':
        this.#refuse(new Error(event.reason));
        this.#socket?.close();
        break;
    }
  }

  /** Seals and sends everything since the shadow, if there is anything and
   *  nothing is already in flight. */
  #push(): void {
    clearTimeout(this.#pushTimer);
    const socket = this.#socket;
    if (socket === null || socket.readyState !== WebSocket.OPEN) return;
    if (this.#inFlight !== null) return;

    // Read before sealing, adopt only on the ack: an edit made while the push
    // travels then stays local instead of being counted as sent.
    const version = this.#core.version();
    // Byte equality on an opaque version. If two encodings of one version ever
    // differ, this sends a delta that turns out to be empty — a wasted frame,
    // never a lost edit, which is the right way round for a guess.
    if (sameBytes(version, this.#shadow)) return;

    try {
      socket.send(this.#core.syncPush(this.#shadow));
      this.#inFlight = version;
    } catch (cause) {
      this.#refuse(cause);
      socket.close();
    }
  }

  /** Closes deliberately: the app is going away, and 0011 says the socket
   *  goes with it. No retry is scheduled — coming back is what reconnects. */
  #park(): void {
    clearTimeout(this.#retryTimer);
    clearTimeout(this.#pushTimer);
    const socket = this.#socket;
    this.#drop();
    socket?.close();
    this.#rememberProgress(true);
    if (this.phase !== 'refused' && this.phase !== 'unpaired') this.phase = 'idle';
  }

  #drop(): void {
    this.#socket = null;
    this.#inFlight = null;
    this.#core.syncClose();
  }

  #scheduleRetry(): void {
    this.phase = 'retrying';
    const step = Math.min(RETRY_MIN_MS * 2 ** this.#attempt, RETRY_MAX_MS);
    this.#attempt += 1;
    // Full jitter: two phones woken by the same relay coming back must not
    // keep colliding on every step afterwards.
    const delay = step / 2 + Math.random() * (step / 2);
    clearTimeout(this.#retryTimer);
    this.#retryTimer = setTimeout(() => {
      if (document.visibilityState === 'visible') this.#connect();
      else this.phase = 'idle';
    }, delay);
  }

  #refuse(cause: unknown): void {
    this.phase = 'refused';
    console.error('sync refused:', cause);
  }

  // --- what outlives the connection ---------------------------------------

  #readCursor(): void {
    const status = this.#core.syncStatus();
    if (status === null) return;
    this.#cursor = status.cursor;
    this.replayed = status.replayed;
    this.dropped = status.dropped;
    this.#rememberProgress();
  }

  /**
   * Writes the cursor down, coalesced — a replay of two hundred frames is one
   * write, not two hundred synchronous touches of the disk on a phone.
   * Delaying it is safe: what a lost write costs is re-merging frames already
   * held, and merges are idempotent (0042). `now` is for the moments where
   * there may be no later: an ack, and the app going away.
   */
  #rememberProgress(now = false): void {
    clearTimeout(this.#cursorTimer);
    const write = (): void => {
      localStorage.setItem(
        CURSOR_KEY,
        JSON.stringify({
          epoch: this.#cursor.epoch,
          since: this.#cursor.since,
          shadow: encode(this.#shadow),
        }),
      );
    };
    if (now) write();
    else this.#cursorTimer = setTimeout(write, PUSH_DELAY_MS);
  }
}
