/**
 * The photo engine: a second socket, and the policy around it.
 *
 * `sync.svelte.ts` is this file's counterpart for the document, and the two
 * are deliberately separate objects over separate sockets (DECISIONS 0080,
 * 0092). They share the group and the origin and nothing else: no cursor, no
 * epoch, no sequence number, and above all no queue — a photo is hundreds of
 * kilobytes and a list edit is a hundred bytes, so putting them on one socket
 * would mean a tick in a shop waiting behind a picture of a jar (Rule 6).
 *
 * # The whole conversation
 *
 * ```text
 * socket opens   → photoHello(phrase)          → send
 * every message  → photoHandle(wire)           → render what it says
 * whenever idle  → photoFetch() / photoPush()  → send, one at a time
 * status.done    → close
 * ```
 *
 * There is nothing to persist between connections. The next hello re-derives
 * the work from what is actually on disk at both ends, which is why a transfer
 * cut short costs one round trip and never a lost photo.
 *
 * # Why every core call is chained
 *
 * `photoHandle` and `photoPush` take the session *out* of the core's cell
 * while they await a browser transaction (DECISIONS 0032), so two of them in
 * flight at once would find it empty. Messages arrive whenever the relay sends
 * them, so the chain below is what makes "one at a time" true rather than
 * merely likely.
 *
 * # Everything here is opaque
 *
 * Every `Uint8Array` is a sealed photo or a wire message. This file never
 * looks inside one, and never sees a picture: a fetched photo is opened and
 * written to this device's store inside the core, and what comes back out is
 * an event saying which id landed.
 */

import type { PhotoEvent } from './bindings/PhotoEvent';
import type { Core } from './core';
import type { Group } from './sync.svelte';

/**
 * Where the photo socket goes.
 *
 * The same rule as `relayUrl` — the override if there is one, otherwise this
 * app's own origin over the page's own scheme (DECISIONS 0044) — with the path
 * swapped. The override is written in Settings as a `/sync` address because
 * that is the one somebody has to type; taking the `/sync` off it is what
 * keeps a development relay reachable without a second field to fill in and
 * get wrong.
 */
export function photoUrl(group: Group, fallback: string | null): string {
  // The Settings override and the host's own default (DECISIONS 0095) are both
  // `/sync` URLs, so both are read the same way.
  const named = group.relay !== null && group.relay !== '' ? group.relay : fallback;
  if (named !== null && named !== '') {
    return `${named.replace(/\/sync\/?$/, '')}/photos`;
  }
  const scheme = location.protocol === 'https:' ? 'wss:' : 'ws:';
  return `${scheme}//${location.host}/photos`;
}

/**
 * First backoff step, then doubling, capped. Longer at both ends than the
 * document's: nothing on screen is waiting for a photo — a placeholder is a
 * designed state, not a failure — and a phone in a shop should spend its
 * radio on the list.
 */
const RETRY_MIN_MS = 5_000;
const RETRY_MAX_MS = 120_000;

/** What a settings screen may show. It describes a socket, not the library. */
export type PhotoPhase =
  /** No group, or nothing has asked for a transfer yet. */
  | 'idle'
  | 'connecting'
  /** Reconciled, and moving photos. */
  | 'running'
  /** The queues drained. Everything this device can have, it has. */
  | 'done'
  /** The socket dropped mid-transfer; a retry is scheduled. */
  | 'retrying'
  /** The relay said no. Retrying without changing something would not help. */
  | 'refused';

export class PhotoTransfer {
  readonly #core: Core;
  /** Where to fetch photos when the group carries no override. */
  readonly #defaultRelay: string | null;
  /** Read rather than held: `Sync` owns the group and it can change. */
  readonly #group: () => Group | null;

  #socket: WebSocket | null = null;
  /**
   * The phrase the open socket was opened with. Rotating the group (0024)
   * makes an in-flight transfer a conversation with the *previous* group —
   * sealed with a key nothing will ask for again, stored under an id nothing
   * will name again. Harmless and pointless, so a nudge that finds them
   * different drops the socket rather than waiting for it to drain.
   */
  #openedWith: string | null = null;
  /** Serialises the core calls; see the module note. */
  #chain: Promise<void> = Promise.resolve();
  #retryTimer: ReturnType<typeof setTimeout> | undefined;
  #attempt = 0;
  /**
   * Something changed while a transfer was running, so the work this
   * connection reconciled is already out of date. One flag rather than a
   * queue: the next hello re-derives everything, so two nudges and ten are
   * the same instruction.
   */
  #again = false;

  phase = $state<PhotoPhase>('idle');
  /** Photos still to move on this connection — what a progress line counts. */
  pending = $state(0);
  /** Landed on this device, ever, this session. */
  received = $state(0);
  /** Made durable on the relay, ever, this session. */
  sent = $state(0);

  /**
   * Bumped every time a photo lands.
   *
   * `Photo.svelte` reads it, which is the whole mechanism by which a
   * placeholder becomes a picture without anything else being told: the
   * component's effect depends on this number, so incrementing it re-reads
   * every photo on screen. Cheap, because the ones already held come back
   * from IndexedDB and the ones that are not draw nothing either way.
   */
  generation = $state(0);

  constructor(core: Core, group: () => Group | null, defaultRelay: string | null) {
    this.#core = core;
    this.#group = group;
    this.#defaultRelay = defaultRelay;
  }

  /**
   * Starts listening to the page lifecycle. Safe on a device with no group:
   * nothing happens until one is written and something asks for a transfer.
   */
  start(): void {
    document.addEventListener('visibilitychange', () => {
      if (document.visibilityState === 'visible') this.nudge();
      else this.#park();
    });
    window.addEventListener('pagehide', () => this.#park());
    if (document.visibilityState === 'visible') this.nudge();
  }

  /**
   * "There may be photos to move."
   *
   * Called after a photo is taken here, and after the document catches up —
   * which is when this device learns the *names* of photos the other phone
   * took. Both are cheap to over-call: a connection with nothing to do is a
   * hello and a welcome.
   */
  nudge(): void {
    if (document.visibilityState !== 'visible') return;
    if (this.#socket !== null) {
      if (this.#openedWith !== (this.#group()?.phrase ?? null)) {
        // Not this group any more — see `#openedWith`.
        const stale = this.#socket;
        this.#drop();
        stale.close();
        this.#connect();
        return;
      }
      // A connection is already reconciling or working. Its lists were
      // computed before this nudge, so it will have to run again afterwards.
      this.#again = true;
      return;
    }
    this.#connect();
  }

  // --- the socket -----------------------------------------------------------

  #connect(): void {
    const group = this.#group();
    if (group === null) {
      this.phase = 'idle';
      return;
    }
    if (this.#socket !== null) return;
    clearTimeout(this.#retryTimer);
    this.#again = false;

    this.phase = 'connecting';
    const socket = new WebSocket(photoUrl(group, this.#defaultRelay));
    socket.binaryType = 'arraybuffer';
    this.#socket = socket;
    this.#openedWith = group.phrase;

    socket.onopen = () => {
      this.#run(async () => {
        if (this.#socket !== socket) return;
        try {
          // Asynchronous, and the socket may go while it runs: the hello is
          // read off this device's store and its replica.
          const hello = await this.#core.photoHello(group.phrase);
          if (this.#socket !== socket || socket.readyState !== WebSocket.OPEN) return;
          socket.send(hello);
        } catch (cause) {
          // A stored phrase that does not decode, or storage that will not
          // answer. Reconnecting would fail the same way for as long as the
          // phone is on.
          this.#refuse(cause);
          socket.close();
        }
      });
    };

    socket.onmessage = (message: MessageEvent<unknown>) => {
      if (!(message.data instanceof ArrayBuffer)) return;
      const wire = new Uint8Array(message.data);
      this.#run(() => this.#receive(socket, wire));
    };

    socket.onerror = () => {};
    socket.onclose = () => {
      if (this.#socket !== socket) return;
      this.#drop();
      if (this.phase === 'refused') return;
      // Finished is finished: the queues drained and the socket was closed on
      // purpose. Anything else is a drop, and a drop mid-transfer is worth
      // one more go.
      if (this.phase === 'done') {
        if (this.#again) this.nudge();
        return;
      }
      this.#scheduleRetry();
    };
  }

  async #receive(socket: WebSocket, wire: Uint8Array): Promise<void> {
    if (this.#socket !== socket) return;
    let event: PhotoEvent;
    try {
      event = await this.#core.photoHandle(wire);
    } catch (cause) {
      // Not something this build can parse, or a store that refused to write.
      this.#refuse(cause);
      socket.close();
      return;
    }
    if (this.#socket !== socket) return;

    switch (event.event) {
      case 'reconciled':
        this.phase = 'running';
        this.#attempt = 0;
        break;
      case 'received':
        // The bytes are on this device now, so every placeholder on screen
        // can become a picture.
        this.generation += 1;
        break;
      case 'refused':
        this.#refuse(new Error(event.reason));
        socket.close();
        return;
      // `sent`, `absent`, `rejected` and `dropped` all mean the same thing to
      // this side: one fewer thing outstanding. The counters below say what
      // happened, and `Réglages` is where they are read.
      default:
        break;
    }

    await this.#readStatus();
    await this.#pump(socket);
  }

  /**
   * Sends the next request, or closes the socket when there is nothing left.
   *
   * One message in flight at a time, deliberately: the pace is this device's,
   * and a relay answering a welcome by streaming would hand a phone that has
   * just joined the whole library at once (DECISIONS 0080).
   */
  async #pump(socket: WebSocket): Promise<void> {
    if (this.#socket !== socket || socket.readyState !== WebSocket.OPEN) return;

    const status = await this.#core.photoStatus();
    if (this.#socket !== socket) return;
    if (status === null) return;
    if (status.done) {
      // Ours to decide and ours to act on: the relay cannot know what this
      // device still wants.
      this.phase = 'done';
      socket.close();
      return;
    }

    try {
      const fetch = await this.#core.photoFetch();
      if (this.#socket !== socket || socket.readyState !== WebSocket.OPEN) return;
      if (fetch !== undefined) {
        socket.send(fetch);
        return;
      }
      const push = await this.#core.photoPush();
      if (this.#socket !== socket || socket.readyState !== WebSocket.OPEN) return;
      if (push !== undefined) socket.send(push);
      // Neither, and not done: an answer is outstanding, so the next message
      // is what moves this on.
    } catch (cause) {
      this.#refuse(cause);
      socket.close();
    }
  }

  /** Closes deliberately: the app is going away (DECISIONS 0011). */
  #park(): void {
    clearTimeout(this.#retryTimer);
    const socket = this.#socket;
    this.#drop();
    socket?.close();
    if (this.phase !== 'refused' && this.phase !== 'done') this.phase = 'idle';
  }

  #drop(): void {
    this.#socket = null;
    this.#openedWith = null;
    this.pending = 0;
    // On the chain like every other core call, but not through `#run`: a
    // failure to let go of a session that is already gone is worth a line in
    // the console and is not worth refusing the next transfer over.
    this.#chain = this.#chain
      .then(() => this.#core.photoClose())
      .catch((cause: unknown) => {
        console.error('photo close failed:', cause);
      });
  }

  #scheduleRetry(): void {
    this.phase = 'retrying';
    const step = Math.min(RETRY_MIN_MS * 2 ** this.#attempt, RETRY_MAX_MS);
    this.#attempt += 1;
    // Full jitter, for the reason the document's engine has it: two phones
    // woken by the same relay coming back must not keep colliding.
    const delay = step / 2 + Math.random() * (step / 2);
    clearTimeout(this.#retryTimer);
    this.#retryTimer = setTimeout(() => {
      if (document.visibilityState === 'visible') this.#connect();
      else this.phase = 'idle';
    }, delay);
  }

  #refuse(cause: unknown): void {
    this.phase = 'refused';
    console.error('photo transfer refused:', cause);
  }

  async #readStatus(): Promise<void> {
    const status = await this.#core.photoStatus();
    if (status === null) return;
    this.pending = status.pending;
    this.received = status.received;
    this.sent = status.sent;
  }

  /** Every core call, one at a time. See the module note. */
  #run(work: () => Promise<void>): void {
    this.#chain = this.#chain.then(work).catch((cause: unknown) => {
      this.#refuse(cause);
      this.#socket?.close();
    });
  }
}
