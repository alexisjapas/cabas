<script lang="ts">
  /**
   * The very first launch, and the only screen that runs before the app has
   * an identity.
   *
   * Nothing else can start until the device knows who it is (DECISIONS 0031):
   * every list entry carries an `added_by` and every ticked line a
   * `checked_by`. Both names are labels, not credentials — this asks who you
   * are, it does not verify it (Rule 7).
   */
  let { onsubmit }: { onsubmit: (userName: string, deviceName: string) => void } = $props();

  /** A first guess at the device name, so the second field is usually a no-op. */
  function guessDeviceName(): string {
    const agent = navigator.userAgent;
    if (/iPhone/.test(agent)) return 'iPhone';
    if (/iPad/.test(agent)) return 'iPad';
    if (/Android/.test(agent)) return 'Téléphone Android';
    return 'Ordinateur';
  }

  let userName = $state('');
  let deviceName = $state(guessDeviceName());
  let ready = $derived(userName.trim() !== '' && deviceName.trim() !== '');

  function submit(event: SubmitEvent): void {
    event.preventDefault();
    if (!ready) return;
    onsubmit(userName.trim(), deviceName.trim());
  }
</script>

<main>
  <h1>cabas</h1>
  <p class="lead">Les courses et les recettes, à deux, hors ligne.</p>

  <form onsubmit={submit}>
    <label>
      Votre prénom
      <input
        bind:value={userName}
        required
        autocomplete="given-name"
        enterkeyhint="next"
        placeholder="Alexis"
      />
    </label>

    <label>
      Nom de cet appareil
      <input bind:value={deviceName} required enterkeyhint="done" />
      <small>Pour distinguer vos appareils plus tard, au moment de les appairer.</small>
    </label>

    <button type="submit" disabled={!ready}>Commencer</button>
  </form>
</main>

<style>
  main {
    max-width: var(--content-width);
    margin: 0 auto;
    padding: var(--space-6) var(--space-4);
    padding-top: calc(var(--safe-top) + var(--space-7));
    /* No tab bar on this screen, but the same keyboard: two fields and the
       button that gets past them, on the first screen the phone ever shows. */
    padding-bottom: max(var(--space-6), var(--keyboard-inset));
  }

  h1 {
    font-size: var(--text-2xl);
  }

  .lead {
    margin: var(--space-2) 0 var(--space-6);
    color: var(--text-muted);
  }

  form {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
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
    font-size: var(--text-base);
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
    font-size: var(--text-base);
    font-weight: var(--weight-semibold);
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
