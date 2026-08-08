/**
 * Rasterise the home-screen icons from their SVG sources.
 *
 * iOS reads `apple-touch-icon` as a bitmap and ignores SVG, so the PNGs beside
 * the sources are committed assets rather than build products — nothing in the
 * everyday loop runs this. It exists so that changing the drawing is an edit to
 * `icon.svg` and one command, instead of an edit to `icon.svg` and a paragraph
 * of prose about how the PNGs were once made.
 *
 * Run it in the browser shell, which is where chromium lives:
 *
 *   nix develop .#wasm-test -c node ui/tools/render-icons.mjs
 *
 * Driven over the DevTools protocol for the same reason `tests/smoke.mjs` is —
 * Node has `fetch` and `WebSocket` built in, and a renderer that needs its own
 * npm tree is a second lockfile to keep honest. The protocol is also the only
 * way to get an exact viewport: `--window-size` sizes the *window*, and the
 * screenshot then comes out a browser frame short.
 */

import { spawn } from 'node:child_process';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const ICONS = fileURLToPath(new URL('../public/icons/', import.meta.url));

/** Each PNG the manifest and `index.html` name, and the drawing it comes from. */
const OUTPUTS = [
  { source: 'icon.svg', size: 192, name: 'icon-192.png' },
  { source: 'icon.svg', size: 512, name: 'icon-512.png' },
  // 180 is what iOS asks for; it downsamples the rest itself.
  { source: 'icon.svg', size: 180, name: 'apple-touch-icon.png' },
  { source: 'icon-maskable.svg', size: 512, name: 'icon-maskable-512.png' },
];

const PORT = 9333;
const profile = await mkdtemp(join(tmpdir(), 'cabas-icons-'));
const browser = spawn(
  'chromium',
  [
    '--headless',
    '--no-sandbox',
    '--disable-gpu',
    '--hide-scrollbars',
    `--remote-debugging-port=${PORT}`,
    `--user-data-dir=${profile}`,
    'about:blank',
  ],
  { stdio: 'ignore' },
);

let socket;
try {
  socket = await connect();
  for (const output of OUTPUTS) await render(socket, output);
} finally {
  socket?.close();
  browser.kill();
  // Wait for it to actually be gone: chromium keeps writing its profile for a
  // moment after SIGTERM, and removing the directory under it fails with
  // ENOTEMPTY on a race the icons have already survived.
  await new Promise((resolve) => browser.on('exit', resolve));
  await rm(profile, { recursive: true, force: true });
}

console.log(`\nrender-icons: ${OUTPUTS.length} icons written to ui/public/icons/`);

// --- the protocol, and nothing but ------------------------------------------

async function connect() {
  for (let attempt = 0; attempt < 100; attempt++) {
    try {
      const targets = await (await fetch(`http://localhost:${PORT}/json/list`)).json();
      const page = targets.find((candidate) => candidate.type === 'page');
      if (page) return await open(page.webSocketDebuggerUrl);
    } catch {
      // Not up yet. It binds the port a moment after forking.
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error('chromium never came up');
}

function open(url) {
  const pending = new Map();
  const ws = new WebSocket(url);
  ws.onmessage = (event) => {
    const message = JSON.parse(event.data);
    const waiter = pending.get(message.id);
    if (!waiter) return;
    pending.delete(message.id);
    if (message.error) waiter.reject(new Error(JSON.stringify(message.error)));
    else waiter.resolve(message.result);
  };
  let nextId = 0;
  ws.send_ = (method, params = {}) => {
    const id = ++nextId;
    return new Promise((resolve, reject) => {
      pending.set(id, { resolve, reject });
      ws.send(JSON.stringify({ id, method, params }));
    });
  };
  return new Promise((resolve, reject) => {
    ws.onopen = () => resolve(ws);
    ws.onerror = reject;
  });
}

async function render(socket, { source, size, name }) {
  // The SVG goes in as text rather than as a `file://` image: an <img> load is
  // one more thing to wait for, and an inlined document is painted with the
  // page. Its own width/height are overridden here so one source serves every
  // size.
  const svg = readFileSync(join(ICONS, source), 'utf8').replace(
    /<svg([^>]*)width="\d+" height="\d+"/,
    `<svg$1width="${size}" height="${size}"`,
  );
  // `display:block` is load-bearing: an inline <svg> sits on a text baseline,
  // which leaves a few pixels of descender space under it, which overflows a
  // viewport sized to the drawing, which puts a scrollbar in the screenshot.
  const style = 'html,body{margin:0;overflow:hidden}svg{display:block}';
  const page = `<!doctype html><meta charset="utf-8"><style>${style}</style>${svg}`;

  await socket.send_('Emulation.setDeviceMetricsOverride', {
    width: size,
    height: size,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await socket.send_('Page.navigate', {
    url: `data:text/html;charset=utf-8,${encodeURIComponent(page)}`,
  });
  await socket.send_('Runtime.evaluate', {
    expression: 'new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)))',
    awaitPromise: true,
  });

  const { data } = await socket.send_('Page.captureScreenshot', { format: 'png' });
  await writeFile(join(ICONS, name), Buffer.from(data, 'base64'));
  console.log(`  ok  ${name} (${size}×${size}, from ${source})`);
}
