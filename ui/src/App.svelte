<script lang="ts">
  import { onMount } from 'svelte';

  import ErrorBanner from './components/ErrorBanner.svelte';
  import TabBar from './components/TabBar.svelte';
  import { mintDevice, readIdentity, rememberIdentity } from './lib/core';
  import { keyboard } from './lib/keyboard.svelte';
  import { Session } from './lib/session.svelte';
  import Cart from './screens/Cart.svelte';
  import Identify from './screens/Identify.svelte';
  import Ingredients from './screens/Ingredients.svelte';
  import List from './screens/List.svelte';
  import Pairing from './screens/Pairing.svelte';
  import Recipes from './screens/Recipes.svelte';
  import Settings from './screens/Settings.svelte';
  import { rememberGroup, type Group } from './lib/sync.svelte';

  /**
   * Boot, in the order DECISIONS 0031 requires: the device's identity comes
   * out of `localStorage` before anything else can run, and a device that has
   * never launched mints one.
   *
   * A first launch is asked one thing before that — which group this device
   * belongs to. Pairing comes first because joining an existing one is the
   * common case for the *second* phone, and because the twelve words are what
   * make the group's roster reachable at all.
   *
   * **Who is carrying the device is asked afterwards, not here** (DECISIONS
   * 0068). The roster lives in the document and the document arrives over the
   * network, so the question is asked over a running app rather than in front
   * of one — which is why there is no onboarding step in this list any more.
   */
  type Phase =
    | { step: 'loading' }
    | { step: 'pairing' }
    | { step: 'ready'; session: Session }
    | { step: 'failed'; message: string };

  let phase = $state<Phase>({ step: 'loading' });

  function describe(cause: unknown): string {
    return cause instanceof Error ? cause.message : String(cause);
  }

  async function openWith(identity: Parameters<typeof Session.open>[0]): Promise<void> {
    try {
      phase = { step: 'ready', session: await Session.open(identity) };
    } catch (cause) {
      phase = { step: 'failed', message: describe(cause) };
    }
  }

  onMount(() => {
    // Ahead of the identity check, and not inside `Session`: onboarding is a
    // form, so the very first screen a new phone shows already has a keyboard
    // in front of it.
    keyboard.watch();

    const identity = readIdentity();
    if (identity === null) {
      phase = { step: 'pairing' };
      return;
    }
    void openWith(identity);
  });


  /**
   * A device that has just been paired: remember the group, mint the device,
   * open.
   *
   * The group is written before the identity, so the engine finds it the
   * moment `Session.open` starts it — which is what makes the roster arrive
   * while "Qui êtes-vous ?" is on screen (DECISIONS 0068). No user is minted
   * here, and that is the change: a device that names a person before seeing
   * the group's roster is how one human ends up in the document twice.
   */
  async function register(group: Group): Promise<void> {
    phase = { step: 'loading' };
    try {
      rememberGroup(group);
      // Unnamed: `Identify` asks, and `name_device` writes it before the
      // device record exists at all.
      const identity = await mintDevice('');
      // Stored before the replica opens, deliberately: if opening fails, the
      // next launch must retry with *this* device id. A second one would
      // leave a dead peer in the replica's history.
      rememberIdentity(identity);
      await openWith(identity);
    } catch (cause) {
      phase = { step: 'failed', message: describe(cause) };
    }
  }
</script>

{#if phase.step === 'loading'}
  <p class="notice" role="status">Chargement…</p>
{:else if phase.step === 'failed'}
  <p class="notice" role="alert">
    L'application n'a pas pu démarrer.<br />
    <span class="detail">{phase.message}</span>
  </p>
{:else if phase.step === 'pairing'}
  <main class="first">
    <Pairing onpaired={(group) => void register(group)} />
  </main>
{:else}
  {@const session = phase.session}
  {#if session.error !== null}
    <ErrorBanner message={session.error} ondismiss={() => session.dismissError()} />
  {/if}

  {#if session.state.me === null}
    <!-- The app is open, the replica is loaded and sync is running; what is
         missing is the one thing only the group's roster can answer, so the
         question sits over it rather than in front of it (DECISIONS 0068). -->
    <main class="first">
      <Identify {session} />
    </main>
  {:else}
    <main>
      {#if session.screen === 'cart'}
        <Cart {session} />
      {:else if session.screen === 'list'}
        <List {session} />
      {:else if session.screen === 'recipes'}
        <Recipes {session} />
      {:else if session.screen === 'ingredients'}
        <Ingredients {session} />
      {:else}
        <Settings {session} />
      {/if}
    </main>

    <TabBar current={session.screen} onselect={(screen) => session.show(screen)} />
  {/if}
{/if}

<style>
  main {
    max-width: var(--content-width);
    margin: 0 auto;
  }

  /* The screens before the app proper — pairing, then "qui êtes-vous ?" —
     have no tab bar and a keyboard in front of the one field that matters
     (DECISIONS 0040). */
  .first {
    padding: var(--space-6) var(--space-4);
    padding-top: calc(var(--safe-top) + var(--space-7));
    padding-bottom: max(var(--space-6), var(--keyboard-inset));
  }

  .notice {
    max-width: var(--content-width);
    margin: 0 auto;
    padding: var(--space-7) var(--space-4);
    color: var(--text-muted);
    text-align: center;
  }

  .detail {
    font-family: var(--font-numeric);
    font-size: var(--text-sm);
  }
</style>
