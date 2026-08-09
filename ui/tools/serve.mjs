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
 *   - **HTTPS** serves `ui/dist`, and proxies `/sync` to the relay (DECISIONS
 *     0044). This is the origin the phone installs.
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
import { connect } from 'node:net';
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
 * Where the relay listens, as `host:port`. Matches `CABAS_RELAY_ADDR`'s
 * default on the relay side; override with `CABAS_RELAY` to reach one
 * somewhere else on the network.
 *
 * Parsed through `URL` for the one case a split on `:` gets wrong — an IPv6
 * literal — whose brackets belong to the URL syntax and not to the address
 * `connect` wants.
 */
const authority = process.env.CABAS_RELAY ?? '127.0.0.1:8787';
const relay = new URL(`tcp://${authority}`);
const relayHost = relay.hostname.replace(/^\[|\]$/g, '');
const relayPort = Number(relay.port === '' ? 8787 : relay.port);

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
 * The sync socket, handed to the relay (DECISIONS 0044).
 *
 * A page served over `https:` may not open a `ws:` — the browser blocks it as
 * mixed content — and the relay terminates no TLS, because in production the
 * Cloudflare Tunnel does that in front of it (0012). Without this, the one
 * configuration M5 has to be proven in, an installed PWA on a phone, is the one
 * configuration that cannot reach a development relay. Proxying here also keeps
 * the two sides on **one origin**, which is what production looks like, so the
 * relay URL defaulting to the app's own origin (0043) is exercised rather than
 * first attempted at M6.
 *
 * This speaks no WebSocket. It forwards the handshake verbatim and copies bytes
 * in both directions afterwards, so there is no frame parser to get wrong, no
 * dependency to add, and nothing here that could read the payload if it were
 * not already sealed (Rule 7).
 */
https.on('upgrade', (request, socket, head) => {
  const url = new URL(request.url ?? '/', 'https://cabas.invalid');
  if (url.pathname !== '/sync') {
    socket.end('HTTP/1.1 404 Not Found\r\n\r\n');
    return;
  }

  // A foreground session holds this open for as long as the app is on screen
  // (DECISIONS 0011), idle between edits. Neither half may time it out, and
  // small frames should not wait on Nagle.
  socket.setNoDelay(true);
  socket.setTimeout(0);

  const upstream = connect(relayPort, relayHost);
  upstream.setNoDelay(true);
  upstream.setTimeout(0);

  let piping = false;
  upstream.on('connect', () => {
    upstream.write(handshake(request));
    // Bytes the parser read past the headers. Empty in practice — a client
    // waits for `101` before framing anything — but dropping them would be a
    // corruption that only appears under load.
    if (head.length > 0) upstream.write(head);
    socket.pipe(upstream);
    upstream.pipe(socket);
    piping = true;
  });

  upstream.on('error', (error) => {
    // Once frames are flowing, an HTTP response written into this socket
    // reaches the client as garbage inside a WebSocket. Dropping the
    // connection is both correct and what the frontend already handles: a
    // relay restarted mid-session is an ordinary event here.
    if (piping) {
      socket.destroy();
      return;
    }
    // Before the handshake there is still an HTTP conversation to answer, and
    // this is the failure by a wide margin — with a phone in hand — so it says
    // what to start rather than which errno came back.
    console.error(`\n  /sync → ${authority}: ${error.message}`);
    console.error('  the relay does not seem to be running:');
    console.error('    CABAS_RELAY_DATA=.relay cargo run -p cabas-relay\n');
    socket.end('HTTP/1.1 502 Bad Gateway\r\n\r\n');
  });

  // `pipe` forwards the end of a stream but not the death of one, and half a
  // proxied socket is worse than none: the app would hold a connection whose
  // other end is gone instead of reconnecting.
  socket.on('error', () => upstream.destroy());
  socket.on('close', () => upstream.destroy());
  upstream.on('close', () => socket.destroy());
});

/**
 * The client's upgrade request, rebuilt on the wire.
 *
 * `rawHeaders` keeps what arrived, in order and in case, which matters because
 * these headers are a handshake: `Sec-WebSocket-Key` is answered with a hash of
 * itself. Only `Host` is rewritten — the relay routes on the path and would not
 * notice, but a proxy that tells an upstream it is someone else is a debugging
 * session waiting to happen.
 *
 * @param {import('node:http').IncomingMessage} request
 * @returns {string}
 */
function handshake(request) {
  const lines = [`${request.method} ${request.url} HTTP/1.1`];
  const raw = request.rawHeaders;
  for (let i = 0; i < raw.length; i += 2) {
    lines.push(raw[i].toLowerCase() === 'host' ? `Host: ${authority}` : `${raw[i]}: ${raw[i + 1]}`);
  }
  return `${lines.join('\r\n')}\r\n\r\n`;
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
console.log(`\n  the sync socket  ← /sync on that same origin, proxied to ${authority}`);
console.log('                   CABAS_RELAY moves it elsewhere');
console.log('\n  ctrl-c to stop\n');
