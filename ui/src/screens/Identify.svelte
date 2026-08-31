<script lang="ts">
  /**
   * "Qui êtes-vous ?" — the screen between joining a group and using it
   * (DECISIONS 0068).
   *
   * A device knows what it is from the moment it exists. Who is carrying it
   * is a row on the group's roster, and the roster is in the document, which
   * arrives over the network. So this screen is shown *over* a running app:
   * the replica is open, sync is connected, and the list of members fills in
   * underneath the question as the first frames land.
   *
   * # It never waits, and it never pretends
   *
   * Rule 6 says no user action waits on the network, and that holds here even
   * though the useful answer comes from there. The roster is whatever has
   * arrived; "créer" is always available, and on a brand-new group it is the
   * only thing there is. What the screen must not do is imply that an empty
   * roster means the group is empty — hence the line about the connection,
   * which reads the sync phase rather than guessing.
   *
   * # Two questions, in this order
   *
   * Who, then which device. That ordering is not cosmetic: the device record
   * cannot exist before it has an owner, so naming the device first would
   * write nothing, and naming it after the owner is chosen would write the
   * record twice — once with an empty name, which is the version the other
   * phone would see. `name_device` before `choose_user`/`create_user` writes
   * it once, already named.
   */
  import Screen from '../components/Screen.svelte';
  import SearchField from '../components/SearchField.svelte';
  import type { Command } from '../lib/bindings/Command';
  import { byName, matches } from '../lib/format';
  import type { Session } from '../lib/session.svelte';
  import type { SyncPhase } from '../lib/sync.svelte';

  let {
    session,
    /** Given when this is a change of user rather than a first launch. */
    oncancel = undefined,
  }: { session: Session; oncancel?: (() => void) | undefined } = $props();

  /** The people already in the group, as far as this device knows. */
  let people = $derived([...session.state.people].sort(byName));

  /** Naming the device is only asked on the way in, not on a handover. */
  let arriving = $derived(session.state.me === null);

  /**
   * The two ways to answer "qui êtes-vous ?", carried as the command each one
   * is rather than as a shape to be turned into one later.
   */
  type Chosen = Extract<Command, { command: 'choose_user' | 'create_user' }>;

  type Step =
    | { at: 'who' }
    /** Somebody was chosen; the device still needs a name. */
    | { at: 'device'; user: Chosen };

  let step = $state<Step>({ at: 'who' });
  let query = $state('');
  let creating = $state(false);
  let newName = $state('');
  let deviceName = $state(guessDeviceName());

  let shown = $derived(people.filter((person) => matches(person.name, query)));

  /**
   * What the connection is doing, in the two words this screen needs. It is
   * not the sync indicator — it answers one question, "is this list all of
   * them", and only while the answer might be no.
   */
  const WAITING: Partial<Record<SyncPhase, string>> = {
    connecting: 'Connexion au groupe…',
    retrying: 'Groupe hors de portée — nouvelle tentative.',
    refused: 'Le serveur a refusé la connexion.',
    idle: 'En attente de synchronisation.',
  };
  let waiting = $derived(WAITING[session.sync.phase]);

  /** A first guess, so the last field is usually a no-op. */
  function guessDeviceName(): string {
    const agent = navigator.userAgent;
    if (/iPhone/.test(agent)) return 'iPhone';
    if (/iPad/.test(agent)) return 'iPad';
    if (/Android/.test(agent)) return 'Téléphone Android';
    return 'Ordinateur';
  }

  /** Chosen off the roster, or typed into the field under it. */
  function pick(user: Chosen): void {
    // On a device that is already in the group, the name is the only question
    // — its record is written and keeps the name it has.
    if (!arriving) {
      apply(user, null);
      return;
    }
    step = { at: 'device', user };
  }

  function nameDevice(event: SubmitEvent): void {
    event.preventDefault();
    if (step.at !== 'device' || deviceName.trim() === '') return;
    apply(step.user, deviceName.trim());
  }

  /**
   * The device's name first, then the person: that is what writes the device
   * record once, with its name already on it.
   *
   * Both move an identity whose only durable copy is the host's (DECISIONS
   * 0031); `run` writes it back for exactly these commands, which is why
   * neither call has to remember to.
   */
  function apply(user: Chosen, device: string | null): void {
    if (device !== null && !session.run({ command: 'name_device', name: device })) return;
    if (!session.run(user)) {
      // The core refused — the person was deleted from another device between
      // the render and the tap. Back to a roster that no longer has them.
      step = { at: 'who' };
      return;
    }
    oncancel?.();
  }

  function create(event: SubmitEvent): void {
    event.preventDefault();
    if (newName.trim() === '') return;
    pick({ command: 'create_user', name: newName.trim() });
  }
</script>

{#snippet who()}
  <p class="lead bubble">
    {#if arriving}
      Ce groupe est partagé entre plusieurs personnes. Dites laquelle vous êtes, pour que les
      autres sachent qui a ajouté quoi.
    {:else}
      Cet appareil signera ce que vous ajoutez au nom de la personne choisie.
    {/if}
  </p>

  {#if people.length > 3}
    <SearchField bind:value={query} placeholder="Chercher quelqu'un…" />
  {/if}

  {#if shown.length > 0}
    <ul class="people">
      {#each shown as person (person.id)}
        <li>
          <button type="button" class:current={person.is_me} onclick={() => pick({ command: 'choose_user', user: person.id })}>
            <span class="name">{person.name}</span>
            {#if person.is_me}<span class="tag">vous</span>{/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  {#if creating}
    <form onsubmit={create}>
      <label>
        Votre prénom
        <input
          bind:value={newName}
          data-field="new-user"
          required
          autocomplete="given-name"
          enterkeyhint="done"
          placeholder="Alexis"
        />
      </label>
      <div class="buttons">
        <button type="submit" disabled={newName.trim() === ''}>Continuer</button>
        <button type="button" class="secondary" onclick={() => (creating = false)}>Annuler</button>
      </div>
    </form>
  {:else}
    <button type="button" class="create" onclick={() => (creating = true)}>
      + {people.length === 0 ? 'Créer un utilisateur' : "Je n'y suis pas — me créer"}
    </button>
  {/if}

  {#if waiting !== undefined}
    <p class="note bubble" data-waiting>
      {waiting}
      {#if people.length === 0}
        La liste des membres arrive avec la première synchronisation.
      {/if}
    </p>
  {/if}
{/snippet}

{#snippet device()}
  <p class="lead bubble">Pour distinguer vos appareils dans la liste des membres.</p>
  <form onsubmit={nameDevice}>
    <label>
      Nom de cet appareil
      <input
        bind:value={deviceName}
        data-field="device-name"
        required
        enterkeyhint="done"
        autocomplete="off"
      />
    </label>
    <div class="buttons">
      <button type="submit" disabled={deviceName.trim() === ''}>Terminer</button>
      <button type="button" class="secondary" onclick={() => (step = { at: 'who' })}>Retour</button>
    </div>
  </form>
{/snippet}

{#if arriving}
  <!-- No frame of its own: the first launch renders this inside `main.first`,
       which is the same page shape pairing has (App.svelte). -->
  <h1 class="display">{step.at === 'who' ? 'Qui êtes-vous ?' : 'Cet appareil'}</h1>
  {#if step.at === 'who'}{@render who()}{:else}{@render device()}{/if}
{:else}
  <Screen title="Changer d'utilisateur" onback={() => oncancel?.()}>
    {@render who()}
  </Screen>
{/if}

<style>
  h1 {
    color: var(--display-ink);
    font-size: var(--text-2xl);
  }

  .lead {
    margin: var(--space-3) 0 var(--space-4);
    color: var(--text-muted);
  }

  /* The roster: read, so cream (DECISIONS 0081). */
  ul {
    margin: 0 0 var(--space-4);
    padding: 0;
    list-style: none;
    border-radius: var(--radius-md);
    background: var(--bubble);
    overflow: hidden;
  }

  li + li {
    border-top: 1px solid var(--border);
  }

  .people button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3);
    min-height: var(--tapsize);
    border: 0;
    border-radius: 0;
    background: none;
    color: inherit;
    font-size: var(--text-base);
    font-weight: var(--weight-normal);
    text-align: left;
    box-shadow: none;
    cursor: pointer;
  }

  .people button:active {
    background: var(--surface-sunken);
  }

  .people button.current .name {
    font-weight: var(--weight-semibold);
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Anis is a surface; the word on it takes the dark ink. */
  .tag {
    flex: none;
    padding: 0 var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--accent-soft);
    color: var(--on-accent);
    font-size: var(--text-xs);
    font-weight: var(--weight-bold);
  }

  /* A form is a tile and its fields are cream. */
  form {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
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
    font-size: var(--text-base);
    font-weight: var(--weight-normal);
  }

  .buttons {
    display: flex;
    gap: var(--space-2);
  }

  .buttons button {
    flex: 1;
  }

  button {
    padding: var(--space-3);
    border: 0;
    border-radius: var(--radius-pill);
    background: var(--accent);
    color: var(--on-accent);
    font-size: var(--text-base);
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

  .secondary {
    border: 2px solid var(--border-strong);
    background: var(--surface-raised);
    color: var(--text);
    box-shadow: none;
  }

  .create {
    width: 100%;
    border: 2px solid var(--border-strong);
    background: var(--bubble);
    color: var(--accent-strong);
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
    box-shadow: none;
  }

  .note {
    margin: var(--space-5) 0 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }
</style>

