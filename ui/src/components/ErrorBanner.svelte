<script lang="ts">
  /**
   * A refused command, shown where it cannot be missed and dismissed by hand.
   *
   * The text is the core's own diagnostic, in English: it means another device
   * deleted something under this one, or a typed amount did not parse. It is
   * deliberately not translated — a sentence per failure mode would be a
   * second vocabulary to keep in step with the Rust error enum, and this is
   * the one place in the app where a person is not the intended reader.
   */
  let { message, ondismiss }: { message: string; ondismiss: () => void } = $props();
</script>

<div role="alert">
  <p>{message}</p>
  <button type="button" onclick={ondismiss} aria-label="Masquer">×</button>
</div>

<style>
  div {
    position: sticky;
    top: 0;
    z-index: 3;
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    padding-top: calc(var(--safe-top) + var(--space-3));
    background: var(--danger-soft);
    color: var(--danger);
    border-bottom: 1px solid var(--danger);
  }

  p {
    flex: 1;
    margin: 0;
    font-size: var(--text-sm);
  }

  button {
    flex: none;
    width: var(--space-5);
    height: var(--space-5);
    display: grid;
    place-items: center;
    border: 0;
    border-radius: var(--radius-pill);
    background: none;
    color: inherit;
    font-size: var(--text-lg);
    line-height: 1;
    cursor: pointer;
  }
</style>
