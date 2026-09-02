<script lang="ts">
  import Qr from '../components/Qr.svelte';
  import Screen from '../components/Screen.svelte';
  import { buildVersion, readIdentity } from '../lib/core';
  import type { Session } from '../lib/session.svelte';
  import type { PhotoPhase } from '../lib/photos.svelte';
  import type { SyncPhase } from '../lib/sync.svelte';
  import Events from './Events.svelte';
  import Identify from './Identify.svelte';
  import Pairing from './Pairing.svelte';
  import People from './People.svelte';
  import Shops from './Shops.svelte';
  import Transfer from './Transfer.svelte';

  let { session }: { session: Session } = $props();

  /**
   * Six views behind one tab, where Recipes has three of its own. The roster,
   * the log, the shops, the file door and the user picker are screens rather
   * than sections: each has something to say at the bottom that a section
   * would bury, and all of them are reached from the one place someone would
   * look for them.
   */
  let showing = $state<'settings' | 'people' | 'shops' | 'events' | 'transfer' | 'identify'>(
    'settings',
  );

  /**
   * The device half of the identity never appears in a view-model: it is a
   * fact about this device, not about the group document (DECISIONS 0031).
   * `localStorage` is where it lives, so `localStorage` is where this reads
   * it. Read once — it cannot change while the app is running.
   */
  const identity = readIdentity();

  /**
   * Which build this is. Read once for the same reason as the identity, and
   * from the core rather than from a constant here, because the core is what
   * the relay compiled in (0048) — a bundle and a wasm module that disagreed
   * would be the one thing this line exists to reveal.
   */
  const version = buildVersion();

  /**
   * The field follows the name in the document until somebody starts typing,
   * and their draft wins from then on. Seeding a `$state` from the view once
   * would look simpler and would quietly ignore a rename arriving from
   * another device while this screen is open.
   */
  let edited = $state<string | null>(null);
  let name = $derived(edited ?? session.state.me?.name ?? '');
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

  /**
   * The photo socket says its own piece (DECISIONS 0092).
   *
   * A second line rather than a word folded into the first, because the two
   * are genuinely independent: the list can be synchronised while a hundred
   * megabytes of pictures are still crossing, and "Synchronisé" would be true
   * and misleading. It is also the only place a photo transfer that keeps
   * failing is visible at all — nothing else on screen waits for one.
   */
  const PHOTO_PHASES: Record<PhotoPhase, string> = {
    idle: 'Photos : en veille',
    connecting: 'Photos : connexion…',
    running: 'Photos : transfert',
    done: 'Photos : à jour',
    retrying: 'Photos : hors de portée',
    refused: 'Photos : refusées par le serveur',
  };

  /** Shown on demand, never by default: it is the key, and a settings screen
   *  gets left open on a table (DECISIONS 0021). */
  let revealed = $state(false);
  let pairingOpen = $state(false);

  /** Same shape as the name field: a draft that wins once typing starts, and
   *  the stored value until then. */
  let relayDraft = $state<string | null>(null);
  let relay = $derived(relayDraft ?? session.sync.group?.relay ?? '');

  function saveRelay(event: SubmitEvent): void {
    event.preventDefault();
    const group = session.sync.group;
    if (group === null) return;
    const trimmed = relay.trim();
    session.sync.pair({ phrase: group.phrase, relay: trimmed === '' ? null : trimmed });
    relayDraft = null;
  }

  function rename(event: SubmitEvent): void {
    event.preventDefault();
    const trimmed = name.trim();
    if (trimmed === '' || trimmed === session.state.me?.name) return;
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
{:else if showing === 'shops'}
  <Shops {session} onback={() => (showing = 'settings')} />
{:else if showing === 'events'}
  <Events {session} onback={() => (showing = 'settings')} />
{:else if showing === 'transfer'}
  <Transfer {session} onback={() => (showing = 'settings')} />
{:else if showing === 'identify'}
  <!-- The same screen the first launch shows, minus the device question: the
       device is already in the roster and keeps its name (DECISIONS 0068). -->
  <Identify {session} oncancel={() => (showing = 'settings')} />
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
      <button type="submit" disabled={name.trim() === '' || name.trim() === session.state.me?.name}>
        {saved ? 'Enregistré' : 'Enregistrer'}
      </button>
    </form>

    <!-- Renaming and changing person are two different acts, and the
         difference matters: the first corrects a label everyone sees, the
         second says this phone is now somebody else's (DECISIONS 0068). -->
    <button type="button" class="secondary switch" onclick={() => (showing = 'identify')}>
      Changer d'utilisateur
    </button>

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

    <section class="group">
      <h2 class="display">Groupe</h2>
      <p class="status" data-phase={session.sync.phase}>{PHASES[session.sync.phase]}</p>
      {#if session.sync.group !== null}
        <p class="photo-status" data-phase={session.photos.phase}>
          {PHOTO_PHASES[session.photos.phase]}{#if session.photos.pending > 0}&nbsp;— {session.photos
              .pending} en attente{/if}
        </p>
      {/if}

      {#if session.sync.group === null}
        {#if pairingOpen}
          <Pairing
            onpaired={(group) => {
              session.sync.pair(group);
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
        {@const group = session.sync.group}
        <p class="note">
          Pour ajouter un appareil : ouvrez cabas dessus, choisissez « Rejoindre un groupe », et
          recopiez ces douze mots.
        </p>

        {#if revealed}
          <p class="phrase" data-phrase>{group.phrase}</p>
          <Qr text={group.phrase} label="La phrase de votre groupe, en QR code" />
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
      <button type="button" class="secondary" onclick={() => (showing = 'shops')}>
        Magasins
      </button>
      <button type="button" class="secondary" onclick={() => (showing = 'events')}>
        Journal
      </button>
      <button type="button" class="secondary" onclick={() => (showing = 'transfer')}>
        Données
      </button>
    </div>

    <p class="note bubble">Tout est enregistré sur cet appareil et fonctionne sans réseau.</p>

    <p class="note build bubble" data-version={version}>
      Version {version}. Une mise à jour s'installe en arrière-plan et s'applique à l'ouverture
      suivante.
    </p>
  </Screen>
{/if}

<style>
  /* A form is a lilac tile and its fields are cream (DECISIONS 0081). */
  form {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    margin-bottom: var(--space-4);
    padding: var(--space-4);
    border: 2px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
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
    border: 2px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--bubble);
    font-weight: var(--weight-normal);
  }

  small {
    color: var(--text-muted);
    font-weight: var(--weight-normal);
  }

  button {
    padding: var(--space-3);
    border: 0;
    border-radius: var(--radius-pill);
    background: var(--accent);
    color: var(--on-accent);
    font-weight: var(--weight-bold);
    box-shadow: var(--shadow-sm);
    cursor: pointer;
  }

  /* A disabled primary is a sunken pill in muted ink, not a half-transparent
     orange one: over a lilac tile the alpha turns the label to mud
     (DECISIONS 0081). */
  button:disabled {
    background: var(--surface-sunken);
    color: var(--text-muted);
    box-shadow: none;
    cursor: default;
  }

  /* Facts, on cream: this is read, not pressed. */
  dl {
    margin: 0 0 var(--space-5);
    padding: 0 var(--space-3);
    border-radius: var(--radius-md);
    background: var(--bubble);
  }

  dl div {
    display: flex;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-3) 0;
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
    font-family: var(--font-numeric);
    font-variant-numeric: tabular-nums;
  }

  .note {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    margin-bottom: var(--space-5);
    padding: var(--space-4);
    border: 2px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
  }

  /* `--text` and not `--display-ink`: the olive is prescribed for the display
     face on anis or apricot, and this heading sits on a lilac tile. */
  h2 {
    margin: 0;
    color: var(--text);
    font-size: var(--text-lg);
  }

  .status {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  /* Quieter than the line above it: the document is what has to be up to
     date, and a photo still crossing is a fact rather than a problem. */
  .photo-status {
    margin: 0;
    color: var(--text-faint);
    font-size: var(--text-xs);
  }

  .group .note {
    margin: 0;
  }

  .build {
    font-size: var(--text-xs);
  }

  .group form {
    margin: 0;
    padding: 0;
    border: 0;
    background: none;
    gap: var(--space-3);
  }

  .phrase {
    margin: 0;
    padding: var(--space-3);
    border: 2px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    font-family: var(--font-numeric);
    line-height: var(--leading-normal);
    word-spacing: var(--space-2);
    -webkit-user-select: all;
    user-select: all;
  }

  .secondary {
    border: 2px solid var(--border-strong);
    background: var(--surface-raised);
    color: var(--text);
    box-shadow: none;
  }

  .switch {
    width: 100%;
    margin-bottom: var(--space-5);
  }

  /* `Pairing` is a tile of its own, and this group is already one: nesting
     the two draws a border inside a border. It is flattened here rather than
     given a prop, because the difference is entirely this screen's. */
  .group :global(.pairing) {
    padding: 0;
    border: 0;
    background: none;
  }

  .elsewhere {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-bottom: var(--space-5);
  }
</style>

