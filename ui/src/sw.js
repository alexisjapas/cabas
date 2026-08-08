/**
 * The service worker: what makes the app installable, and what makes it open
 * with no signal (Rule 6, DECISIONS 0038).
 *
 * It caches **the application** and none of the data. The recipes, the list and
 * the overlay live in IndexedDB and already work offline; what is missing
 * without this file is the shell that reads them — the HTML, the JS, the CSS
 * and the 1.8 MB wasm core. That is one strategy, cache-first over a versioned
 * shell, which is why there is no Workbox here.
 *
 * `BUILD` is written by the `cabas:service-worker` plugin in `vite.config.ts`,
 * from the bundle Vite just produced. Two things follow from that, and both are
 * the point:
 *
 *   - the precache list is never hand-maintained, so a renamed chunk cannot be
 *     left out of it;
 *   - the version changes whenever the build does, so **this file's bytes
 *     change too** — which is the only signal a browser uses to decide there is
 *     a new service worker at all. A hand-written constant that someone forgets
 *     to bump serves the old app forever, and on an installed iOS PWA that is
 *     indistinguishable from the app being broken.
 *
 * Written as plain JS, not TypeScript: a service worker's globals come from
 * `lib.webworker`, which cannot share a program with the `lib.dom` every other
 * file here needs. `tsconfig.sw.json` type-checks it on its own, and `pnpm
 * check` runs that too.
 */

/** @type {{ version: string, shell: string[] }} */
const BUILD = JSON.parse('__CABAS_BUILD__');

/**
 * One cache per build. Everything precached and everything picked up at runtime
 * shares it, so a new version drops the whole previous app in one delete and
 * nothing survives into a build that did not put it there.
 */
const CACHE = `cabas-${BUILD.version}`;

/**
 * The worker's own global. `self` is typed as a plain `WorkerGlobalScope`,
 * which has neither `skipWaiting` nor `clients`.
 *
 * @type {ServiceWorkerGlobalScope}
 */
const worker = /** @type {any} */ (self);

worker.addEventListener('install', (event) => {
  event.waitUntil(
    (async () => {
      const cache = await caches.open(CACHE);
      // `cache: 'reload'` bypasses the HTTP cache. Every asset but the shell
      // carries a content hash in its name and could not be stale, but `/` is
      // `index.html` under a stable URL — precaching a copy the browser had
      // lying around would install the previous build's markup next to this
      // build's assets.
      await cache.addAll(BUILD.shell.map((url) => new Request(url, { cache: 'reload' })));
    })(),
  );

  // Deliberately no `skipWaiting()`. A new build takes over at the next cold
  // start, once no page is running on the old one — because activating early
  // deletes the caches the running page is still loading from, and offline
  // there is nowhere else for it to get them. iOS kills a backgrounded PWA
  // often enough that "the next cold start" is soon.
});

worker.addEventListener('activate', (event) => {
  event.waitUntil(
    (async () => {
      for (const name of await caches.keys()) {
        if (name !== CACHE) await caches.delete(name);
      }
      // On a first install there is no page to wait for, and claiming means the
      // tab that just registered this worker is already covered — it does not
      // take a reload to become offline-capable.
      await worker.clients.claim();
    })(),
  );
});

worker.addEventListener('fetch', (event) => {
  const request = event.request;

  // Anything not answered here falls through to the network untouched. A
  // command never travels over HTTP (it goes through wasm into IndexedDB), so
  // in practice this is the relay at M5, and it is not ours to cache.
  if (request.method !== 'GET') return;
  if (new URL(request.url).origin !== worker.location.origin) return;

  event.respondWith(request.mode === 'navigate' ? shell() : cacheFirst(event));
});

/**
 * Read a precached entry.
 *
 * `ignoreVary` is load-bearing, and its absence is the kind of bug that only
 * appears with the network off. A server that answers `Vary: Origin` — Vite's
 * preview does, and the relay may — makes the cache match on the request's
 * `Origin` header too. The precache is filled by this worker, whose requests
 * carry no `Origin`; the page then asks for its own JS and CSS with one,
 * because Vite marks both tags `crossorigin`. Every asset is in the cache and
 * every lookup misses. Online that is invisible, since the miss falls through
 * to a network that answers; offline it is a blank page.
 *
 * Ignoring `Vary` is not a shortcut here: every URL in this app has exactly one
 * representation, and the ones that matter carry a content hash in the name.
 *
 * @param {Cache} cache
 * @param {RequestInfo} request
 * @returns {Promise<Response | undefined>}
 */
function lookup(cache, request) {
  return cache.match(request, { ignoreVary: true });
}

/**
 * Every navigation renders the same single page (DECISIONS 0037): which screen
 * is open is core state, not a URL.
 *
 * @returns {Promise<Response>}
 */
async function shell() {
  const cache = await caches.open(CACHE);
  const cached = await lookup(cache, '/');
  return cached ?? fetch('/');
}

/**
 * Cache first, then network — and keep what the network gave.
 *
 * The precache covers everything the bundle produced; what reaches here is the
 * handful of files copied verbatim out of `public/`, whose names carry no hash:
 * the manifest, the icons, the favicon. Keeping them in the versioned cache is
 * what stops them being stale forever — the whole bucket is dropped on the next
 * build, so the copy is never older than the app around it.
 *
 * @param {FetchEvent} event
 * @returns {Promise<Response>}
 */
async function cacheFirst(event) {
  const cache = await caches.open(CACHE);
  const cached = await lookup(cache, event.request);
  if (cached) return cached;

  const response = await fetch(event.request);
  // `basic` means same-origin and fully readable; an opaque response has a
  // status of 0 and caching one stores a failure that looks like a success.
  if (response.ok && response.type === 'basic') {
    // Not awaited: `cache.put` reads the clone to completion, and awaiting it
    // would hold the real response back until the copy is written.
    event.waitUntil(cache.put(event.request, response.clone()));
  }
  return response;
}
