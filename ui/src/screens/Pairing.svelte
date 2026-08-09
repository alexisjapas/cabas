<script lang="ts">
  /**
   * Pairing: start a family, or join one.
   *
   * The phrase is the whole secret (DECISIONS 0042) — key and family id both
   * derive from it — so this screen is the only place it is ever shown, and it
   * says what that means rather than assuming anyone knows.
   *
   * The QR is displayed and never scanned (DECISIONS 0047): the second device
   * joins by typing or pasting the twelve words, which 0021 requires to exist
   * anyway. So the picture is a convenience for reading them across a table,
   * not a mechanism anything depends on.
   *
   * It is used from two places — the first launch, before there is a replica
   * at all, and Settings on a device that already has one. That is why it
   * takes a callback and knows nothing about `Session`.
   */
  import Qr from '../components/Qr.svelte';
  import { mintPhrase, readPhrase } from '../lib/core';
  import type { Family } from '../lib/sync.svelte';

  let {
    onpaired,
    oncancel,
  }: { onpaired: (family: Family) => void; oncancel?: () => void } = $props();

  type Step =
    | { at: 'choose' }
    | { at: 'created'; phrase: string }
    | { at: 'joining' }
    | { at: 'failed'; message: string };

  let step = $state<Step>({ at: 'choose' });
  let typed = $state('');
  let problem = $state<string | null>(null);
  let copied = $state(false);

  async function start(): Promise<void> {
    try {
      step = { at: 'created', phrase: await mintPhrase() };
    } catch (cause) {
      step = { at: 'failed', message: cause instanceof Error ? cause.message : String(cause) };
    }
  }

  /**
   * The words are checked here before anything is stored, so a mistyped phrase
   * fails on this screen instead of looking like a relay that is down.
   *
   * The reason is worked out on this side because the core answers in English
   * and this app writes French (DECISIONS 0035): counting words is input
   * handling, and the core still has the last word on the checksum — which is
   * the part that catches a single swapped letter.
   */
  async function join(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const words = typed.trim().split(/\s+/).filter((word) => word !== '');
    if (words.length !== 12) {
      problem = `Il faut douze mots — il y en a ${words.length}.`;
      return;
    }
    try {
      onpaired({ phrase: await readPhrase(typed), relay: null });
    } catch {
      problem = "Un des mots n'est pas dans la liste, ou il a été mal recopié.";
    }
  }

  async function copy(phrase: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(phrase);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch {
      // Not every context allows it, and the words are on screen regardless.
      problem = 'Copie impossible ici — recopiez les mots à la main.';
    }
  }
</script>

<div class="pairing">
  {#if step.at === 'choose'}
    <h1>cabas</h1>
    <p class="lead">Les courses et les recettes, à deux, hors ligne.</p>
    <div class="choice">
      <button type="button" onclick={start}>Commencer une famille</button>
      <button type="button" class="secondary" onclick={() => (step = { at: 'joining' })}>
        Rejoindre une famille
      </button>
    </div>
    <p class="note">
      Une famille, c'est une liste et des recettes partagées entre vos appareils. Si l'un d'eux en a
      déjà une, rejoignez-la.
    </p>
    {#if oncancel}
      <button type="button" class="quiet" onclick={oncancel}>Annuler</button>
    {/if}
  {:else if step.at === 'created'}
    <h1>Votre phrase</h1>
    <p class="lead">Douze mots. C'est la clé de votre famille, et il n'y en a pas d'autre.</p>

    <p class="phrase" data-phrase>{step.phrase}</p>
    <Qr text={step.phrase} label="La phrase de votre famille, en QR code" />

    <p class="note">
      Notez-la ailleurs que sur cet appareil. Elle déchiffre tout ce que vous partagez, et
      quiconque l'obtient peut lire vos listes. Le serveur, lui, ne peut pas.
    </p>

    <div class="actions">
      <button type="button" class="secondary" onclick={() => copy((step as { phrase: string }).phrase)}>
        {copied ? 'Copié' : 'Copier les mots'}
      </button>
      <button type="button" onclick={() => onpaired({ phrase: (step as { phrase: string }).phrase, relay: null })}>
        J'ai noté la phrase
      </button>
    </div>
  {:else if step.at === 'joining'}
    <h1>Rejoindre</h1>
    <p class="lead">Saisissez les douze mots affichés sur l'autre appareil.</p>

    <form onsubmit={join}>
      <textarea
        bind:value={typed}
        rows="3"
        autocapitalize="none"
        autocomplete="off"
        spellcheck="false"
        placeholder="douze mots séparés par des espaces"
        oninput={() => (problem = null)}
      ></textarea>
      {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
      <button type="submit" disabled={typed.trim() === ''}>Rejoindre</button>
      <button type="button" class="quiet" onclick={() => (step = { at: 'choose' })}>Retour</button>
    </form>
  {:else}
    <h1>Raté</h1>
    <p class="problem" role="alert">{step.message}</p>
    <button type="button" onclick={() => (step = { at: 'choose' })}>Recommencer</button>
  {/if}
</div>

<style>
  .pairing {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  h1 {
    margin: 0;
    font-size: var(--text-2xl);
  }

  .lead {
    margin: 0;
    color: var(--text-muted);
  }

  .choice,
  .actions {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  form {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  /* The words themselves: wide spacing and a monospaced face, because they are
     read aloud across a room and retyped by hand on the other device. */
  .phrase {
    margin: 0;
    padding: var(--space-4);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    font-family: var(--font-numeric);
    font-size: var(--text-lg);
    line-height: var(--leading-normal);
    word-spacing: var(--space-2);
    -webkit-user-select: all;
    user-select: all;
  }

  textarea {
    padding: var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    font-family: var(--font-numeric);
    font-size: var(--text-base);
    resize: vertical;
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

  button:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .secondary {
    border: 1px solid var(--border-strong);
    background: var(--surface-raised);
    color: var(--text);
  }

  .quiet {
    padding: var(--space-2);
    background: none;
    color: var(--text-muted);
    font-weight: var(--weight-normal);
  }

  .note {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .problem {
    margin: 0;
    color: var(--danger);
    font-size: var(--text-sm);
  }
</style>
