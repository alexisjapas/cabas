import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vite';
import type { Plugin } from 'vite';

// A single-page bundle of static files: no server renders anything, and at M6
// the whole `dist/` is embedded into the relay binary (DECISIONS 0037, 0010).
//
// Nothing here uses a Node API on purpose — that is what lets this file be
// type-checked by the same strict `tsconfig.json` as the browser code, with no
// second config and no `@types/node`. The service-worker plugin below is
// written to keep that true: it reads the bundle the bundler hands it and never
// the file system.

/**
 * The placeholder `src/sw.js` declares, and what replaces it.
 *
 * Matched with its quotes, so the replacement produces a string literal again.
 * All three kinds are accepted because the minifier rewrites them: rolldown
 * normalises string literals to backticks, and a pattern that only knew about
 * `'` matched nothing and failed the build — which is the good failure. The
 * silent one would be a worker shipping the placeholder as its version.
 */
const BUILD_TOKEN = /(['"`])__CABAS_BUILD__\1/;

/**
 * Writes the precache list and the cache name into the service worker, from the
 * build that just happened (DECISIONS 0038).
 *
 * The version is a hash of the shell list plus the bytes of `index.html`. Every
 * other output carries a content hash in its own name, so the list already
 * changes when any of them do; `index.html` is the one file whose name is
 * stable and whose contents are not.
 *
 * What that deliberately leaves out: the files copied verbatim from `public/` —
 * the manifest, the icons, the favicon. Vite copies them straight to `dist/`
 * without passing through the bundle, so they cannot be read from here without
 * reaching for `node:fs`. They are picked up by the worker's runtime cache
 * instead, which lives in the same versioned bucket and is therefore dropped on
 * every new build. The cost is one edge: changing only an icon, with no source
 * change anywhere else, produces the same version and the old icon stays until
 * the next real build.
 */
function serviceWorker(): Plugin {
  return {
    name: 'cabas:service-worker',
    apply: 'build',

    // After Vite's own plugins: `index.html` is emitted by one of them, and a
    // hook that runs first sees a bundle with the JS in it and no page.
    generateBundle: {
      order: 'post',
      handler(_options, bundle) {
        const worker = bundle['sw.js'];
        if (worker === undefined || worker.type !== 'chunk') {
          throw new Error('sw.js is missing from the bundle — check rollupOptions.input');
        }

        // Registered as a classic script, because module service workers are
        // too recent to rely on across the iOS versions this has to run on.
        // The output format is ES, so that only holds while the file has no
        // imports and no exports — which it has no reason to acquire, and
        // which would fail silently at registration if it did.
        const selfContained =
          worker.imports.length === 0 &&
          worker.dynamicImports.length === 0 &&
          worker.exports.length === 0;
        if (!selfContained) {
          throw new Error('the service worker must stay self-contained: no imports, no exports');
        }

        const index = bundle['index.html'];
        if (index === undefined || index.type !== 'asset') {
          throw new Error('index.html is missing from the bundle');
        }

        // Source maps are a development aid, not part of the app, and
        // precaching them would put megabytes on a phone that never reads them.
        const shell = Object.keys(bundle)
          .filter((name) => name !== 'sw.js' && !name.endsWith('.map'))
          .map((name) => (name === 'index.html' ? '/' : `/${name}`))
          .sort();

        const build = {
          version: hash(`${shell.join('\n')}\n${String(index.source)}`),
          shell,
        };

        // `JSON.stringify` twice: once for the data the worker parses, once to
        // turn it into the source of a string literal, escapes and all.
        const replaced = worker.code.replace(BUILD_TOKEN, JSON.stringify(JSON.stringify(build)));
        if (replaced === worker.code) {
          throw new Error('the service worker no longer carries its __CABAS_BUILD__ placeholder');
        }
        worker.code = replaced;

        this.info(`service worker: ${shell.length} files precached, version ${build.version}`);
      },
    },
  };
}

/**
 * FNV-1a, 32 bits, as eight hex digits.
 *
 * A cache name only has to change when the input does; it is not a checksum and
 * nothing verifies anything against it. Written out rather than imported so
 * this file keeps its promise about Node APIs — `node:crypto` would break it.
 */
function hash(input: string): string {
  let value = 0x811c9dc5;
  for (let i = 0; i < input.length; i++) {
    value ^= input.charCodeAt(i);
    // The FNV prime, by shifts: `value * 16777619` overflows a double's exact
    // integer range and stops being the same function.
    value = (value + (value << 1) + (value << 4) + (value << 7) + (value << 8) + (value << 24)) >>> 0;
  }
  return value.toString(16).padStart(8, '0');
}

export default defineConfig({
  plugins: [svelte(), serviceWorker()],

  server: {
    // Bound to every interface so the phone on the same wifi can load the dev
    // server. Testing this on a desktop browser only is how the iOS-specific
    // half of M4 gets discovered late.
    host: true,
  },

  build: {
    // Safari on an iPhone that still gets updates handles ES2022. Going lower
    // costs bundle size for devices this app does not target.
    target: 'es2022',
    sourcemap: true,

    rollupOptions: {
      // Two entries: the page, and the service worker. The worker has to land
      // at the root as `/sw.js` — a worker's scope is the directory it is
      // served from, and one under `/assets/` could not control the app.
      input: { app: 'index.html', sw: 'src/sw.js' },
      output: {
        entryFileNames: (chunk) =>
          chunk.name === 'sw' ? 'sw.js' : 'assets/[name]-[hash].js',
      },
    },
  },
});
