/**
 * The whole vertical, in a real browser: mint an identity, build a library,
 * put something on the list, derive the cart, tick a line, reload from
 * IndexedDB, and open the whole thing again with the network turned off.
 *
 * This is the frontend's counterpart to `crates/app/tests/scenario.rs`, and it
 * exists for the same reason `wasm-test` does — everything it covers is
 * invisible outside a browser. A missing `getrandom` backend, a `null` that
 * arrives as `undefined`, an IndexedDB write that never lands: all of them
 * look like a blank page on a phone and like nothing at all in a unit test.
 *
 * Driven over the DevTools protocol with no dependencies: Node has `fetch` and
 * `WebSocket` built in, and a test harness that needs its own npm tree is a
 * second lockfile to keep honest.
 *
 * Run it with `ui-test` in the browser shell — it expects a relay with the
 * bundle compiled into it on 8788 and chromium listening on 9222. The app and
 * the sync socket therefore share an origin here exactly as they do in
 * production (DECISIONS 0048), which is the only arrangement in which the
 * service worker, the cache headers and the WebSocket are all the real ones.
 */

import { execFileSync } from 'node:child_process';
import { readFile, readdir, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const DEVTOOLS = process.env.DEVTOOLS_URL ?? 'http://localhost:9222';
const APP = process.env.APP_URL ?? 'http://127.0.0.1:8788';
const SHOTS = process.env.SCREENSHOT_DIR ?? null;

/** The relay `ui-test` started, and the directory it writes its sealed log to.
 *  Both come from the script; running this file by hand is not supported, and
 *  a sync test that quietly skipped itself would be worse than none. */
const RELAY_DATA = process.env.CABAS_RELAY_DATA ?? null;
const RELAY_URL = process.env.CABAS_RELAY_URL ?? null;
if (RELAY_DATA === null || RELAY_URL === null) {
  throw new Error('no relay: CABAS_RELAY_DATA and CABAS_RELAY_URL come from ui-test');
}

const targets = await (await fetch(`${DEVTOOLS}/json/list`)).json();
const target = targets.find((candidate) => candidate.type === 'page');
if (!target) throw new Error('no page target — is chromium running with --remote-debugging-port?');

const socket = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  socket.onopen = resolve;
  socket.onerror = reject;
});

let nextId = 0;
const pending = new Map();
const consoleErrors = [];

socket.onmessage = (event) => {
  const message = JSON.parse(event.data);
  const waiter = pending.get(message.id);
  if (waiter) {
    pending.delete(message.id);
    if (message.error) waiter.reject(new Error(JSON.stringify(message.error)));
    else waiter.resolve(message.result);
    return;
  }
  // Anything the page logs as an error fails the run at the end. A screen that
  // renders correctly while throwing in the console is a bug with a fuse on it.
  if (message.method === 'Log.entryAdded' && message.params.entry.level === 'error') {
    const entry = message.params.entry;
    // A sync socket that cannot reach its relay is an event, not a defect:
    // this suite turns the network off on purpose, and the engine is meant to
    // retry rather than to give up quietly. What it is *not* allowed to do is
    // log something itself — a `console.error` from the app still fails the
    // run, and the connection is asserted where it matters, by watching data
    // arrive at the other end.
    if (entry.source === 'network' && /WebSocket connection to/.test(entry.text ?? '')) return;
    consoleErrors.push(`[${entry.source}] ${entry.text || '(no text)'} ${entry.url ?? ''}`.trim());
  }
  // `Log.entryAdded` carries what the *browser* complains about — a failed
  // request, a bad manifest icon. What the *app* logs comes through here and
  // nowhere else, so without this an engine that gives up with a
  // `console.error` fails no test at all.
  if (message.method === 'Runtime.consoleAPICalled' && message.params.type === 'error') {
    const text = message.params.args
      .map((arg) => arg.value ?? arg.description ?? arg.unserializableValue ?? '')
      .join(' ')
      .trim();
    consoleErrors.push(`[console] ${text || '(no text)'}`);
  }
  if (message.method === 'Runtime.exceptionThrown') {
    consoleErrors.push(message.params.exceptionDetails.text ?? 'uncaught exception');
  }
};

/**
 * `sessionId` addresses a target other than the page — the service worker,
 * which is its own target and does not hear anything sent to the page.
 */
function send(method, params = {}, sessionId = undefined) {
  const id = ++nextId;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    socket.send(JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) }));
  });
}

async function evaluate(expression) {
  const outcome = await send('Runtime.evaluate', {
    expression,
    awaitPromise: true,
    returnByValue: true,
  });
  if (outcome.exceptionDetails) {
    const detail = outcome.exceptionDetails.exception?.description ?? outcome.exceptionDetails.text;
    throw new Error(`evaluating ${expression}\n  → ${detail}`);
  }
  return outcome.result.value;
}

async function waitFor(expression, label, timeoutMs = 15000) {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    if (await evaluate(`!!(${expression})`)) return;
    if (Date.now() > deadline) throw failed(`timed out waiting for ${label}`);
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
}

/**
 * Svelte listens for real events, so assigning `.value` changes the pixel and
 * nothing else. These go through the native setter and then dispatch, which is
 * what a keystroke does.
 */
const HELPERS = `
  window.__setNode = (el, value) => {
    const proto =
      el instanceof HTMLSelectElement ? HTMLSelectElement.prototype
      : el instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype
      : HTMLInputElement.prototype;
    Object.getOwnPropertyDescriptor(proto, 'value').set.call(el, value);
    el.dispatchEvent(new Event('input', { bubbles: true }));
    el.dispatchEvent(new Event('change', { bubbles: true }));
  };
  window.__set = (selector, value) => {
    const el = document.querySelector(selector);
    if (!el) throw new Error('missing ' + selector);
    __setNode(el, value);
  };
  /** The nth match, because a form repeats a field once per line. */
  window.__setNth = (selector, n, value) => {
    const el = document.querySelectorAll(selector)[n];
    if (!el) throw new Error('missing ' + selector + ' #' + n);
    __setNode(el, value);
  };
  /** Chooses an option by the words on it: the value is an opaque id. */
  window.__pick = (selector, n, text) => {
    const el = document.querySelectorAll(selector)[n];
    if (!el) throw new Error('missing ' + selector + ' #' + n);
    const option = [...el.options].find((o) => o.textContent.trim() === text);
    if (!option) throw new Error(text + ' missing from ' + selector + ' #' + n);
    __setNode(el, option.value);
  };
  /**
   * A soft keyboard, in the shape iOS makes: the layout viewport keeps every
   * pixel of its height and the visual viewport loses \`px\` off the bottom.
   *
   * There is no CDP command for this. \`setDeviceMetricsOverride\` resizes the
   * layout viewport, which is the one thing a keyboard never does — emulating
   * it that way would test a page that had been told the truth. Overriding the
   * accessor is what leaves the page believing it is still 640 px tall while
   * the API that knows better says otherwise, which is the whole problem.
   */
  window.__keyboard = (px) => {
    const proto = Object.getPrototypeOf(window.visualViewport);
    window.__vvHeight ??= Object.getOwnPropertyDescriptor(proto, 'height');
    const real = window.__vvHeight.get;
    Object.defineProperty(
      proto,
      'height',
      px === 0
        ? window.__vvHeight
        : { configurable: true, get() { return real.call(this) - px; } },
    );
    window.visualViewport.dispatchEvent(new Event('resize'));
  };
  window.__count = (selector) => document.querySelectorAll(selector).length;
  window.__click = (selector) => {
    const el = document.querySelector(selector);
    if (!el) throw new Error('missing ' + selector);
    el.click();
  };
  window.__clickText = (selector, text) => {
    const el = [...document.querySelectorAll(selector)].find((n) => n.textContent.trim().includes(text));
    if (!el) throw new Error('no ' + selector + ' containing ' + text);
    el.click();
  };
  window.__text = (selector) => document.querySelector(selector)?.textContent.trim() ?? null;
  window.__all = (selector) => [...document.querySelectorAll(selector)].map((n) => n.textContent.trim());
  true;
`;

async function load(url) {
  await send('Page.navigate', { url });
  await waitFor('document.readyState === "complete"', 'document ready');
  await evaluate(HELPERS);
  // Network emulation does not survive a navigation: the new document comes up
  // online however the old one was left. Re-applying here is what makes
  // "offline" mean offline for the reload, which is the only load that matters.
  if (offline) await setOffline(true);
}

/** Whether the network is meant to be off, and the worker sessions holding it off. */
let offline = false;
const workers = new Set();

/**
 * Offline, everywhere it has to be.
 *
 * Two traps, both of which make an offline test quietly prove nothing.
 * `Network.emulateNetworkConditions` is scoped to a single target, and a
 * service worker is a target of its own — so a page put offline on its own
 * still has a worker behind it that can reach the network on a cache miss. And
 * the emulation is per-document, so it has to be re-applied after every load.
 *
 * @returns the number of worker targets it is holding offline; zero means the
 * page is the only thing that was switched off, which would prove nothing.
 */
async function setOffline(value) {
  offline = value;

  const { targetInfos } = await send('Target.getTargets');
  for (const target of targetInfos) {
    if (target.type !== 'service_worker' || target.attached) continue;
    const { sessionId } = await send('Target.attachToTarget', {
      targetId: target.targetId,
      flatten: true,
    });
    workers.add(sessionId);
  }

  for (const session of [undefined, ...workers]) {
    await send('Network.enable', {}, session);
    await send(
      'Network.emulateNetworkConditions',
      {
        offline: value,
        latency: 0,
        downloadThroughput: value ? 0 : -1,
        uploadThroughput: value ? 0 : -1,
      },
      session,
    );
  }
  return workers.size;
}

async function shot(name) {
  if (SHOTS === null) return;
  const { data } = await send('Page.captureScreenshot', { format: 'png' });
  await writeFile(`${SHOTS}/${name}.png`, Buffer.from(data, 'base64'));
}

function ok(label) {
  console.log(`  ok  ${label}`);
}

/**
 * A failure, with what the page was complaining about while it happened.
 *
 * Without this a timeout is a bare "waited for X", and the console entry that
 * explains it — the exception that stopped the engine, the request that 404ed
 * — is collected here and then thrown away with the process.
 */
function failed(what) {
  const logged = consoleErrors.length > 0 ? `\n  the page had logged:\n    ${consoleErrors.join('\n    ')}` : '';
  return new Error(`${what}${logged}`);
}

await send('Runtime.enable');
await send('Log.enable');
await send('Page.enable');

// --- a device that has never run ------------------------------------------

await load(APP);
await evaluate(`
  (async () => {
    localStorage.clear();
    for (const db of await indexedDB.databases()) {
      await new Promise((resolve) => {
        const request = indexedDB.deleteDatabase(db.name);
        request.onsuccess = request.onerror = request.onblocked = resolve;
      });
    }
    return true;
  })()
`);

await load(APP);
await waitFor(`__text('h1') === 'cabas'`, 'the pairing screen');
ok('a device with no identity is asked about its family first');

// Start a family rather than join one: this is the first phone, and the
// phrase it mints is the whole secret from here on (DECISIONS 0021, 0042).
await evaluate(`__clickText('button', 'Commencer une famille')`);
await waitFor(`__text('[data-phrase]')`, 'the phrase');
const phrase = await evaluate(`__text('[data-phrase]')`);
if (phrase.split(/\s+/).length !== 12) {
  throw failed(`the phrase is not twelve words: ${JSON.stringify(phrase)}`);
}
ok(`the family's phrase is twelve words`);

// The encoder against qrencode — fixed phrases plus the one just minted —
// and then the drawing against the encoder. A picture that renders and does
// not scan is exactly the failure a hand-written encoder invites, and the two
// halves fail for different reasons (DECISIONS 0047).
const qr = (...args) =>
  execFileSync('node', ['--experimental-strip-types', 'ui/tools/qr-check.mts', ...args], {
    encoding: 'utf8',
  });
console.log(qr('--verify', phrase).trimEnd());

const drawn = await evaluate(`
  JSON.stringify([...document.querySelectorAll('.qr .module')].map(
    (rect) => [+rect.getAttribute('x'), +rect.getAttribute('y'), +rect.getAttribute('width')]
  ))
`);
const rendered = new Set();
for (const [x, y, width] of JSON.parse(drawn)) {
  for (let i = 0; i < width; i += 1) rendered.add(`${y},${x + i}`);
}
const expected = new Set();
qr('--print', phrase)
  .split('\n')
  .filter((row) => row.length > 0)
  .forEach((row, y) => {
    for (let x = 0; x < row.length; x += 1) if (row[x] === '1') expected.add(`${y},${x}`);
  });
if (rendered.size !== expected.size || [...expected].some((cell) => !rendered.has(cell))) {
  throw failed(
    `the drawing is not the symbol: ${rendered.size} modules against ${expected.size} — ` +
      'the run-merging in Qr.svelte is the suspect',
  );
}
ok(`the page draws the symbol it was given (${expected.size} dark modules)`);
await shot('00-pairing');

await evaluate(`__clickText('button', "J'ai noté la phrase")`);
await waitFor('document.querySelector("form input")', 'onboarding form');
ok('and then it asks who this device belongs to');

await evaluate(`__set('input[autocomplete="given-name"]', 'Alexis')`);
await evaluate(`__set('form label:nth-of-type(2) input', 'iPhone de test')`);
await evaluate(`__click('button[type="submit"]')`);
await waitFor('document.querySelector("nav")', 'tab bar');
ok('identity minted and the replica opened');

const identity = await evaluate(`JSON.parse(localStorage.getItem('cabas.identity'))`);
if (!identity?.user?.startsWith('usr_') || !identity?.device?.startsWith('dev_')) {
  throw new Error(`identity looks wrong: ${JSON.stringify(identity)}`);
}
ok('identity persisted to localStorage (DECISIONS 0031)');

// Point this device at the relay before anything is built, the way a real one
// is pointed at the family's own server — everything below then syncs as it
// happens rather than in one burst at the end. In production the relay serves
// the app and this field stays empty, which is why it says so (0043, 0044).
await evaluate(`__clickText('nav button', 'Réglages')`);
await waitFor(`__text('h1') === 'Réglages'`, 'the settings screen');
await evaluate(`__set('.family input', ${JSON.stringify(RELAY_URL)})`);
await evaluate(`__clickText('.family button', 'Enregistrer le serveur')`);
await waitFor(
  `JSON.parse(localStorage.getItem('cabas.family')).relay === ${JSON.stringify(RELAY_URL)}`,
  'the relay override, stored',
);
await waitFor(`__text('.family .status') === 'Synchronisé'`, 'a live connection');
ok('the relay is set from settings, and the engine connects to it');
await shot('01-empty-cart');

// --- the library -----------------------------------------------------------

await evaluate(`__clickText('nav button', 'Ingrédients')`);
await waitFor(`__text('h1') === 'Ingrédients'`, 'ingredients screen');
await evaluate(`__clickText('button', 'Nouveau')`);
await waitFor('document.querySelector("form")', 'ingredient form');
await evaluate(`__set('form label:nth-of-type(1) input', 'Tomates')`);
await evaluate(`__set('form select', 'produce')`);
await evaluate(`__click('button[type="submit"]')`);
await waitFor(`__all('li .name').includes('Tomates')`, 'Tomates in the library');
ok('an ingredient is created and listed');

await evaluate(`__clickText('button', 'Nouveau')`);
await waitFor('document.querySelector("form")', 'ingredient form');
await evaluate(`__set('form label:nth-of-type(1) input', 'Sel')`);
await evaluate(`__click('form input[type="checkbox"]')`);
await evaluate(`__click('button[type="submit"]')`);
await waitFor(`__all('li .name').includes('Sel')`, 'Sel in the library');
ok('a staple is created');
await shot('02-library');

// --- the list --------------------------------------------------------------

await evaluate(`__clickText('nav button', 'Liste')`);
await waitFor(`__text('h1') === 'Liste'`, 'list screen');
await evaluate(`__clickText('button', 'Ajouter')`);
await waitFor('document.querySelector("form select")', 'add form');

await evaluate(`
  (() => {
    const select = document.querySelector('form select');
    const option = [...select.options].find((o) => o.textContent.trim() === 'Tomates');
    if (!option) throw new Error('Tomates missing from the picker');
    __set('form select', option.value);
  })()
`);
await evaluate(`__set('form input[inputmode="decimal"]', '3')`);
await evaluate(`
  (() => {
    const unit = document.querySelectorAll('form select')[1];
    Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value').set.call(unit, 'piece');
    unit.dispatchEvent(new Event('input', { bubbles: true }));
    unit.dispatchEvent(new Event('change', { bubbles: true }));
  })()
`);
await evaluate(`__clickText('button', 'Ajouter à la liste')`);

const refusal = await evaluate(`__text('[role="alert"]')`);
if (refusal !== null) throw new Error(`the command was refused: ${refusal}`);
await waitFor(`__all('li .name').includes('Tomates')`, 'Tomates on the list');
ok('an ingredient goes onto the list');
await shot('03-list');

// --- the cart derives ------------------------------------------------------

await evaluate(`__clickText('nav button', 'Courses')`);
await waitFor(`__text('h1') === 'Courses'`, 'cart screen');
await waitFor(`__all('li .name').includes('Tomates')`, 'Tomates in the cart');

const aisles = await evaluate(`__all('section h2')`);
if (!aisles.includes('Fruits et légumes')) {
  throw new Error(`expected an aisle heading, got ${JSON.stringify(aisles)}`);
}
ok(`the cart derives and groups by aisle (${JSON.stringify(aisles)})`);
await shot('04-cart');

await evaluate(`__clickText('section li button', 'Tomates')`);
await waitFor(`__text('details summary')?.startsWith('Acheté')`, 'the bought section');
ok('ticking a line moves it to "Acheté"');
await shot('05-ticked');

// --- and none of it was only in memory -------------------------------------

// Long enough for the debounced flush to have written (Session.FLUSH_DELAY_MS).
await new Promise((resolve) => setTimeout(resolve, 1200));
await load(APP);
await waitFor('document.querySelector("nav")', 'tab bar after reload');
await waitFor(`__text('h1') === 'Courses'`, 'the screen that was open');
await waitFor(`__text('details summary')?.startsWith('Acheté')`, 'the ticked line');
ok('everything survived a cold reload, on the screen it was left on');
await shot('06-reloaded');

// --- a recipe, written in one pass ------------------------------------------
//
// The editor's whole premise: lines and the prose that mentions them go out in
// one `SaveRecipe`, because a line carries its id from the moment it is added
// (DECISIONS 0039). If that were not so, the mention below would save as a
// reference to nothing and read back as "ligne supprimée".

await evaluate(`__clickText('nav button', 'Recettes')`);
await waitFor(`__text('h1') === 'Recettes'`, 'the recipes screen');
await evaluate(`__clickText('button', 'Nouvelle')`);
await waitFor(`__text('h1') === 'Nouvelle recette'`, 'the recipe editor');

await evaluate(`__set('input[placeholder="Tarte aux tomates"]', 'Salade de tomates')`);

await evaluate(`__clickText('.adders button', '+ Ingrédient')`);
await waitFor(`__count('select[aria-label="Ingrédient"]') === 1`, 'the first line');
await evaluate(`__pick('select[aria-label="Ingrédient"]', 0, 'Tomates')`);
await evaluate(`__setNth('input[aria-label="Quantité"]', 0, '2')`);
await evaluate(`__setNth('select[aria-label="Unité"]', 0, 'piece')`);

await evaluate(`__clickText('.adders button', '+ Ingrédient')`);
await waitFor(`__count('select[aria-label="Ingrédient"]') === 2`, 'the second line');
await evaluate(`__pick('select[aria-label="Ingrédient"]', 1, 'Sel')`);
await evaluate(`__setNth('input[aria-label="Quantité"]', 1, '5')`);
await evaluate(`__setNth('select[aria-label="Unité"]', 1, 'g')`);
ok('two ingredient lines, each named before the recipe exists');

// Typing "@tom" offers the recipe's own tomato line, and picking it splits the
// prose around a reference — never a marker inside a string (DECISIONS 0022).
await evaluate(`__clickText('.adders button', '+ Étape')`);
await waitFor(`__count('textarea') === 1`, 'a step to write in');
await evaluate(`__set('textarea', 'Couper les @tom')`);
await waitFor(`__count('.picker button') === 1`, 'the mention picker, scoped to this recipe');
await evaluate(`__clickText('.picker button', 'Tomates')`);
await waitFor(`__count('.chip') === 1`, 'the mention became a chip');
await evaluate(`__setNth('textarea', 1, ' en quartiers.')`);
ok('an @ mention resolves against a line that has never been saved');
await shot('07-editor');

await evaluate(`__clickText('button', 'Enregistrer')`);
await waitFor(`__text('h1') === 'Salade de tomates'`, 'the recipe opens after saving');

const prose = await evaluate(`__text('.prose')`);
if (!prose.includes('Couper les') || !prose.includes('Tomates') || !prose.includes('quartiers')) {
  throw new Error(`the step did not render its reference: ${JSON.stringify(prose)}`);
}
if (prose.includes('supprimée')) {
  throw new Error(`the mention dangled: ${JSON.stringify(prose)}`);
}
ok(`the step reads back with its reference resolved (${JSON.stringify(prose)})`);

// Read at eight what was written for four: every quantity doubles, in the
// ingredient list and inside the step, and none of it is computed here.
await evaluate(`__click('button[aria-label="Plus"]')`);
await evaluate(`__click('button[aria-label="Plus"]')`);
await evaluate(`__click('button[aria-label="Plus"]')`);
await evaluate(`__click('button[aria-label="Plus"]')`);
await waitFor(`__text('.count') === '8 pers.'`, 'the recipe read at eight');

const scaled = await evaluate(`__all('.components .amount')`);
if (!scaled.includes('4') || !scaled.includes('10 g')) {
  throw new Error(`expected doubled quantities, got ${JSON.stringify(scaled)}`);
}
ok(`scaling is the core's arithmetic, not the frontend's (${JSON.stringify(scaled)})`);
await shot('08-recipe');

await evaluate(`__clickText('button', 'Ajouter à la liste')`);
await waitFor(`__text('.primary') === 'Ajoutée à la liste'`, 'the recipe on the list');
await evaluate(`__clickText('nav button', 'Liste')`);
await waitFor(`__all('li .name').includes('Salade de tomates')`, 'the entry');
const entry = await evaluate(`__all('li .servings span')`);
if (!entry.includes('8 pers.')) {
  throw new Error(`the entry did not keep the servings it was added at: ${JSON.stringify(entry)}`);
}
ok('the recipe goes onto the list at the servings it was read at');

// --- and the recipe survived too -------------------------------------------

await new Promise((resolve) => setTimeout(resolve, 1200));
await load(APP);
await waitFor('document.querySelector("nav")', 'tab bar after the second reload');
await evaluate(`__clickText('nav button', 'Recettes')`);
await waitFor(`__all('li .name').includes('Salade de tomates')`, 'the recipe in the library');
await evaluate(`__clickText('li button', 'Salade de tomates')`);
await waitFor(`__text('h1') === 'Salade de tomates'`, 'the recipe reopens');
const reread = await evaluate(`__text('.prose')`);
if (!reread.includes('Tomates') || reread.includes('supprimée')) {
  throw new Error(`the reference did not survive the round trip: ${JSON.stringify(reread)}`);
}
ok('the recipe and its references came back out of IndexedDB');
await shot('09-reloaded-recipe');

// --- and it can be edited without losing what it points at ------------------
//
// The other half of the editor: a draft seeded from `focus.edit`, whose lines
// already carry ids the steps already reference. A save that dropped or
// re-minted them would turn every mention into "ligne supprimée".

await evaluate(`__clickText('button', 'Modifier')`);
await waitFor(`__text('h1') === 'Modifier'`, 'the editor on an existing recipe');
const seeded = await evaluate(`__count('.chip')`);
if (seeded !== 1) throw new Error(`the draft lost its mention: ${seeded} chips`);

await evaluate(`__set('input[placeholder="Tarte aux tomates"]', 'Salade de tomates au sel')`);
await evaluate(`__clickText('button', 'Enregistrer')`);
await waitFor(`__text('h1') === 'Salade de tomates au sel'`, 'the renamed recipe');
const edited = await evaluate(`__text('.prose')`);
if (!edited.includes('Tomates') || edited.includes('supprimée')) {
  throw new Error(`editing broke the reference: ${JSON.stringify(edited)}`);
}
ok('editing an existing recipe keeps its lines and their mentions');

// --- and the keyboard does not sit on top of it -----------------------------
//
// The last thing M4 owes the phone. iOS never resizes the page for the
// keyboard: the layout viewport keeps its height, the keys are drawn over the
// bottom of it, and a form's last field ends up behind a viewport that still
// reports itself full size. `lib/keyboard.svelte.ts` measures what is covered
// and publishes it as `--keyboard-inset`.
//
// A browser is not a phone, and none of this is the milestone's exit criterion
// — the iPhone is, keys and all. What it does pin down is the half that is
// ours: given a visual viewport 300 px shorter than the page, the layout clears
// those pixels and the picker climbs out of them.

const KEYBOARD = 300;

await send('Emulation.setDeviceMetricsOverride', {
  width: 390,
  height: 640,
  deviceScaleFactor: 1,
  mobile: true,
});

await evaluate(`__clickText('button', 'Modifier')`);
await waitFor(`__text('h1') === 'Modifier'`, 'the editor, with a keyboard coming');

const inset = `getComputedStyle(document.documentElement).getPropertyValue('--keyboard-inset').trim()`;
await evaluate(`__keyboard(${KEYBOARD})`);
await waitFor(`${inset} === '${KEYBOARD}px'`, 'the keyboard, measured');
ok(`a visual viewport ${KEYBOARD}px shorter than the page reads as a keyboard`);

// Without this the document simply ends behind the keys and no amount of
// scrolling brings the last field out — the failure that has no workaround.
const padding = await evaluate(
  `Math.round(Number.parseFloat(getComputedStyle(document.querySelector('.body')).paddingBottom))`,
);
if (padding < KEYBOARD) {
  throw new Error(`the form cannot clear the keys: ${padding}px of padding under it`);
}
ok(`the form can be scrolled clear of the keyboard (${padding}px under it)`);

await waitFor(
  `document.querySelector('nav').getBoundingClientRect().top >=
     document.documentElement.clientHeight - ${KEYBOARD}`,
  'the tab bar, out of the visible band',
);
ok('the tab bar goes down with the keyboard instead of floating over the keys');

// Where iOS leaves a field it has just focused: bottom edge against the top of
// the keyboard. The picker is drawn *below* that, which is to say into the keys
// — and it is drawn there because of what was typed, so nothing scrolled it
// into view on the way in.
await evaluate(`
  (() => {
    const area = document.querySelectorAll('textarea')[0];
    const visibleBottom = document.documentElement.clientHeight - ${KEYBOARD};
    window.scrollBy(0, area.getBoundingClientRect().bottom - visibleBottom);
  })()
`);
const before = await evaluate('window.scrollY');

await evaluate(`__setNth('textarea', 0, 'Couper les @t')`);
await waitFor(`__count('.picker button') === 1`, 'the picker, opening under the keyboard');
await waitFor(
  `document.querySelector('.picker').getBoundingClientRect().bottom <=
     document.documentElement.clientHeight - ${KEYBOARD}`,
  'the picker, out from under the keyboard',
);

const after = await evaluate('window.scrollY');
if (after <= before) {
  throw new Error(`nothing scrolled: the picker was never under the keyboard (${before}px)`);
}
ok(`the mention picker climbs out from under the keys it opened behind (+${after - before}px)`);
await shot('10-keyboard');

await evaluate(`__keyboard(0)`);
await waitFor(`${inset} === '0px'`, 'the keyboard, put away');
ok('and putting it away leaves the layout exactly as it was');

await evaluate(`__clickText('button', 'Annuler')`);
await waitFor(`__text('h1') === 'Salade de tomates au sel'`, 'the recipe, unedited');
await send('Emulation.clearDeviceMetricsOverride');

// --- installable, and it opens with the network off -------------------------
//
// M4's exit criterion, minus the phone. The data has already been proven to
// survive a reload; what is proven here is that the *app* does — the HTML, the
// JS, the CSS and the wasm core, which come off the network on every load
// until a service worker says otherwise (DECISIONS 0038).

await waitFor('navigator.serviceWorker.controller !== null', 'a controlling service worker');
ok('the service worker registered and took control');

const precache = await evaluate(`
  (async () => {
    const names = (await caches.keys()).filter((name) => name.startsWith('cabas-'));
    if (names.length !== 1) return { names };
    const cache = await caches.open(names[0]);
    const keys = await cache.keys();
    return { name: names[0], paths: keys.map((request) => new URL(request.url).pathname) };
  })()
`);

// Exactly one, always: a build owns its cache and deletes every other on
// activation. Two would mean an old app's assets are still on the phone, which
// is how a half-updated PWA happens.
if (!precache.name) {
  throw new Error(`expected one cabas cache, found ${JSON.stringify(precache.names)}`);
}
if (precache.name === 'cabas-__CABAS_BUILD__' || precache.name === 'cabas-undefined') {
  throw new Error(`the build placeholder was never replaced: ${precache.name}`);
}
for (const suffix of ['/', '.js', '.css', '.wasm']) {
  const found =
    suffix === '/'
      ? precache.paths.includes('/')
      : precache.paths.some((path) => path.endsWith(suffix));
  if (!found) {
    throw new Error(`the shell is missing its ${suffix}: ${JSON.stringify(precache.paths)}`);
  }
}
ok(`the shell is precached under ${precache.name} (${precache.paths.length} files)`);

// The manifest and the icons are what make it installable, and a 404 in either
// is silent — iOS simply declines to offer "add to home screen". Fetching them
// also puts them in the runtime cache, which is how they survive the reload
// below: they are copied verbatim out of `public/` and are not in the precache.
const shellAssets = await evaluate(`
  (async () => {
    const paths = [
      '/manifest.webmanifest',
      '/favicon.svg',
      '/icons/icon.svg',
      '/icons/icon-192.png',
      '/icons/icon-512.png',
      '/icons/icon-maskable-512.png',
      '/icons/apple-touch-icon.png',
    ];
    const statuses = {};
    for (const path of paths) statuses[path] = (await fetch(path)).status;
    return statuses;
  })()
`);
const missing = Object.entries(shellAssets).filter(([, status]) => status !== 200);
if (missing.length > 0) {
  throw new Error(`these are not being served: ${JSON.stringify(missing)}`);
}
ok(`the manifest and every icon it names are served (${Object.keys(shellAssets).length} files)`);

const manifest = await evaluate(`fetch('/manifest.webmanifest').then((r) => r.json())`);
if (manifest.start_url !== '/' || manifest.display !== 'standalone') {
  throw new Error(`the manifest would not install standalone: ${JSON.stringify(manifest)}`);
}
if (!manifest.icons?.some((icon) => icon.purpose === 'maskable')) {
  throw new Error('the manifest has no maskable icon');
}
ok(`the manifest is standalone, scoped to ${manifest.scope}, with a maskable icon`);

// Long enough for the debounced flush of the rename above
// (Session.FLUSH_DELAY_MS). Everything from here on reloads the page, and the
// last of those reloads has no network to fall back on.
await new Promise((resolve) => setTimeout(resolve, 1200));

// The update path, which is the half of a service worker that goes wrong
// quietly: a build whose cache outlives it serves the old app forever, and on
// an installed iOS PWA that is indistinguishable from the app being broken.
// A cache from some previous version stands in for that here — activation has
// to take the origin down to exactly one, its own.
await evaluate(`caches.open('cabas-0000stale').then(() => true)`);
await evaluate(`navigator.serviceWorker.getRegistration().then((r) => r.unregister())`);
await load(APP);
await waitFor('navigator.serviceWorker.controller !== null', 'the reinstalled worker');
const remaining = await evaluate('caches.keys()');
if (remaining.length !== 1 || remaining[0] !== precache.name) {
  throw new Error(`activation left caches behind: ${JSON.stringify(remaining)}`);
}
ok('installing drops every cache but its own — an old build cannot outlive itself');

const attached = await setOffline(true);
if (attached === 0) {
  throw new Error('no service worker target to put offline — it would still serve from the network');
}

await load(APP);
if ((await evaluate('navigator.onLine')) !== false) {
  throw new Error('the browser still thinks it is online — the offline test would prove nothing');
}

// And the network is genuinely gone, worker included: a request that misses the
// cache has nowhere to go. Without this the whole section would pass just as
// well against a server that was up the entire time.
const probe = `/offline-probe-${Date.now()}`;
if (await evaluate(`fetch('${probe}').then(() => true, () => false)`)) {
  throw new Error('a cache miss still reached the network — the service worker is not offline');
}
// That failure is the assertion, and the browser logs it as a failed resource.
// Drop exactly it, rather than loosening the check every other line relies on.
await new Promise((resolve) => setTimeout(resolve, 200));
for (let i = consoleErrors.length - 1; i >= 0; i--) {
  if (consoleErrors[i].includes(probe)) consoleErrors.splice(i, 1);
}
ok('the network is gone, for the page and for the worker behind it');

await waitFor('document.querySelector("nav")', 'the app, with no network');
ok('the app boots with the network off');

await evaluate(`__clickText('nav button', 'Recettes')`);
await waitFor(`__all('li .name').includes('Salade de tomates au sel')`, 'the recipe, offline');
await evaluate(`__clickText('li button', 'Salade de tomates au sel')`);
await waitFor(`__text('h1') === 'Salade de tomates au sel'`, 'the recipe opens offline');
const offlineProse = await evaluate(`__text('.prose')`);
if (!offlineProse.includes('Tomates') || offlineProse.includes('supprimée')) {
  throw new Error(`the recipe did not read back offline: ${JSON.stringify(offlineProse)}`);
}
ok('and the library is all there — this is the shop with no signal');
await shot('11-offline');

await setOffline(false);

// --- and it comes back where it was left ------------------------------------
//
// The screen is already persisted; this is the offset within it. Same reason
// (DECISIONS 0003): an iOS cold reload mid-shop otherwise drops you at the top
// of a list you were halfway down.
//
// The viewport is squeezed rather than the library grown — two ingredients do
// not fill a phone, and a hundred would cost a minute of form filling to prove
// something about a scrollbar.
await send('Emulation.setDeviceMetricsOverride', {
  width: 390,
  height: 220,
  deviceScaleFactor: 1,
  mobile: true,
});

await evaluate(`__clickText('nav button', 'Ingrédients')`);
await waitFor(`__text('h1') === 'Ingrédients'`, 'the ingredients screen');
await evaluate('window.scrollTo(0, 10000)');
const left = await evaluate('window.scrollY');
if (left === 0) {
  throw new Error('nothing scrolled — the viewport is not small enough to prove anything');
}

await evaluate(`__clickText('nav button', 'Courses')`);
await waitFor(`__text('h1') === 'Courses'`, 'the cart screen');
await waitFor('window.scrollY === 0', 'the cart, at its own offset');
await evaluate(`__clickText('nav button', 'Ingrédients')`);
await waitFor(`window.scrollY === ${left}`, 'the offset the ingredients screen was left at');
ok(`coming back to a screen returns to where it was (${left}px)`);

await load(APP);
await waitFor('document.querySelector("nav")', 'the app after a cold reload');
await waitFor(`__text('h1') === 'Ingrédients'`, 'the screen it was left on');
await waitFor(`window.scrollY === ${left}`, 'the offset, after a cold reload');
ok('and a cold reload comes back to the same place, not the top');
await shot('12-scroll-restored');

await send('Emulation.clearDeviceMetricsOverride');

// --- and the same library, through a real relay ------------------------------
//
// M5's exit criterion, minus the second phone: this device pushes what it has
// to a relay that cannot read it, loses its replica, and gets everything back
// from the relay alone. Which is also the never-simultaneous case
// (DECISIONS 0009) — nothing else is connected at any point.
//
// The relay is the real binary, started by `ui-test`, because a mock of it
// would only prove that this file and the mock agree.

/** The relay's log for whichever family appeared, or `null` while it is
 *  empty. One family for most of this run — but **not at the end**, where
 *  rotating mints a second one, and where this returns the older of the two
 *  because it takes the first non-empty it finds. Everything below the
 *  rotation must use `waitForNewFamily` instead; assuming "the first
 *  directory is the one" is what made that assertion race. */
async function relayLog() {
  const families = await readdir(RELAY_DATA).catch(() => []);
  for (const family of families) {
    const bytes = await readFile(join(RELAY_DATA, family, 'log')).catch(() => null);
    if (bytes !== null && bytes.length > 0) return bytes;
  }
  return null;
}

async function waitForRelay(label, timeoutMs = 15000) {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    const log = await relayLog();
    if (log !== null) return log;
    if (Date.now() > deadline) throw failed(`timed out waiting for ${label}`);
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
}

/**
 * One family's log, once it has stopped growing.
 *
 * "What the relay had at the moment I looked" is not "what it ends up with":
 * a change is debounced on the device (Session.FLUSH_DELAY_MS) and then has a
 * round trip to make. Taking the first and comparing it against the second is
 * exactly how the untouched-log check below failed on a runner — the deletion
 * from the journal section was still in flight when its baseline was taken,
 * and landed in the old family a moment later, correctly.
 *
 * @param {string} id
 * @param {string} label
 * @returns {Promise<Buffer>}
 */
async function waitForSettledLog(id, label, quietMs = 1500, timeoutMs = 25000) {
  const deadline = Date.now() + timeoutMs;
  let previous = -1;
  let stableSince = Date.now();
  for (;;) {
    const log = (await readFile(join(RELAY_DATA, id, 'log')).catch(() => null)) ?? Buffer.alloc(0);
    if (log.length !== previous) {
      previous = log.length;
      stableSince = Date.now();
    } else if (Date.now() - stableSince >= quietMs) {
      return log;
    }
    if (Date.now() > deadline) throw failed(`timed out waiting for ${label} to settle`);
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
}

/**
 * A family that is not one of `known`, once it has actually been written to.
 *
 * Rotating the phrase is the only thing here that produces a second family,
 * and two events have to land before it can be asserted on: the relay creates
 * the directory on the new `Hello`, and the device pushes its library a moment
 * later. Waiting for the directory alone would swap one race for another.
 *
 * @param {string[]} known
 * @param {string} label
 * @param {number} timeoutMs
 * @returns {Promise<{ id: string, bytes: number }>}
 */
async function waitForNewFamily(known, label, timeoutMs = 20000) {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    const families = await readdir(RELAY_DATA).catch(() => []);
    for (const id of families) {
      if (known.includes(id)) continue;
      const log = await readFile(join(RELAY_DATA, id, 'log')).catch(() => null);
      if (log !== null && log.length > 0) return { id, bytes: log.length };
    }
    if (Date.now() > deadline) {
      throw failed(`timed out waiting for ${label} (families: ${JSON.stringify(families)})`);
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
}

// The family was created on the pairing screen at the top of this file and
// the relay set right after, so everything above has been syncing as it was
// built. What is left is to look at what reached the relay.
const log = await waitForRelay('the push to reach the relay');
ok(`the engine connected and pushed (${log.length} bytes of sealed log)`);

// The whole point of the relay (Rule 7). If any of this reads as text, the
// frames are not sealed and nothing else in this file matters.
for (const secret of ['Tomates', 'Salade de tomates', 'Sel']) {
  if (log.includes(Buffer.from(secret, 'utf8'))) {
    throw new Error(`the relay's log contains "${secret}" in the clear`);
  }
}
ok('and the relay cannot read a word of it');

// Lose the replica: a device that was wiped, or one iOS evicted, with nothing
// left but what is in `localStorage`.
//
// The cursor is deliberately **not** cleared here, and the request to clear it
// would not survive anyway — leaving the page runs the engine's own
// `pagehide`, which writes the cursor back on the way out. So this is the
// harder case of the two: a device whose cursor says "I have everything up to
// frame N" on behalf of a replica that has nothing. If the engine resumed that
// cursor the relay would honestly replay nothing and the library would stay
// empty for good, which is why the core is asked whether it opened fresh.
await evaluate(`
  new Promise((resolve) => {
    const request = indexedDB.deleteDatabase('cabas');
    request.onsuccess = resolve;
    request.onerror = resolve;
    request.onblocked = resolve;
  })
`);
await load(APP);
await waitFor('document.querySelector("nav")', 'the app, on an empty replica');

await evaluate(`__clickText('nav button', 'Ingrédients')`);
await waitFor(`__text('h1') === 'Ingrédients'`, 'the ingredients screen');
await waitFor(`__all('li .name').includes('Tomates')`, 'the library, back from the relay');
ok('an empty replica gets the whole library back from the relay alone');

await evaluate(`__clickText('nav button', 'Recettes')`);
await waitFor(`__all('li .name').includes('Salade de tomates au sel')`, 'the recipe, resynced');
await evaluate(`__clickText('li button', 'Salade de tomates au sel')`);
await waitFor(`__text('h1') === 'Salade de tomates au sel'`, 'the recipe opens');
const resynced = await evaluate(`__text('.prose')`);
if (!resynced.includes('Tomates') || resynced.includes('supprimée')) {
  throw new Error(`the recipe did not survive the round trip: ${JSON.stringify(resynced)}`);
}
ok('recipe, steps and the lines they point at, all of it');
await shot('13-synced');

// --- and a second device, joining by hand ------------------------------------
//
// Everything this device is, gone: identity, family, cursor and replica. What
// comes back is a phone that has only the twelve words someone read out to it,
// which is the path 0021 makes mandatory and the one that has to work when a
// camera does not.

await evaluate(`
  (async () => {
    localStorage.clear();
    await new Promise((resolve) => {
      const request = indexedDB.deleteDatabase('cabas');
      request.onsuccess = request.onerror = request.onblocked = resolve;
    });
    return true;
  })()
`);
await load(APP);
await waitFor(`__text('h1') === 'cabas'`, 'the pairing screen, on a blank device');

await evaluate(`__clickText('button', 'Rejoindre une famille')`);
await waitFor('document.querySelector("textarea")', 'the phrase field');

await evaluate(`__set('textarea', 'abandon abandon abandon')`);
await evaluate(`__click('button[type="submit"]')`);
await waitFor(`__text('[role="alert"]')?.includes('douze mots')`, 'the complaint about the count');
ok('a phrase of the wrong length is refused, in French, before anything is stored');

await evaluate(`__set('textarea', ${JSON.stringify(phrase.toUpperCase())})`);
await evaluate(`__click('button[type="submit"]')`);
await waitFor('document.querySelector("form input")', 'the naming form');
ok('and the right one is accepted whatever the case it is typed in');

await evaluate(`__set('input[autocomplete="given-name"]', 'Camille')`);
await evaluate(`__set('form label:nth-of-type(2) input', 'Téléphone de Camille')`);
await evaluate(`__click('button[type="submit"]')`);
await waitFor('document.querySelector("nav")', 'the app, on the device that joined');

await evaluate(`__clickText('nav button', 'Réglages')`);
await waitFor(`__text('h1') === 'Réglages'`, 'settings, on the joined device');
await evaluate(`__set('.family input', ${JSON.stringify(RELAY_URL)})`);
await evaluate(`__clickText('.family button', 'Enregistrer le serveur')`);

await evaluate(`__clickText('nav button', 'Ingrédients')`);
await waitFor(`__all('li .name').includes('Tomates')`, 'the family library, on the new device');
ok('twelve words typed by hand are the whole of pairing — the library follows');
await shot('14-joined');

// --- and both devices are on the roster --------------------------------------
//
// The screen that looks like access control and is not one (Rule 7). It can
// only be checked here, after two devices have met: before this point the
// family has one of each and proves nothing about grouping.

await evaluate(`__clickText('nav button', 'Réglages')`);

// --- and it says which build it is -------------------------------------------
//
// The answer to "did the update land", which a service worker otherwise makes
// unanswerable: a new build installs behind the running one and takes over at
// the next launch (0038). It comes from the core, so this compares it to the
// workspace version — the string the Supervisor compares too (Rule 15), and
// the one `check-addon` holds the add-on manifest to.

await waitFor(`__text('h1') === 'Réglages'`, 'settings, to read the build off');

const cargoToml = await readFile(
  join(dirname(fileURLToPath(import.meta.url)), '..', '..', 'Cargo.toml'),
  'utf8',
);
const workspaceVersion = /^version = "([^"]+)"$/m.exec(cargoToml)?.[1];
const shown = await evaluate(`document.querySelector('.build')?.dataset.version ?? null`);
if (shown !== workspaceVersion) {
  throw new Error(
    `settings shows version ${JSON.stringify(shown)}, the workspace is at ` +
      `${JSON.stringify(workspaceVersion)} — a stale build-wasm, or the line is gone`,
  );
}
ok(`settings names the build it is running (${shown})`);

await evaluate(`__clickText('button', 'Personnes et appareils')`);
await waitFor(`__text('h1') === 'Personnes et appareils'`, 'the roster');
await waitFor(`__all('.people > li .name').includes('Alexis')`, 'the person who started it');

const roster = JSON.parse(
  await evaluate(`
    JSON.stringify([...document.querySelectorAll('.people > li')].map((person) => ({
      name: person.querySelector('.name').textContent.trim(),
      devices: [...person.querySelectorAll('.devices .device-name')].map((d) => d.textContent.trim()),
    })))
  `),
);
const camille = roster.find((person) => person.name.startsWith('Camille'));
const alexis = roster.find((person) => person.name.startsWith('Alexis'));
if (roster.length !== 2 || !camille || !alexis) {
  throw failed(`the roster is not two people: ${JSON.stringify(roster)}`);
}
if (!alexis.devices.includes('iPhone de test') || !camille.devices.includes('Téléphone de Camille')) {
  throw failed(`devices are under the wrong person: ${JSON.stringify(roster)}`);
}
if (!camille.name.includes('vous')) {
  throw failed(`this device belongs to Camille and the screen does not say so: ${camille.name}`);
}
ok('the roster shows both people, each with the device they paired');

// The warning is the point of the screen, so it is asserted like anything else.
const revocation = await evaluate(`__text('.revoke')`);
for (const claim of ['pas de moyen de retirer un seul appareil', 'la même clé']) {
  if (!revocation.includes(claim)) {
    throw failed(`the revocation warning does not say "${claim}": ${JSON.stringify(revocation)}`);
  }
}
ok('and states plainly that there is no such thing as revoking one of them (Rule 7)');
await shot('15-people');

// --- the journal ------------------------------------------------------------
//
// What the data cannot remember. Read here, on the device that joined second,
// so the first entry it shows was written by somebody else and arrived sealed
// through the relay.

await evaluate(`__clickText('button', 'Retour')`);
await waitFor(`__text('h1') === 'Réglages'`, 'settings again');
await evaluate(`__clickText('button', 'Journal')`);
await waitFor(`__text('h1') === 'Journal'`, 'the journal');

await waitFor(
  `__all('li .what').some((line) => line.includes('Alexis') && line.includes('a modifié'))`,
  "the other device's edit",
);
ok("the journal shows what the other device did, in the other device's name");

// A deletion, made here, from the screen that offers it — the only kind of
// event nothing else on screen can show, since the thing itself is gone.
await evaluate(`__clickText('nav button', 'Ingrédients')`);
await waitFor(`__text('h1') === 'Ingrédients'`, 'the ingredients screen');
await evaluate(`__clickText('button', 'Nouveau')`);
await waitFor('document.querySelector("form")', 'the ingredient form');
await evaluate(`__set('form label:nth-of-type(1) input', 'Cannelle')`);
await evaluate(`__click('button[type="submit"]')`);
await waitFor(`__all('li .name').includes('Cannelle')`, 'the throwaway ingredient');

await evaluate(`__clickText('li button', 'Cannelle')`);
await waitFor('document.querySelector("form")', 'the edit form');
await evaluate(`__clickText('form button', 'Supprimer')`);
await evaluate(`__clickText('form button', 'Confirmer la suppression')`);
await waitFor(`!__all('li .name').includes('Cannelle')`, 'the ingredient, gone');

await evaluate(`__clickText('nav button', 'Réglages')`);
await evaluate(`__clickText('button', 'Journal')`);
await waitFor(`__text('h1') === 'Journal'`, 'the journal again');
const newest = await evaluate(`__text('li:first-child .what')`);
for (const claim of ['Camille', 'vous', 'a supprimé', "l'ingrédient", 'Cannelle']) {
  if (!newest.includes(claim)) {
    throw failed(`the newest entry does not say "${claim}": ${JSON.stringify(newest)}`);
  }
}
ok('and a deletion made here lands at the top of it, named and attributed');
await shot('16-journal');

// Rotating the key is the whole of revocation, and it is destructive enough to
// be the last thing this file does: the family it leaves behind is the one
// every assertion above was made against.
// By name, not by count: what comes after has to tell the new family from the
// old one, and identify the old one again to prove nothing wrote into it.
//
// Settled, not merely read: everything above this line has been pushing as it
// went, and the journal's deletion is the most recent of them. A baseline
// taken while that is still in flight makes the untouched check below fail on
// the arrival of a change that predates the rotation entirely.
const familiesBefore = await readdir(RELAY_DATA);
if (familiesBefore.length !== 1) {
  throw failed(`one family up to this point, found ${JSON.stringify(familiesBefore)}`);
}
const abandoned = familiesBefore[0];
const abandonedLog = await waitForSettledLog(abandoned, 'the family about to be abandoned');
await evaluate(`__clickText('button', 'Retour')`);
await waitFor(`__text('h1') === 'Réglages'`, 'settings, on the way to the roster');
await evaluate(`__clickText('button', 'Personnes et appareils')`);
await waitFor(`__text('h1') === 'Personnes et appareils'`, 'the roster, to rotate from');
await evaluate(`__clickText('.revoke button', 'Changer la phrase de la famille')`);
// Four, and the count is the assertion's teeth: rotating is the one
// irreversible thing in the app, and every consequence of it — including the
// log it leaves on the relay (DECISIONS 0050) — is named before it happens.
await waitFor(`__count('.consequences li') === 4`, 'the consequences, before anything happens');
ok('rotating asks first, and says what it costs — all four of them');

await evaluate(`__clickText('.revoke button', 'Changer la phrase')`);
await waitFor(`__text('.revoke [data-phrase]')`, 'the new phrase');
const rotated = await evaluate(`__text('.revoke [data-phrase]')`);
if (rotated.split(/\s+/).length !== 12 || rotated === phrase) {
  throw failed(`the new phrase is not a new phrase: ${JSON.stringify(rotated)}`);
}
const stored = await evaluate(`JSON.parse(localStorage.getItem('cabas.family')).phrase`);
if (stored !== rotated) {
  throw failed('the device kept its old phrase');
}
ok('a new phrase is minted and this device moves to it');

// A different phrase is a different family id, so the relay grows a second
// log rather than writing into the first. The old one stays exactly where it
// was — sealed, and readable only by whoever still has the old words. That is
// the log DECISIONS 0050 is about: nothing here will ever collect it.
//
// Waited for by *name*, and not through `waitForRelay`: that one returns as
// soon as any family has a non-empty log, and the family this rotation
// abandoned has had one for the whole run — so it waited for a condition that
// was already true and left the assertion below racing the new device's first
// push. It won that race on a laptop and lost it on a runner.
const arrived = await waitForNewFamily(familiesBefore, 'the library, pushed into the new family');
const familiesAfter = await readdir(RELAY_DATA);
if (familiesAfter.length !== familiesBefore.length + 1) {
  throw failed(
    `expected one more family on the relay, found ${familiesAfter.length} against ${familiesBefore.length}`,
  );
}
// "Untouched" was claimed here long before anything checked it.
const abandonedNow = await readFile(join(RELAY_DATA, abandoned, 'log'));
if (!abandonedNow.equals(abandonedLog)) {
  throw failed(`rotating wrote ${abandonedNow.length - abandonedLog.length} bytes into the old family`);
}
ok(`and the relay holds a second family (${arrived.bytes} bytes), the first one untouched`);
await shot('17-rotated');

if (consoleErrors.length > 0) {
  throw new Error(`the page logged errors:\n${consoleErrors.join('\n')}`);
}

console.log('\nui-test: ok');
socket.close();
