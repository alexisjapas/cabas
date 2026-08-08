<script lang="ts">
  import QuantityField from '../components/QuantityField.svelte';
  import Screen from '../components/Screen.svelte';
  import type { UnitTag } from '../lib/bindings/UnitTag';
  import { byName, formatQuantity, relativeTime } from '../lib/format';
  import { PROBLEM_LABEL } from '../lib/labels';
  import type { Session } from '../lib/session.svelte';

  /**
   * What has been asked for — the sources the cart derives from (Rule 3).
   *
   * Entries leave on their own once every ingredient they contributed is
   * settled, and the purge is deferred to "terminer les courses" so the trip
   * can be undone until then (DECISIONS 0020).
   */
  let { session }: { session: Session } = $props();

  let entries = $derived(session.state.list);
  let problems = $derived(session.state.problems);

  /** Sorted for a reader; the core sorts by id, which is stable, not legible. */
  let ingredients = $derived([...session.state.ingredients].sort(byName));

  let adding = $state(false);
  let chosen = $state('');
  let amount = $state('');
  let unit = $state<UnitTag>('g');

  function add(event: SubmitEvent): void {
    event.preventDefault();
    if (chosen === '') return;
    const accepted = session.run({
      command: 'add_ingredient_to_list',
      ingredient: chosen,
      quantity: { amount, unit },
    });
    if (accepted) {
      adding = false;
      chosen = '';
      amount = '';
    }
  }

  function rescale(entry: string, servings: number): void {
    if (servings < 1) return;
    session.run({ command: 'set_entry_servings', entry, servings });
  }
</script>

<Screen title="Liste" subtitle={entries.length === 0 ? 'Vide' : `${entries.length} entrées`}>
  {#snippet actions()}
    <button type="button" class="add" onclick={() => (adding = !adding)}>
      {adding ? 'Fermer' : 'Ajouter'}
    </button>
  {/snippet}

  {#if adding}
    <form onsubmit={add}>
      {#if ingredients.length === 0}
        <p class="hint">
          Aucun ingrédient dans la bibliothèque. Créez-en un dans l'onglet Ingrédients.
        </p>
      {:else}
        <label>
          Ingrédient
          <select bind:value={chosen} required>
            <option value="" disabled>Choisir…</option>
            {#each ingredients as ingredient (ingredient.id)}
              <option value={ingredient.id}>{ingredient.name}</option>
            {/each}
          </select>
        </label>

        <QuantityField bind:amount bind:unit />

        <button type="submit" class="submit" disabled={chosen === ''}>Ajouter à la liste</button>
      {/if}
    </form>
  {/if}

  {#each problems as problem, index (`${problem.entry ?? ''}-${problem.kind}-${index}`)}
    <div class="problem" role="status">
      <p>{PROBLEM_LABEL[problem.kind]}</p>
      <details>
        <summary>Détail</summary>
        <code>{problem.detail}</code>
      </details>
    </div>
  {/each}

  {#if entries.length === 0}
    <p class="empty">Rien sur la liste pour l'instant.</p>
  {/if}

  <ul>
    {#each entries as entry (entry.id)}
      <li>
        <div class="head">
          <span class="name">{entry.item.name}</span>
          <button
            type="button"
            class="remove"
            aria-label="Retirer {entry.item.name}"
            onclick={() => session.run({ command: 'remove_list_entry', entry: entry.id })}>×</button
          >
        </div>

        {#if entry.item.kind === 'recipe'}
          {@const item = entry.item}
          <div class="servings">
            <button type="button" aria-label="Moins" onclick={() => rescale(entry.id, item.servings - 1)}
              >−</button
            >
            <span>{item.servings} pers.</span>
            <button type="button" aria-label="Plus" onclick={() => rescale(entry.id, item.servings + 1)}
              >+</button
            >
            {#if item.servings !== item.written_for}
              <small>écrite pour {item.written_for}</small>
            {/if}
          </div>
        {:else}
          <p class="quantity">{formatQuantity(entry.item.quantity)}</p>
        {/if}

        <p class="meta">
          {#if entry.progress.total > 0}
            {entry.progress.settled} / {entry.progress.total} réglé{entry.progress.settled > 1
              ? 's'
              : ''} ·
          {/if}
          {#if entry.added_by !== null}{entry.added_by} ·{/if}
          {relativeTime(entry.added_at)}
        </p>
      </li>
    {/each}
  </ul>
</Screen>

<style>
  .add {
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-pill);
    background: var(--surface-raised);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    cursor: pointer;
  }

  form {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    margin-bottom: var(--space-5);
    padding: var(--space-4);
    border: 1px solid var(--border);
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

  select {
    padding: var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-weight: var(--weight-normal);
  }

  .submit {
    padding: var(--space-3);
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--on-accent);
    font-weight: var(--weight-semibold);
    cursor: pointer;
  }

  .submit:disabled {
    opacity: 0.5;
  }

  .problem {
    margin-bottom: var(--space-3);
    padding: var(--space-3);
    border: 1px solid var(--danger);
    border-radius: var(--radius-md);
    background: var(--danger-soft);
    color: var(--danger);
    font-size: var(--text-sm);
  }

  .problem p {
    margin: 0;
  }

  .problem summary {
    margin-top: var(--space-2);
    font-size: var(--text-xs);
    cursor: pointer;
  }

  code {
    font-family: var(--font-numeric);
    font-size: var(--text-xs);
  }

  .empty {
    margin: var(--space-6) 0;
    color: var(--text-muted);
    text-align: center;
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  li {
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .name {
    flex: 1;
    min-width: 0;
    font-weight: var(--weight-medium);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .remove {
    flex: none;
    width: var(--space-6);
    height: var(--space-6);
    display: grid;
    place-items: center;
    border: 0;
    border-radius: var(--radius-pill);
    background: none;
    color: var(--text-muted);
    font-size: var(--text-lg);
    line-height: 1;
    cursor: pointer;
  }

  .servings {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }

  .servings button {
    width: var(--tapsize);
    height: var(--space-6);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    cursor: pointer;
  }

  .servings small {
    color: var(--text-faint);
    font-size: var(--text-xs);
  }

  .quantity {
    margin: var(--space-1) 0 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .meta {
    margin: var(--space-2) 0 0;
    color: var(--text-faint);
    font-size: var(--text-xs);
  }
</style>
