<script lang="ts">
  import AmountDialog, {
    quantityDraft,
    servingsDraft,
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

  /**
   * Two lists, not one (DECISIONS 0097).
   *
   * A recipe and a loose ingredient are asked for by two different questions
   * — "what are we eating" and "what has run out" — and reading them
   * interleaved by name means reading the kind off every row before it says
   * anything. Recipes first, because that is the half that decides the other.
   *
   * Only what is still going is split. A settled entry is already out of the
   * way under "Terminées", and a folded section that is itself two sections
   * is two headings to say what one line already says.
   *
   * Written here rather than as two filters in the markup: the order of the
   * sections is this screen's own statement, and it belongs where it reads.
   */
  let sections = $derived(
    [
      { title: 'Recettes', entries: pending.filter((entry) => entry.item.kind === 'recipe') },
      {
        title: 'Ingrédients',
        entries: pending.filter((entry) => entry.item.kind === 'ingredient'),
      },
    ].filter((section) => section.entries.length > 0),
  );

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
   * Choosing an ingredient fills in what one usually buys of it — the amount
   * *and* the unit (DECISIONS 0089).
   *
   * The two fields are the one place in the app where an ingredient was asked
   * for by hand and its usual quantity was ignored: the swipe uses it, the
   * long press uses it, and this form opened on "g" whatever the shelf said.
   *
   * `seeded` is a plain variable and not `$state` on purpose. This effect
   * depends on the library as well as on the choice, so a frame arriving from
   * the other phone re-runs it — and re-running it must not overwrite an
   * amount somebody is in the middle of typing. The guard is what makes the
   * seeding happen once per choice rather than once per render.
   */
  let seeded = '';

  $effect(() => {
    const id = chosen;
    const usual = session.state.ingredients.find((held) => held.id === id)?.default_quantity;
    if (id === seeded) return;
    seeded = id;
    if (id === '') return;
    // Nobody has said what one buys of it: leave the fields as they are rather
    // than inventing "1 pièce" in a form that is about to be filled in anyway.
    if (usual === null || usual === undefined) return;
    amount = usual.amount;
    unit = usual.unit;
  });

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
          session.run({ command: 'add_recipe_to_list', recipe: chosenRecipe, servings, only: null })
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

  // --- changing what an entry asks for --------------------------------------

  /**
   * What a line asks for, the same two ways the shelf offers it and the same
   * two ways whichever kind of line it is (DECISIONS 0072, 0077, 0079): a
   * notch either side of the amount, and the exact answer behind the amount
   * itself.
   *
   * A notch is `nudge_list_entry` and never arithmetic done here — what one
   * is worth is the ingredient's usual shopping quantity or one whole recipe
   * as written, which is the core's rule (Rule 9). It is also the core that
   * decides a line nudged below its last notch comes off the list, which is
   * why "−" can empty a row and why that reads the same as pressing "×".
   *
   * The recipe row used to do its own subtraction and hold its own floor, so
   * one screen carried two identical "−" that meant opposite things: one
   * removed the row, the other stopped at one person. Both are the same
   * control now, and the exact answer is behind the amount on both.
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
  function editQuantity(entry: string, name: string, held: QuantityInput): void {
    entryAmount = quantityDraft(held);
    editing = { entry, name };
  }

  /** The same door on a recipe's line: for how many people, exactly. */
  function editServings(entry: string, name: string, held: number): void {
    entryAmount = servingsDraft(held);
    editing = { entry, name };
  }

  /**
   * Which command the door writes back is the draft's own business: an
   * amount and a serving count are different commands because they measure
   * different things, and the core refuses each on the other kind of line.
   */
  async function confirmAmount(typed: AmountDraft): Promise<void> {
    if (editing === null) return;
    const accepted = await (typed.kind === 'servings'
      ? session.run({
          command: 'set_entry_servings',
          entry: editing.entry,
          servings: typed.servings,
        })
      : session.run({
          command: 'set_entry_quantity',
          entry: editing.entry,
          quantity: { amount: typed.amount, unit: typed.unit },
        }));
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

    <!-- One control, two kinds of line, and one line for both the control and
         what the row has to say about itself (DECISIONS 0090). Every
         accessible name carries the row it belongs to and the amount it is
         showing: navigating by button is the ordinary way through this
         screen, and "Moins" alone names neither what it takes one off nor
         which of six rows it is on — on a bare ingredient it is also the
         button that empties the row. -->
    <div class="line">
      {#if entry.item.kind === 'recipe'}
        {@const item = entry.item}
        <div class="amount">
          <button
            type="button"
            class="notch"
            aria-label="Moins de {item.name}"
            onclick={() => nudge(entry.id, -1)}>−</button
          >
          <button
            type="button"
            class="quantity"
            aria-label="Quantité de {item.name} : {item.servings} personnes — modifier"
            onclick={() => editServings(entry.id, item.name, item.servings)}
            >{item.servings} pers.</button
          >
          <button
            type="button"
            class="notch"
            aria-label="Plus de {item.name}"
            onclick={() => nudge(entry.id, 1)}>+</button
          >
        </div>
      {:else}
        {@const item = entry.item}
        <div class="amount">
          <button
            type="button"
            class="notch"
            aria-label="Moins de {item.name}"
            onclick={() => nudge(entry.id, -1)}>−</button
          >
          <!-- The amount *and its unit*: this button is the only door to
               either, and its name says so (DECISIONS 0090). -->
          <button
            type="button"
            class="quantity"
            aria-label="Quantité et unité de {item.name} : {formatQuantity(
              item.quantity,
            )} — modifier"
            onclick={() => editQuantity(entry.id, item.name, item.edit)}
            >{formatQuantity(item.quantity)}</button
          >
          <button
            type="button"
            class="notch"
            aria-label="Plus de {item.name}"
            onclick={() => nudge(entry.id, 1)}>+</button
          >
        </div>
      {/if}

      <p class="meta">
        {#if entry.item.kind === 'recipe' && entry.item.servings !== entry.item.written_for}
          écrite pour {entry.item.written_for} ·
        {/if}
        {#if entry.progress.total > 0}
          {entry.progress.settled} / {entry.progress.total} réglé{entry.progress.settled > 1
            ? 's'
            : ''} ·
        {/if}
        {#if entry.added_by !== null}{entry.added_by} ·{/if}
        {relativeTime(entry.added_at)}
      </p>
    </div>
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
    <p class="empty bubble">Rien sur la liste pour l'instant.</p>
  {:else if pending.length === 0}
    <p class="empty bubble">Tout est réglé. « Terminer les courses » vide la liste.</p>
  {/if}

  <!-- One `<ul class="pending">` per kind rather than one for the screen: the
       class means "still going", which is what every other reader of it — the
       tests included — is asking about. -->
  {#each sections as section (section.title)}
    <section>
      <h2 class="display">{section.title}</h2>
      <ul class="pending">
        {#each section.entries as entry (entry.id)}
          {@render row(entry)}
        {/each}
      </ul>
    </section>
  {/each}

  {#if done.length > 0}
    <details class="done">
      <summary><span class="display">Terminées ({done.length})</span></summary>
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
    padding: var(--space-2) var(--space-4);
    border: 0;
    border-radius: var(--radius-pill);
    background: var(--accent);
    color: var(--on-accent);
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
    box-shadow: var(--shadow-sm);
    cursor: pointer;
  }

  /* A form is a lilac tile and its fields are cream — one shape, used by every
     form in the app (DECISIONS 0081). Cream on cream would leave a field with
     no edge against the card it sits in. */
  form {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    min-width: 0;
    margin-bottom: var(--space-5);
    padding: var(--space-4);
    border: 2px solid var(--border);
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
    border: 2px solid var(--border-strong);
    border-radius: var(--radius-pill);
    background: var(--bubble);
    color: var(--text);
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .modes button.on {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--on-accent);
    font-weight: var(--weight-bold);
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
    height: var(--tapsize);
    border: 2px solid var(--border-strong);
    border-radius: var(--radius-pill);
    background: var(--bubble);
    color: var(--text);
    cursor: pointer;
  }

  .pour span {
    font-family: var(--font-numeric);
    font-weight: var(--weight-bold);
    font-variant-numeric: tabular-nums;
  }

  .pour small {
    color: var(--text-muted);
    font-size: var(--text-xs);
  }

  .submit {
    padding: var(--space-3);
    border: 0;
    border-radius: var(--radius-pill);
    background: var(--accent);
    color: var(--on-accent);
    font-weight: var(--weight-bold);
    box-shadow: var(--shadow-sm);
    cursor: pointer;
  }

  /* A disabled primary is a sunken pill in muted ink, not a half-transparent
     orange one: over a lilac tile the alpha turns the label to mud
     (DECISIONS 0081). */
  .submit:disabled {
    background: var(--surface-sunken);
    color: var(--text-muted);
    box-shadow: none;
  }

  .problem {
    margin-bottom: var(--space-3);
    padding: var(--space-3);
    border: 2px solid var(--danger);
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
    margin: var(--space-5) 0;
    color: var(--text-muted);
    text-align: center;
  }

  /* One heading per kind, in the display face directly on the cloth — the
     same shape the cart uses for an aisle, and allowed there for the same
     reason: at `--text-lg` in a heavy face it is over the 24px-equivalent AA
     reads at 3:1, and `--display-ink` clears that against both checks
     (DECISIONS 0081, 0097). */
  section {
    margin-bottom: var(--space-5);
  }

  h2 {
    margin-bottom: var(--space-2);
    color: var(--display-ink);
    font-size: var(--text-lg);
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
    border-radius: var(--radius-md);
    background: var(--bubble);
  }

  /* The same shape as the cart's folded sections, because it is the same
     statement: this is settled, and it is out of the way (DECISIONS 0059). */
  .done {
    margin-top: var(--space-4);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
  }

  .done summary {
    padding: var(--space-2) var(--space-1);
    color: var(--text-muted);
    font-size: var(--text-lg);
    cursor: pointer;
  }

  .done ul {
    margin-top: var(--space-2);
  }

  .hint {
    margin: 0 var(--space-1) var(--space-2);
    color: var(--text-muted);
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
    font-weight: var(--weight-semibold);
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

  /* The amount and what the row has to say about itself, side by side
     (DECISIONS 0090). This is the line that used to be two: three separate
     bordered pills on one row and the meta on another, which cost a third of
     a card's height to say "500 g". */
  .line {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-top: var(--space-2);
  }

  /* One shape for both kinds of line: a notch either side, and the amount in
     the middle is the door to the exact one (DECISIONS 0077, 0079). Written
     once because it is one control — the recipe row's copy of it drifted into
     meaning something else within a single release.

     **One pill holding three buttons**, which is the shape `AmountDialog` and
     `RecipeReader` already use for the same question (0090). Three separate
     bordered pills drew six edges to carry one number; this draws one, and
     the buttons inside it are the same size they were. */
  .amount {
    flex: none;
    display: flex;
    align-items: center;
    min-height: var(--tapsize);
    border-radius: var(--radius-pill);
    background: var(--ring);
    color: var(--on-ring);
  }

  /* `--tapsize` in both directions. It was `--space-6` tall, which is 32px:
     below the floor, on the control a thumb aims at in a shop while walking,
     and the one either side of it takes the row off the list. */
  .amount button {
    height: var(--tapsize);
    border: 0;
    background: none;
    color: inherit;
    cursor: pointer;
  }

  /* `.notch` and not `.step`: `RecipeEditor` already has a `.step`, Svelte
     scopes the styles but `ui-test` queries the DOM globally, and two
     components naming a class the same thing silently change what a selector
     counts (the same trap as `.picker`). */
  .notch {
    flex: none;
    width: var(--tapsize);
    font-size: var(--text-lg);
    font-weight: var(--weight-bold);
    line-height: 1;
  }

  /* The number between the two notches, and the door to the exact amount —
     and, on a bare ingredient, to its unit. Underlined rather than boxed: it
     is already inside the pink pill, so a second surface would say nothing,
     and something has to say it can be pressed. */
  .amount .quantity {
    flex: 0 1 auto;
    min-width: 0;
    padding: 0 var(--space-1);
    font-family: var(--font-numeric);
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
    font-variant-numeric: tabular-nums;
    text-decoration: underline;
    text-underline-offset: 3px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Takes what is left of the line, and ellipsises rather than pushing the
     control it shares the line with off a narrow phone. */
  .meta {
    flex: 1;
    min-width: 0;
    margin: 0;
    color: var(--text-muted);
    font-size: var(--text-xs);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>

