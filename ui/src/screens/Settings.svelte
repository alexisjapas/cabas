<script lang="ts">
  import Qr from '../components/Qr.svelte';
  import Screen from '../components/Screen.svelte';
  import { readIdentity } from '../lib/core';
  import type { Session } from '../lib/session.svelte';
  import type { SyncPhase } from '../lib/sync.svelte';
  import Events from './Events.svelte';
  import Pairing from './Pairing.svelte';
  import People from './People.svelte';

  let { session }: { session: Session } = $props();

  /**
   * Three views behind one tab, the way Recipes has three of its own. The
   * roster and the log are screens rather than sections: each has something
   * to say at the bottom that a section would bury, and both are reached from
   * the one place someone would look for them.
   */
  let showing = $state<'settings' | 'people' | 'events'>('settings');

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

  /**
   * The connection, in words. Frontend vocabulary rather than a core tag, so
   * it lives here and not in `labels.ts` (DECISIONS 0035).
   */
  const PHASES: Record<SyncPhase, string> = {
    unpaired: 'Cet appareil est seul',
    idle: 'En veille',
    connecting: 'Connexion…',
    online: 'Synchronisé',
    retrying: 'Hors de portée — nouvelle tentative',
    refused: 'Refusé par le serveur',
  };

  /** Shown on demand, never by default: it is the key, and a settings screen
   *  gets left open on a table (DECISIONS 0021). */
  let revealed = $state(false);
  let pairingOpen = $state(false);

  /** Same shape as the name field: a draft that wins once typing starts, and
   *  the stored value until then. */
  let relayDraft = $state<string | null>(null);
  let relay = $derived(relayDraft ?? session.sync.family?.relay ?? '');

  function saveRelay(event: SubmitEvent): void {
    event.preventDefault();
    const family = session.sync.family;
    if (family === null) return;
    const trimmed = relay.trim();
    session.sync.pair({ phrase: family.phrase, relay: trimmed === '' ? null : trimmed });
    relayDraft = null;
  }

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

{#if showing === 'people'}
  <People {session} onback={() => (showing = 'settings')} />
{:else if showing === 'events'}
  <Events {session} onback={() => (showing = 'settings')} />
{:else}
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

    <section class="family">
      <h2>Famille</h2>
      <p class="status" data-phase={session.sync.phase}>{PHASES[session.sync.phase]}</p>

      {#if session.sync.family === null}
        {#if pairingOpen}
          <Pairing
            onpaired={(family) => {
              session.sync.pair(family);
              pairingOpen = false;
            }}
            oncancel={() => (pairingOpen = false)}
          />
        {:else}
          <p class="note">
            Cet appareil n'est appairé à aucun autre. Tout fonctionne, rien n'est partagé.
          </p>
          <button type="button" onclick={() => (pairingOpen = true)}>Appairer cet appareil</button>
        {/if}
      {:else}
        {@const family = session.sync.family}
        <p class="note">
          Pour ajouter un appareil : ouvrez cabas dessus, choisissez « Rejoindre une famille », et
          recopiez ces douze mots.
        </p>

        {#if revealed}
          <p class="phrase" data-phrase>{family.phrase}</p>
          <Qr text={family.phrase} label="La phrase de votre famille, en QR code" />
          <button type="button" class="secondary" onclick={() => (revealed = false)}>Masquer</button>
        {:else}
          <button type="button" class="secondary" onclick={() => (revealed = true)}>
            Afficher la phrase
          </button>
        {/if}

        <form onsubmit={saveRelay}>
          <label>
            Serveur
            <input
              value={relay}
              oninput={(event) => (relayDraft = event.currentTarget.value)}
              autocapitalize="none"
              autocomplete="off"
              spellcheck="false"
              placeholder="cet appareil parle au serveur qui l'héberge"
            />
            <small>À laisser vide, sauf en développement.</small>
          </label>
          <button type="submit" disabled={relayDraft === null}>Enregistrer le serveur</button>
        </form>
      {/if}
    </section>

    <div class="elsewhere">
      <button type="button" class="secondary" onclick={() => (showing = 'people')}>
        Personnes et appareils
      </button>
      <button type="button" class="secondary" onclick={() => (showing = 'events')}>
        Journal
      </button>
    </div>

    <p class="note">Tout est enregistré sur cet appareil et fonctionne sans réseau.</p>
  </Screen>
{/if}

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

  .family {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    margin-bottom: var(--space-5);
    padding: var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
  }

  h2 {
    margin: 0;
    font-size: var(--text-lg);
  }

  .status {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .family .note {
    margin: 0;
  }

  .family form {
    margin: 0;
    gap: var(--space-3);
  }

  .phrase {
    margin: 0;
    padding: var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    font-family: var(--font-numeric);
    line-height: var(--leading-normal);
    word-spacing: var(--space-2);
    -webkit-user-select: all;
    user-select: all;
  }

  .secondary {
    border: 1px solid var(--border-strong);
    background: var(--surface-raised);
    color: var(--text);
  }

  .elsewhere {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-bottom: var(--space-5);
  }
</style>
