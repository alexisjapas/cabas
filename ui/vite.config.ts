import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vite';

// A single-page bundle of static files: no server renders anything, and at M6
// the whole `dist/` is embedded into the relay binary (DECISIONS 0037, 0010).
//
// Nothing here uses a Node API on purpose — that is what lets this file be
// type-checked by the same strict `tsconfig.json` as the browser code, with no
// second config and no `@types/node`.
export default defineConfig({
  plugins: [svelte()],

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
  },
});
