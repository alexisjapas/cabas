import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/**
 * Svelte 5, compiled by Vite. There is no SvelteKit here on purpose
 * (DECISIONS 0037): nothing is rendered on a server, and the current screen is
 * persisted state rather than a URL.
 *
 * `vitePreprocess` is what makes `<script lang="ts">` work — it hands the
 * block to Vite's own TypeScript transform.
 *
 * @type {import('@sveltejs/vite-plugin-svelte').SvelteConfig}
 */
export default {
  preprocess: vitePreprocess(),
};
