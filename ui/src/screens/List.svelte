<script lang="ts">
  import AmountDialog, {
    quantityDraft,
    type AmountDraft,
  } from '../components/AmountDialog.svelte';
  import IngredientPicker from '../components/IngredientPicker.svelte';
  import QuantityField from '../components/QuantityField.svelte';
  import Screen from '../components/Screen.svelte';
  import SearchPicker, { type PickerOption } from '../components/SearchPicker.svelte';
  import type { ListEntryView } from '../lib/bindings/ListEntryView';
  import type { QuantityInput } from '../lib/bindings/QuantityInput';
  import type { RecipeSummaryView } from '../lib/bindings/RecipeSummaryView';
  import type { UnitTag } from '../lib/bindings/UnitTag';
  import { byName, formatQuantity, relativeTime } from '../lib/format';
  import { PROBLEM_LABEL } from '../lib/labels';
  import type { Session } from '../lib/session.svelte';

  /**
   * What has been asked for — the sources the cart derives from (Rule 3).
   *
   * Entries leave on their own once every ingredient they contributed is
   * settled, and the purge is deferred to "terminer les courses" so the trip
   * can be undone until then (DECISIONS 0020). Until that happens they are
   * folded away below the ones that are still going, for the same reason the
   * cart folds away what is already in the trolley: the screen is there to
   * show what is left (DECISIONS 0059).
   *
   * Every entry says how much of it is wanted and every entry can be told
   * otherwise, whichever kind it is (DECISIONS 0077): a recipe in people, a
   * bare ingredient in its own unit. This is the screen the amount is read
   * off when the answer turns out to be wrong, and it used to be the one
   * screen where it could not be changed.
   */
  let { session }: { session: Session } = $props();

  let problems = $derived(session.state.problems);

  /**
   * Alphabetical, and stable: the core hands the list back in the order the
   * entries were added, which is the order of a log rather than the order of
   * anything a person is looking for. The id breaks ties so two entries of the
   * same name never swap places between renders.
   */
  let entries = $derived(
    [...session.state.list].sort(
      (a, b) => byName(a.item, b.item) || a.id.localeCompare(b.id),
    ),
  );

  /** An entry is done when every ingredient it contributed is settled. */
  let pending = $derived(entries.filter((entry) => !entry.progress.complete));
  let done = $derived(entries.filter((entry) => entry.progress.complete));

  let recipes = $derived([...session.state.recipes].sort(byName));
  let recipeOptions = $derived<PickerOption[]>(
    recipes.map((recipe) => ({
      id: recipe.id,
      name: recipe.name,
      hint: `${recipe.servings} pers.`,
    })),
  );

  // --- adding ----------------------------------------------------------------

  /**
   * Both halves of "what do I want": a recipe, and the thing that is not in a
   * recipe. Adding a recipe was only reachable by opening it and reading it
   * first, which is the wrong way round when you already know what you want
   * (DECISIONS 0059).
   */
  type Mode = 'ingredient' | 'recipe';

  let adding = $state(false);
  let mode = $state<Mode>('ingredient');
  let chosen = $state('');
  let amount = $state('');
  let unit = $state<UnitTag>('g');
  let chosenRecipe = $state('');

  let recipe = $derived<RecipeSummaryView | null>(
    recipes.find((summary) => summary.id === chosenRecipe) ?? null,
  );

  /**
   * How many people this entry is for.
   *
   * Derived from the chosen recipe with an override on top, rather than
   * seeded from it: a `$state` seeded from a view captures the value once and
   * would keep the last recipe's serving count when another is picked — and
   * the warning Svelte raises about that is right. The override carries the
   * recipe it belongs to, so choosing a different one drops it.
   */
  let servingsOverride = $state<{ recipe: string; servings: number } | null>(null);
  let servings = $derived(
    servingsOverride !== null && servingsOverride.recipe === chosenRecipe
      ? servingsOverride.servings
      : (recipe?.servings ?? 1),
  );

  function rescaleDraft(by: number): void {
    const next = servings + by;
    if (next < 1) return;
    servingsOverride = { recipe: chosenRecipe, servings: next };
  }

  function toggleAdding(): void {
    adding = !adding;
    if (!adding) return;
    // A panel that opens holding the last thing that was added is a panel that
    // adds it twice; the pickers themselves die with it and forget their own.
    chosen = '';
    amount = '';
    chosenRecipe = '';
    servingsOverride = null;
  }

  function add(event: SubmitEvent): void {
    event.preventDefault();
    const accepted =
      mode === 'recipe'
        ? chosenRecipe !== '' &&
          session.run({ command: 'add_recipe_to_list', recipe: chosenRecipe, servings })
        : chosen !== '' &&
          session.run({
            command: 'add_ingredient_to_list',
            ingredient: chosen,
            quantity: { amount, unit },
          });
    if (accepted) {
      adding = false;
      chosen = '';
      amount = '';
      chosenRecipe = '';
      servingsOverride = null;
    }
  }

  function rescale(entry: string, count: number): void {
    if (count < 1) return;
    session.run({ command: 'set_entry_servings', entry, servings: count });
  }

  // --- changing what an entry asks for --------------------------------------

  /**
   * The amount on a bare ingredient's line, the same two ways the shelf offers
   * it (DECISIONS 0072, 0077): a notch either side of it, and the exact answer
   * behind the amount itself.
   *
   * A notch is `nudge_list_entry` and never arithmetic done here — what one
   * is worth is the ingredient's usual shopping quantity, which is the core's
   * rule (Rule 9). It is also the core that decides a line nudged below its
   * last notch comes off the list, which is why "−" can empty a row and why
   * that reads the same as pressing "×".
   */
  let editing = $state<{ entry: string; name: string } | null>(null);
  let entryAmount = $state<AmountDraft>(quantityDraft({ amount: '1', unit: 'piece' }));

  function nudge(entry: string, steps: number): void {
    session.run({ command: 'nudge_list_entry', entry, steps });
  }

  /**
   * `edit` and not `quantity`: the rendered amount is rounded so it reads well
   * on a row, and a form seeded from a rounded value writes the rounding back
   * on the next save.
   */
  function edit(entry: string, name: string, held: QuantityInput): void {
    entryAmount = quantityDraft(held);
    editing = { entry, name };
  }

  function confirmAmount(typed: AmountDraft): void {
    if (editing === null) return;
    const accepted = session.run({
      command: 'set_entry_quantity',
      entry: editing.entry,
      quantity: { amount: typed.amount, unit: typed.unit },
    });
    if (accepted) editing = null;
  }

  let subtitle = $derived.by(() => {
    if (entries.length === 0) return 'Vide';
    if (done.length === 0) return `${entries.length} entrées`;
    return `${pending.length} en cours, ${done.length} terminées`;
  });
</script>

{#snippet row(entry: ListEntryView)}
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
      {@const item = entry.item}
      <div class="amount">
        <button
          type="button"
          class="step"
          aria-label="Moins"
          onclick={() => nudge(entry.id, -1)}>−</button
        >
        <button
          type="button"
          class="quantity"
          aria-label="Quantité de {item.name}"
          onclick={() => edit(entry.id, item.name, item.edit)}
          >{formatQuantity(item.quantity)}</button
        >
        <button type="button" class="step" aria-label="Plus" onclick={() => nudge(entry.id, 1)}
          >+</button
        >
      </div>
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
{/snippet}

<Screen title="Liste" {subtitle}>
  {#snippet actions()}
    <button type="button" class="add" onclick={toggleAdding}>
      {adding ? 'Fermer' : 'Ajouter'}
    </button>
  {/snippet}

  {#if adding}
    <form onsubmit={add}>
      <div class="modes">
        <button type="button" class:on={mode === 'ingredient'} onclick={() => (mode = 'ingredient')}
          >Ingrédient</button
        >
        <button
          type="button"
          class:on={mode === 'recipe'}
          disabled={recipes.length === 0}
          onclick={() => (mode = 'recipe')}>Recette</button
        >
      </div>

      {#if mode === 'ingredient'}
        <!-- The picker lives and dies with this form, so closing it forgets a
             half-typed ingredient rather than keeping it for the next opening. -->
        <IngredientPicker {session} bind:value={chosen} label="Ingrédient" required />

        <QuantityField bind:amount bind:unit />

        <button type="submit" class="submit" disabled={chosen === ''}>Ajouter à la liste</button>
      {:else}
        <SearchPicker
          options={recipeOptions}
          bind:value={chosenRecipe}
          label="Recette"
          name="Recette"
          placeholder="Chercher une recette…"
          empty="Aucune recette ne correspond."
          required
        />

        <fieldset class="pour">
          <legend>Pour</legend>
          <div>
            <button type="button" aria-label="Moins" onclick={() => rescaleDraft(-1)}>−</button>
            <span>{servings} pers.</span>
            <button type="button" aria-label="Plus" onclick={() => rescaleDraft(1)}>+</button>
            {#if recipe !== null && servings !== recipe.servings}
              <small>écrite pour {recipe.servings}</small>
            {/if}
          </div>
        </fieldset>

        <!-- The recipe rather than the id: another device deleting it
             mid-choice takes the button with it, instead of leaving one that
             reports "not found" when it is pressed. -->
        <button type="submit" class="submit" disabled={recipe === null}
          >Ajouter à la liste</button
        >
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
  {:else if pending.length === 0}
    <p class="empty">Tout est réglé. « Terminer les courses » vide la liste.</p>
  {/if}

  <ul class="pending">
    {#each pending as entry (entry.id)}
      {@render row(entry)}
    {/each}
  </ul>

  {#if done.length > 0}
    <details class="done">
      <summary>Terminées ({done.length})</summary>
      <p class="hint">
        Tout ce qu'elles demandaient est réglé. Elles quittent la liste à « Terminer les courses ».
      </p>
      <ul>
        {#each done as entry (entry.id)}
          {@render row(entry)}
        {/each}
      </ul>
    </details>
  {/if}
</Screen>

{#if editing !== null}
  {@const line = editing}
  <AmountDialog
    title={line.name}
    bind:draft={entryAmount}
    onconfirm={confirmAmount}
    oncancel={() => (editing = null)}
  />
{/if}

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
    min-width: 0;
    margin-bottom: var(--space-5);
    padding: var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
  }

  .modes {
    display: flex;
    gap: var(--space-2);
  }

  .modes button {
    flex: 1;
    padding: var(--space-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .modes button.on {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: var(--weight-medium);
  }

  .modes button:disabled {
    opacity: 0.5;
  }

  .pour {
    margin: 0;
    padding: 0;
    min-width: 0;
    border: 0;
  }

  .pour legend {
    padding: 0 0 var(--space-2);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  .pour div {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .pour button {
    width: var(--tapsize);
    height: var(--space-6);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    cursor: pointer;
  }

  .pour span {
    font-variant-numeric: tabular-nums;
  }

  .pour small {
    color: var(--text-faint);
    font-size: var(--text-xs);
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

  /* The same shape as the cart's folded sections, because it is the same
     statement: this is settled, and it is out of the way (DECISIONS 0059). */
  .done {
    margin-top: var(--space-4);
    padding: var(--space-2) 0;
    border-top: 1px solid var(--border);
  }

  .done summary {
    padding: var(--space-2) var(--space-1);
    color: var(--text-muted);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    cursor: pointer;
  }

  .hint {
    margin: 0 var(--space-1) var(--space-2);
    color: var(--text-faint);
    font-size: var(--text-xs);
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

  /* The recipe row's shape, applied to the other kind of amount: a notch
     either side, and the amount in the middle is the door to the exact one
     (DECISIONS 0077). */
  .amount {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }

  .amount button {
    height: var(--space-6);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    cursor: pointer;
  }

  .step {
    flex: none;
    width: var(--tapsize);
  }

  /* It hugs what it says rather than filling the row: full width reads as a
     text field, and this is a door to one. */
  .quantity {
    flex: 0 1 auto;
    min-width: var(--tapsize);
    padding: 0 var(--space-3);
    color: var(--text-muted);
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    margin: var(--space-2) 0 0;
    color: var(--text-faint);
    font-size: var(--text-xs);
  }
</style>
