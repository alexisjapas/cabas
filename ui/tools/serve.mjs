/**
 * Serves the built PWA to a phone on the same wifi, over TLS.
 *
 * This exists for one item on the roadmap: installing the app on the actual
 * iPhone and using it with the network off. A service worker only registers in
 * a **secure context**, and `http://192.168.x.y` is not one — so `pnpm dev`,
 * which binds to the LAN over plain HTTP, can show the app on the phone and can
 * never make it installable. Everything M4 still has to answer is downstream of
 * that: no worker, no precache, and airplane mode is a blank page.
 *
 * `ui-serve` in the flake mints the certificate and calls this; the reasoning
 * behind a local CA rather than a tunnel is DECISIONS 0041.
 *
 * Two servers, on purpose:
 *
 *   - **HTTPS** serves `ui/dist`. This is the origin the phone installs.
 *   - **HTTP** serves the CA certificate and nothing else, because the phone
 *     cannot fetch it over the HTTPS it does not trust yet — Safari's
 *     certificate interstitial has no "download anyway".
 *
 * Zero dependencies, like `render-icons.mjs` and `tests/smoke.mjs`: it is a
 * development tool, it is not part of the bundle, and `tsconfig.json` covers
 * `src/` only.
 */

import { createServer as createHttpServer } from 'node:http';
import { createServer as createHttpsServer } from 'node:https';
import { readFile } from 'node:fs/promises';
import { extname, join, resolve, sep } from 'node:path';

const [dist, certs, httpsPort, httpPort, ...hosts] = process.argv.slice(2);
if (dist === undefined || certs === undefined || httpsPort === undefined || httpPort === undefined) {
  console.error('usage: serve.mjs <dist> <certs> <https-port> <http-port> <host>...');
  process.exit(2);
}

const root = resolve(dist);
const ca = join(certs, 'ca.crt');

/**
 * What a browser is told each file is.
 *
 * `.wasm` is the load-bearing one: `WebAssembly.instantiateStreaming` rejects
 * any other type, and the glue `wasm-bindgen` generates falls back to a
 * non-streaming path that mostly works — so a wrong type here surfaces as a
 * slow cold start on a phone rather than as an error. `.webmanifest` is the
 * other: iOS reads the manifest to decide the app is installable at all.
 */
const TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.wasm': 'application/wasm',
  '.json': 'application/json; charset=utf-8',
  '.map': 'application/json; charset=utf-8',
  '.webmanifest': 'application/manifest+json',
  '.png': 'image/png',
  '.svg': 'image/svg+xml',
  '.ico': 'image/vnd.microsoft.icon',
  '.txt': 'text/plain; charset=utf-8',
};

const https = createHttpsServer(
  {
    key: await readFile(join(certs, 'server.key')),
    cert: await readFile(join(certs, 'server.crt')),
  },
  (request, response) => {
    void serve(request, response);
  },
);

/**
 * The PWA, straight off the disk.
 *
 * @param {import('node:http').IncomingMessage} request
 * @param {import('node:http').ServerResponse} response
 */
async function serve(request, response) {
  if (request.method !== 'GET' && request.method !== 'HEAD') {
    response.writeHead(405, { allow: 'GET, HEAD' }).end();
    return;
  }

  // The base is a formality: only the path is read, and a request line always
  // carries an absolute path.
  const url = new URL(request.url ?? '/', 'https://cabas.invalid');
  let pathname = decodeURIComponent(url.pathname);
  if (pathname.endsWith('/')) pathname += 'index.html';

  // `resolve` collapses `..` before anything touches the filesystem, and the
  // prefix check is what makes that a boundary rather than a hope.
  const target = resolve(root, `.${pathname}`);
  if (target !== root && !target.startsWith(root + sep)) {
    response.writeHead(403).end('403');
    return;
  }

  let body;
  try {
    body = await readFile(target);
  } catch {
    // No SPA fallback. Which screen is open is core state, not a URL
    // (DECISIONS 0037), so there is no deep link that could legitimately
    // arrive here — and answering every typo with the app hides real 404s
    // from the precache list.
    response.writeHead(404, { 'content-type': 'text/plain; charset=utf-8' });
    response.end(request.method === 'HEAD' ? undefined : `404 ${pathname}\n`);
    return;
  }

  response.writeHead(200, {
    'content-type': TYPES[extname(target)] ?? 'application/octet-stream',
    'content-length': body.byteLength,
    // The Cache API is independent of HTTP caching, so the worker still
    // precaches everything this refuses to let Safari keep. What it buys is an
    // honest reload while iterating: on an installed PWA a stale shell served
    // out of the browser's own cache is indistinguishable from a build that
    // did not take.
    'cache-control': 'no-store',
    // Deliberately no `Vary`. Vite's preview answers `Vary: Origin`, which
    // makes the Cache API match on the request's `Origin` header and turns
    // every precache hit into a miss — invisible online, a blank page off it
    // (DECISIONS 0038). The worker passes `ignoreVary`, so this is belt and
    // braces; it also keeps this server honest about what the relay should do
    // at M6.
  });
  response.end(request.method === 'HEAD' ? undefined : body);
}

/**
 * The certificate, and the two steps iOS needs after it.
 *
 * Plain HTTP because this is what bootstraps the trust: the phone has no way to
 * accept the HTTPS certificate before it holds the CA that signed it.
 */
const http = createHttpServer((request, response) => {
  void (async () => {
    if (request.url === '/ca.crt') {
      // This exact media type is what makes iOS treat the download as a
      // configuration profile rather than as a file it has no app for.
      response.writeHead(200, { 'content-type': 'application/x-x509-ca-cert' });
      response.end(await readFile(ca));
      return;
    }
    response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
    response.end(page());
  })();
});

/**
 * The instruction page, served to the phone before it trusts anything.
 *
 * Step 2 is the one that gets forgotten, and it fails in the most misleading
 * way available: the profile is installed, the certificate is listed, and
 * Safari still refuses the origin — because on iOS installing a root and
 * trusting it are two separate actions, in two different screens.
 *
 * @returns {string}
 */
function page() {
  const links = hosts
    .map((host) => `<li><a href="https://${host}:${httpsPort}/">https://${host}:${httpsPort}/</a></li>`)
    .join('');
  return `<!doctype html>
<html lang="fr">
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>cabas — certificat local</title>
<style>
  body { font: 17px/1.5 -apple-system, system-ui, sans-serif; margin: 0 auto; max-width: 34rem; padding: 1.5rem; }
  a { color: #06c; }
  code { background: #eee; border-radius: 3px; padding: 0 .25em; }
  li { margin: .4rem 0; }
</style>
<h1>cabas — certificat local</h1>
<ol>
  <li><a href="/ca.crt"><b>Télécharger le certificat</b></a>, puis
      Réglages → <i>Profil téléchargé</i> → Installer.</li>
  <li><b>Réglages → Général → Informations → Réglages de confiance certificat</b>,
      et activer <code>cabas local CA</code>.
      <br>Sans cette étape le certificat est installé mais pas approuvé, et Safari
      refuse quand même l'adresse.</li>
  <li>Ouvrir l'app, puis <i>Partager → Sur l'écran d'accueil</i> :
      <ul>${links}</ul></li>
</ol>
<p>Une fois l'app lancée depuis l'écran d'accueil, activer le mode avion et la
   rouvrir : elle doit démarrer et afficher les mêmes données.</p>
</html>
`;
}

/**
 * Refuse a port something else already holds, rather than half-starting. The
 * same reasoning as `ui-test`: attaching to whatever is already there means
 * reporting on something this run did not serve.
 *
 * @param {import('node:net').Server} server
 * @param {string} port
 * @param {string} what
 */
function listen(server, port, what) {
  return new Promise((ok, fail) => {
    server.once('error', (error) => {
      const code = /** @type {NodeJS.ErrnoException} */ (error).code;
      fail(
        code === 'EADDRINUSE'
          ? new Error(`${what}: port ${port} is already in use — another ui-serve still running?`)
          : error,
      );
    });
    server.listen(Number(port), '0.0.0.0', () => ok(undefined));
  });
}

await listen(https, httpsPort, 'the app');
await listen(http, httpPort, 'the certificate');

// Both names for both servers. The mDNS one is the one to install from — an
// installed PWA is identified by its origin, so an app added to the home screen
// from an address DHCP can move loses its library the day it moves (DECISIONS
// 0012). The IP is the fallback for a phone that does not resolve `.local`, and
// the certificate page is where that gets discovered, so it needs both too.
const list = hosts.length > 0 ? hosts : ['0.0.0.0'];
console.log('\n  the certificate  ← open this on the phone first');
for (const host of list) console.log(`                   http://${host}:${httpPort}/`);
console.log('\n  the app          ← once the certificate is installed *and* trusted');
for (const host of list) console.log(`                   https://${host}:${httpsPort}/`);
console.log('\n  ctrl-c to stop\n');
