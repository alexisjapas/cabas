<script lang="ts">
  import Screen from '../components/Screen.svelte';
  import { readIdentity } from '../lib/core';
  import type { Session } from '../lib/session.svelte';

  let { session }: { session: Session } = $props();

  /**
   * The device half of the identity never appears in a view-model: it is a
   * fact about this device, not about the family document (DECISIONS 0031).
   * `localStorage` is where it lives, so `localStorage` is where this reads
   * it. Read once — it cannot change while the app is running.
   */
  const identity = readIdentity();

  /**
   * The field follows the name in the document until somebody starts typing,
   * and their draft wins from then on. Seeding a `$state` from the view once
   * would look simpler and would quietly ignore a rename arriving from
   * another device while this screen is open.
   */
  let edited = $state<string | null>(null);
  let name = $derived(edited ?? session.state.me.name);
  let saved = $state(false);

  function rename(event: SubmitEvent): void {
    event.preventDefault();
    const trimmed = name.trim();
    if (trimmed === '' || trimmed === session.state.me.name) return;
    // Attribution is a label, so this changes what future entries are signed
    // with and nothing about what anyone is allowed to do (Rule 7).
    if (session.run({ command: 'rename_user', name: trimmed })) {
      edited = null;
      saved = true;
      setTimeout(() => (saved = false), 2000);
    }
  }
</script>

<Screen title="Réglages">
  <form onsubmit={rename}>
    <label>
      Votre prénom
      <input
        value={name}
        oninput={(event) => (edited = event.currentTarget.value)}
        required
        autocomplete="given-name"
      />
      <small>Ce que voient les autres appareils à côté de ce que vous ajoutez ou cochez.</small>
    </label>
    <button type="submit" disabled={name.trim() === '' || name.trim() === session.state.me.name}>
      {saved ? 'Enregistré' : 'Enregistrer'}
    </button>
  </form>

  <dl>
    <div>
      <dt>Cet appareil</dt>
      <dd>{identity?.device_name ?? '—'}</dd>
    </div>
    <div>
      <dt>Recettes</dt>
      <dd>{session.state.recipes.length}</dd>
    </div>
    <div>
      <dt>Ingrédients</dt>
      <dd>{session.state.ingredients.length}</dd>
    </div>
    <div>
      <dt>Entrées sur la liste</dt>
      <dd>{session.state.list.length}</dd>
    </div>
  </dl>

  <p class="note">
    Tout est enregistré sur cet appareil et fonctionne sans réseau. La synchronisation entre
    appareils arrive plus tard.
  </p>
</Screen>

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    margin-bottom: var(--space-6);
  }

  label {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  input {
    padding: var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    font-weight: var(--weight-normal);
  }

  small {
    color: var(--text-muted);
    font-weight: var(--weight-normal);
  }

  button {
    padding: var(--space-3);
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--on-accent);
    font-weight: var(--weight-semibold);
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.5;
    cursor: default;
  }

  dl {
    margin: 0 0 var(--space-5);
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
  }

  dl div {
    display: flex;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-3);
  }

  dl div + div {
    border-top: 1px solid var(--border);
  }

  dt {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  dd {
    margin: 0;
    font-variant-numeric: tabular-nums;
  }

  .note {
    color: var(--text-faint);
    font-size: var(--text-sm);
  }
</style>
