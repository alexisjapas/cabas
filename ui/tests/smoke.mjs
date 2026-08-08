/**
 * The whole vertical, in a real browser: mint an identity, build a library,
 * put something on the list, derive the cart, tick a line, reload from
 * IndexedDB.
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
 * Run it with `ui-test` in the browser shell — it expects a built `ui/dist`,
 * a preview server on 4173 and chromium listening on 9222.
 */

import { writeFile } from 'node:fs/promises';

const DEVTOOLS = process.env.DEVTOOLS_URL ?? 'http://localhost:9222';
const APP = process.env.APP_URL ?? 'http://localhost:4173';
const SHOTS = process.env.SCREENSHOT_DIR ?? null;

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
    consoleErrors.push(`[${entry.source}] ${entry.text || '(no text)'} ${entry.url ?? ''}`.trim());
  }
  if (message.method === 'Runtime.exceptionThrown') {
    consoleErrors.push(message.params.exceptionDetails.text ?? 'uncaught exception');
  }
};

function send(method, params = {}) {
  const id = ++nextId;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    socket.send(JSON.stringify({ id, method, params }));
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
    if (Date.now() > deadline) throw new Error(`timed out waiting for ${label}`);
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
}

/**
 * Svelte listens for real events, so assigning `.value` changes the pixel and
 * nothing else. These go through the native setter and then dispatch, which is
 * what a keystroke does.
 */
const HELPERS = `
  window.__set = (selector, value) => {
    const el = document.querySelector(selector);
    if (!el) throw new Error('missing ' + selector);
    const proto = el instanceof HTMLSelectElement ? HTMLSelectElement.prototype : HTMLInputElement.prototype;
    Object.getOwnPropertyDescriptor(proto, 'value').set.call(el, value);
    el.dispatchEvent(new Event('input', { bubbles: true }));
    el.dispatchEvent(new Event('change', { bubbles: true }));
  };
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
}

async function shot(name) {
  if (SHOTS === null) return;
  const { data } = await send('Page.captureScreenshot', { format: 'png' });
  await writeFile(`${SHOTS}/${name}.png`, Buffer.from(data, 'base64'));
}

function ok(label) {
  console.log(`  ok  ${label}`);
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
await waitFor('document.querySelector("form input")', 'onboarding form');
ok('onboarding renders on a device with no identity');

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

if (consoleErrors.length > 0) {
  throw new Error(`the page logged errors:\n${consoleErrors.join('\n')}`);
}

console.log('\nui-test: ok');
socket.close();
