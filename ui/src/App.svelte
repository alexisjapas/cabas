<script lang="ts">
  import { onMount, tick } from 'svelte';

  import ErrorBanner from './components/ErrorBanner.svelte';
  import TabBar from './components/TabBar.svelte';
  import { mintIdentity, readIdentity, rememberIdentity } from './lib/core';
  import { keyboard } from './lib/keyboard.svelte';
  import { Session } from './lib/session.svelte';
  import Cart from './screens/Cart.svelte';
  import Ingredients from './screens/Ingredients.svelte';
  import List from './screens/List.svelte';
  import Onboarding from './screens/Onboarding.svelte';
  import Pairing from './screens/Pairing.svelte';
  import Recipes from './screens/Recipes.svelte';
  import Settings from './screens/Settings.svelte';
  import { rememberFamily, type Family } from './lib/sync.svelte';

  /**
   * Boot, in the order DECISIONS 0031 requires: the device's identity comes
   * out of `localStorage` before anything else can run, and a device that has
   * never launched is asked who it is.
   *
   * A first launch is asked one thing before that, though — which family this
   * device belongs to. Pairing comes first because joining an existing one is
   * the common case for the *second* phone, and a device that mints an
   * identity before knowing that has already written a user into a document
   * nobody else will ever see.
   */
  type Phase =
    | { step: 'loading' }
    | { step: 'pairing' }
    | { step: 'onboarding'; family: Family }
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
   * Put each screen back where it was left (DECISIONS 0003, same reasoning as
   * the persisted screen: an iOS cold reload otherwise drops you at the top of
   * the list mid-shop).
   *
   * After `tick()`, because the offset means nothing until the screen it
   * belongs to has rendered something to scroll. The effect depends on
   * `session.screen` and on nothing else — the offsets themselves are a plain
   * field, so the scroll listener that keeps them current cannot re-trigger
   * this.
   */
  $effect(() => {
    if (phase.step !== 'ready') return;
    const session = phase.session;
    const screen = session.screen;
    void tick().then(() => session.restoreScroll(screen));
  });

  async function register(family: Family, userName: string, deviceName: string): Promise<void> {
    phase = { step: 'loading' };
    try {
      // The family before the identity, so the engine finds it the moment
      // `Session.open` starts it — the first sync then carries this device's
      // own user record out with everything else.
      rememberFamily(family);
      const identity = await mintIdentity(userName, deviceName);
      // Stored before the replica opens, deliberately: if opening fails, the
      // next launch must retry with *this* identity. Minting a second one
      // would leave the first user record orphaned in the family document.
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
    <Pairing onpaired={(family) => (phase = { step: 'onboarding', family })} />
  </main>
{:else if phase.step === 'onboarding'}
  {@const family = phase.family}
  <Onboarding onsubmit={(userName, deviceName) => register(family, userName, deviceName)} />
{:else}
  {@const session = phase.session}
  {#if session.error !== null}
    <ErrorBanner message={session.error} ondismiss={() => session.dismissError()} />
  {/if}

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

<style>
  main {
    max-width: var(--content-width);
    margin: 0 auto;
  }

  /* Pairing has no tab bar and the same keyboard as onboarding: one field, on
     the first screen the phone ever shows. */
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
