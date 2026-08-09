<script lang="ts">
  /**
   * Who is in the family, on what, and the one thing this screen has to be
   * honest about.
   *
   * It is the screen that looks most like an access control panel and is the
   * furthest thing from one (Rule 7, DECISIONS 0024): a single key decrypts
   * the whole document, every paired device holds it, and these names are
   * labels on edits rather than credentials. So it says so, in the place
   * someone would come looking to remove a device — and tells them the truth
   * about what removing one actually costs.
   */
  import Qr from '../components/Qr.svelte';
  import Screen from '../components/Screen.svelte';
  import { mintPhrase } from '../lib/core';
  import { byName, relativeTime } from '../lib/format';
  import type { Session } from '../lib/session.svelte';

  let { session, onback }: { session: Session; onback: () => void } = $props();

  /** Sorted for a reader; the core sorts by id, which is stable, not legible. */
  let people = $derived([...session.state.people].sort(byName));

  type Rotation =
    | { at: 'idle' }
    /** The consequences, on screen, before anything happens. */
    | { at: 'asking' }
    | { at: 'done'; phrase: string }
    | { at: 'failed'; message: string };

  let rotation = $state<Rotation>({ at: 'idle' });

  /**
   * Rotating is the whole of revocation, and it is not a small act: this
   * device leaves for a family only it knows the phrase of, taking its copy
   * of the library with it, and every other device stays where it was until
   * somebody types the new words into it.
   *
   * The old family's log stays on the relay, sealed, and the lost device keeps
   * whatever it had already. What it stops getting is everything after this.
   */
  async function rotate(): Promise<void> {
    const family = session.sync.family;
    if (family === null) return;
    try {
      const phrase = await mintPhrase();
      session.sync.pair({ phrase, relay: family.relay });
      rotation = { at: 'done', phrase };
    } catch (cause) {
      rotation = {
        at: 'failed',
        message: cause instanceof Error ? cause.message : String(cause),
      };
    }
  }
</script>

<Screen title="Personnes et appareils">
  {#snippet actions()}
    <button type="button" class="back" onclick={onback}>Retour</button>
  {/snippet}

  <ul class="people">
    {#each people as person (person.id)}
      <li>
        <p class="name">
          {person.name}
          {#if person.is_me}<span class="tag">vous</span>{/if}
        </p>
        {#if person.devices.length === 0}
          <p class="empty">Aucun appareil.</p>
        {:else}
          <ul class="devices">
            {#each person.devices as device (device.id)}
              <li>
                <span class="device-name">{device.name}</span>
                {#if device.is_this_one}<span class="tag">cet appareil</span>{/if}
                <span class="when">appairé {relativeTime(device.paired_at)}</span>
              </li>
            {/each}
          </ul>
        {/if}
      </li>
    {/each}
  </ul>

  <p class="note">
    Ces noms disent qui a probablement fait quoi. Ce ne sont pas des comptes : la clé de la famille
    est la même pour tout le monde, donc n'importe quel appareil appairé peut tout lire et tout
    écrire, sous n'importe quel nom.
  </p>

  <section class="revoke">
    <h2>Retirer un appareil</h2>

    {#if rotation.at === 'done'}
      <p>Voici la nouvelle phrase. Saisissez-la sur les appareils que vous gardez.</p>
      <p class="phrase" data-phrase>{rotation.phrase}</p>
      <Qr text={rotation.phrase} label="La nouvelle phrase de votre famille, en QR code" />
      <p class="note">
        Cet appareil est déjà passé à la nouvelle famille, avec tout son contenu. Les autres
        continuent sur l'ancienne tant qu'ils n'ont pas cette phrase.
      </p>
    {:else if rotation.at === 'failed'}
      <p class="problem" role="alert">{rotation.message}</p>
      <button type="button" onclick={() => (rotation = { at: 'idle' })}>Réessayer</button>
    {:else}
      <p>
        Il n'y a pas de moyen de retirer un seul appareil : ils partagent tous la même clé. La seule
        chose possible est d'en changer, et de réappairer ceux que vous gardez.
      </p>

      {#if rotation.at === 'asking'}
        <ul class="consequences">
          <li>Cet appareil passe à une nouvelle phrase et emporte tout ce qu'il a.</li>
          <li>
            Chaque autre appareil devra saisir cette phrase. Tant qu'il ne l'a pas, il continue tout
            seul de son côté et ne voit plus rien de ce qui se passe ici.
          </li>
          <li>
            L'appareil perdu garde ce qu'il avait déjà — rien ne peut l'en effacer à distance. Il
            n'apprendra simplement plus rien de nouveau.
          </li>
          <li>
            L'ancien journal reste sur le serveur, scellé et illisible. Rien ne l'efface tout seul —
            c'est une commande à lancer sur le serveur, une fois que tout le monde aura la nouvelle
            phrase.
          </li>
        </ul>
        <div class="pair-of-buttons">
          <button type="button" class="secondary" onclick={() => (rotation = { at: 'idle' })}>
            Annuler
          </button>
          <button type="button" class="danger" onclick={rotate}>Changer la phrase</button>
        </div>
      {:else if session.sync.family !== null}
        <button type="button" class="secondary" onclick={() => (rotation = { at: 'asking' })}>
          Changer la phrase de la famille
        </button>
      {/if}
    {/if}
  </section>
</Screen>

<style>
  .back {
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font-size: var(--text-sm);
    cursor: pointer;
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .people > li {
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
  }

  .people > li + li {
    margin-top: var(--space-3);
  }

  .name {
    margin: 0;
    font-weight: var(--weight-semibold);
  }

  .devices {
    margin-top: var(--space-2);
  }

  .devices li {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--space-2);
    padding: var(--space-1) 0;
  }

  .device-name {
    font-size: var(--text-sm);
  }

  .when {
    color: var(--text-faint);
    font-size: var(--text-xs);
  }

  .empty {
    margin: var(--space-1) 0 0;
    color: var(--text-faint);
    font-size: var(--text-sm);
  }

  .tag {
    padding: 0 var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--accent-soft);
    color: var(--accent);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
  }

  .note {
    margin: var(--space-4) 0 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .revoke {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    margin-top: var(--space-5);
    padding: var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
  }

  .revoke h2 {
    margin: 0;
    font-size: var(--text-lg);
  }

  .revoke p {
    margin: 0;
    font-size: var(--text-sm);
  }

  .consequences {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding-left: var(--space-4);
    color: var(--text-muted);
    font-size: var(--text-sm);
    list-style: disc;
  }

  .pair-of-buttons {
    display: flex;
    gap: var(--space-3);
  }

  .pair-of-buttons button {
    flex: 1;
  }

  button {
    padding: var(--space-3);
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--on-accent);
    font-size: var(--text-base);
    font-weight: var(--weight-semibold);
    cursor: pointer;
  }

  .secondary {
    border: 1px solid var(--border-strong);
    background: var(--surface-raised);
    color: var(--text);
  }

  .danger {
    background: var(--danger);
    color: var(--on-danger);
  }

  .phrase {
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

  .problem {
    color: var(--danger);
  }
</style>
