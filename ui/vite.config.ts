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
 * The files under `public/` that have to be in the precache, named by hand.
 *
 * Vite copies `public/` straight to `dist/` without passing it through the
 * bundle, so this plugin cannot see it without `node:fs` — which this file
 * deliberately does not use. Everything else in there (the manifest, the
 * icons, the favicon) is picked up by the worker's runtime cache on the first
 * launch that asks for it, and that is good enough for all of them.
 *
 * The two faces are not. A font fetched at runtime is a font that is absent
 * the first time the app is opened with no signal, and the app falls back to
 * the system face — which is precisely the trip this app exists for
 * (DECISIONS 0081).
 *
 * The cost of naming a file here is that `cache.addAll` rejects as a whole if
 * any one of them 404s, which would leave the app with no precache at all. So
 * these are committed assets, and a rename has to be made here too.
 */
const PUBLIC_SHELL = ['/fonts/shrikhand-400.woff2', '/fonts/quicksand-variable.woff2'];

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
        const shell = [
          ...Object.keys(bundle)
            .filter((name) => name !== 'sw.js' && !name.endsWith('.map'))
            .map((name) => (name === 'index.html' ? '/' : `/${name}`)),
          ...PUBLIC_SHELL,
        ].sort();

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

// The host is chosen by Vite's own `--mode`, and deliberately not by an
// environment variable: reading `process.env` here would be the first Node API
// in this file and would cost it the strict browser tsconfig it is checked
// under (see the note at the top).
export default defineConfig(({ mode }) => {
  // Which host this build is for. Everything below reads this and nothing
  // else, so "what changes under Tauri" is one list in one place.
  const tauri = mode === 'tauri';

  // Two entries: the page, and the service worker. The worker has to land at
  // the root as `/sw.js` — a worker's scope is the directory it is served
  // from, and one under `/assets/` could not control the app.
  //
  // One entry under Tauri, and dropping the plugin below is not enough on its
  // own: this input is what *emits* `sw.js`, while the plugin is what replaces
  // the precache token in it. Leaving the entry would ship a worker whose
  // cache is named after the placeholder — and nothing registers it there,
  // which would make it the quietest possible way to ship that bug.
  //
  // Annotated rather than inferred: without the type the two branches widen to
  // `{ sw?: undefined } | { sw: string }`, which is not a
  // `Record<string, string>` and fails `pnpm check` with a wall of overload
  // text about `UserConfigExport`.
  const input: Record<string, string> = tauri
    ? { app: 'index.html' }
    : { app: 'index.html', sw: 'src/sw.js' };

  return {
    // No service worker under Tauri: the assets are on the device already and
    // the app is installed by the APK, so the worker has nothing to do and its
    // precache would be a second copy of what is beside it (DECISIONS 0093).
    plugins: tauri ? [svelte()] : [svelte(), serviceWorker()],

    resolve: {
      alias: {
        // The one conditional in `ui/` (DECISIONS 0093). `core.ts` is the
        // interface; this decides which implementation is behind it, so no
        // screen, engine or component ever learns which host it runs on.
        // `--mode tauri` selects the Android one; every other mode is the PWA,
        // which is what keeps `dev`, `build` and `preview` unchanged.
        // `tsconfig.json` carries the same mapping for `svelte-check`, and the
        // two are only in step because a human keeps them so.
        '$core-host': tauri ? '/src/lib/core.tauri.ts' : '/src/lib/core.wasm.ts',
      },
    },

    server: {
      // Bound to every interface so the phone on the same wifi can load the
      // dev server. Testing this on a desktop browser only is how the
      // iOS-specific half of M4 gets discovered late.
      host: true,
    },

    build: {
      // A directory of its own for the Tauri bundle, and it is not tidiness.
      // `crates/relay/build.rs` compiles `ui/dist` into the relay binary
      // (DECISIONS 0048), and `cargo tauri android build` runs
      // `beforeBuildCommand` — so a shared output directory means an Android
      // build silently replaces the PWA the relay is about to ship with one
      // that has no wasm and no service worker in it. It builds, it is green,
      // and the phones get a blank page. Two directories make that
      // impossible rather than documented.
      outDir: tauri ? 'dist-tauri' : 'dist',

      // Safari on an iPhone that still gets updates handles ES2022. Going
      // lower costs bundle size for devices this app does not target.
      target: 'es2022',
      sourcemap: true,

      rollupOptions: {
        input,
        output: {
          entryFileNames: (chunk) =>
            chunk.name === 'sw' ? 'sw.js' : 'assets/[name]-[hash].js',
        },
      },
    },
  };
});
