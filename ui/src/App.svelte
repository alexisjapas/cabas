<script lang="ts">
  import { onMount } from 'svelte';

  import ErrorBanner from './components/ErrorBanner.svelte';
  import TabBar from './components/TabBar.svelte';
  import { mintIdentity, readIdentity, rememberIdentity } from './lib/core';
  import { Session } from './lib/session.svelte';
  import Cart from './screens/Cart.svelte';
  import Ingredients from './screens/Ingredients.svelte';
  import List from './screens/List.svelte';
  import Onboarding from './screens/Onboarding.svelte';
  import Recipes from './screens/Recipes.svelte';
  import Settings from './screens/Settings.svelte';

  /**
   * Boot, in the order DECISIONS 0031 requires: the device's identity comes
   * out of `localStorage` before anything else can run, and a device that has
   * never launched is asked who it is.
   */
  type Phase =
    | { step: 'loading' }
    | { step: 'onboarding' }
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
    const identity = readIdentity();
    if (identity === null) {
      phase = { step: 'onboarding' };
      return;
    }
    void openWith(identity);
  });

  async function register(userName: string, deviceName: string): Promise<void> {
    phase = { step: 'loading' };
    try {
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
{:else if phase.step === 'onboarding'}
  <Onboarding onsubmit={register} />
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
