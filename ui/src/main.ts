import { mount } from 'svelte';

import App from './App.svelte';
import './app.css';

const target = document.getElementById('app');
if (!target) {
  throw new Error('#app is missing from index.html');
}

// The service worker is what makes the app installable and what makes it open
// with no signal (DECISIONS 0038). Nothing waits on it: registration is a
// background concern and the app has already rendered from IndexedDB by the
// time it resolves (Rule 6).
//
// Production only. In `vite dev` there is no `/sw.js` to register, and a
// worker left over from a preview run would serve a stale module graph over
// the one the dev server is rebuilding.
//
// `updateViaCache: 'none'` keeps the browser from answering the update check
// out of its HTTP cache: the worker's own bytes are the only signal that a new
// build exists, and a cached copy of them says there is not one.
if (import.meta.env.PROD && 'serviceWorker' in navigator) {
  void navigator.serviceWorker.register('/sw.js', { updateViaCache: 'none' });
}

export default mount(App, { target });
