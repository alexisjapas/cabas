<script lang="ts">
  import Photo from '../components/Photo.svelte';
  import Screen from '../components/Screen.svelte';
  import type { FocusView } from '../lib/bindings/FocusView';
  import { decimal, formatQuantity } from '../lib/format';
  import { chosenLines, entriesBySource } from '../lib/list';
  import type { Session } from '../lib/session.svelte';

  /**
   * A recipe, read at the number of people it is being cooked for.
   *
   * Nothing here scales anything. `focus.recipe` arrives already rendered at
   * `servings` — every quantity in the ingredient list and inside every step
   * — because the arithmetic is exact rationals and exact rationals stay in
   * Rust (Rule 4). Changing the stepper re-issues `OpenRecipe` and the whole
   * view comes back scaled.
   */
  let {
    session,
    focus,
    onedit,
    ondelete,
    onclose,
  }: {
    session: Session;
    focus: FocusView;
    onedit: () => void;
    ondelete: () => void;
    onclose: () => void;
  } = $props();

  let recipe = $derived(focus.recipe);

  let confirmingDelete = $state(false);

  /**
   * The confirmation is tied to what was added, not to a flag: rescaling and
   * adding again is a normal thing to do, and a button still reading "ajoutée"
   * at a different serving count would be claiming something untrue.
   */
  let addedAt = $state<string | null>(null);
  let added = $derived(addedAt === `${recipe.id}:${recipe.servings}`);

  function read(servings: number): void {
    if (servings < 1) return;
    session.run({ command: 'open_recipe', recipe: recipe.id, servings });
  }

  /**
   * The whole recipe, at the servings being read — not the servings it was
   * written for: the stepper above is the "we are six tonight" the list entry
   * records.
   *
   * Two commands behind one button, and which one it is depends on whether
   * some of this recipe is *already* asked for (DECISIONS 0091). Over a
   * half-added recipe it says "Ajouter le reste" and clears the restriction on
   * that entry, because a second entry beside it would ask for the chosen
   * lines twice. Otherwise it adds an entry, including when one already exists
   * — asking for a recipe twice is a thing people do, and it was true before
   * any of this.
   */
  function addToList(): void {
    if (
      partial && entry !== undefined
        ? session.run({ command: 'set_entry_components', entry: entry.id, only: null })
        : session.run({
            command: 'add_recipe_to_list',
            recipe: recipe.id,
            servings: recipe.servings,
            only: null,
          })
    ) {
      addedAt = `${recipe.id}:${recipe.servings}`;
    }
  }

  // --- a line at a time (DECISIONS 0091) -------------------------------------

  /**
   * This recipe's entry on the list, and which of its lines that entry asks
   * for.
   *
   * Read off the core's list rather than remembered here, like every other
   * shelf that can push a row onto it (DECISIONS 0067): it is what makes the
   * ticks right after a reload and right when the other phone changes them.
   * An empty set is "all of them" — see `chosenLines`.
   */
  let entry = $derived(entriesBySource(session.state.list).get(recipe.id));
  let chosen = $derived(chosenLines(entry));
  let partial = $derived(entry !== undefined && chosen.size > 0);

  function asked(usage: string): boolean {
    if (entry === undefined) return false;
    return chosen.size === 0 || chosen.has(usage);
  }

  /**
   * Adds or removes one line of the recipe.
   *
   * Four cases, and the shape of each is decided by what the entry currently
   * is rather than by a mode this screen holds:
   *
   * - nothing on the list yet → a new entry asking for this line alone;
   * - a restricted entry → the line joins or leaves the selection, and the
   *   last one leaving takes the entry off the list, exactly the way the last
   *   notch off a bare ingredient does (0072);
   * - a whole recipe, line ticked → the selection becomes *every other line*,
   *   which is what "I already have this one" means;
   * - a whole recipe, line unticked → impossible, since a whole recipe asks
   *   for all of them.
   */
  function toggleLine(usage: string): void {
    if (entry === undefined) {
      session.run({
        command: 'add_recipe_to_list',
        recipe: recipe.id,
        servings: recipe.servings,
        only: [usage],
      });
      return;
    }

    const lines = recipe.components.map((component) => component.usage);
    const next = new Set(chosen.size === 0 ? lines : chosen);
    if (next.has(usage)) next.delete(usage);
    else next.add(usage);

    if (next.size === 0) {
      // Not `set_entry_components` with an empty list: the core refuses that,
      // and rightly — an entry asking for nothing is an entry to remove.
      session.run({ command: 'remove_list_entry', entry: entry.id });
      return;
    }
    // Every line ticked is the whole recipe again, and saying so keeps a row
    // that asks for all of it from reading as a restriction that happens to
    // cover everything.
    session.run({
      command: 'set_entry_components',
      entry: entry.id,
      only: next.size === lines.length ? null : [...next],
    });
  }

  function remove(): void {
    if (!confirmingDelete) {
      confirmingDelete = true;
      return;
    }
    ondelete();
  }
</script>

<Screen title={recipe.name} subtitle={recipe.yields !== null ? `Donne ${formatQuantity(recipe.yields)}` : undefined}>
  {#snippet actions()}
    <button type="button" class="close" onclick={onclose}>Fermer</button>
  {/snippet}

  <Photo {session} photo={recipe.photo} alt={recipe.name} size="full" />

  <div class="servings">
    <div class="stepper">
      <button type="button" aria-label="Moins" onclick={() => read(recipe.servings - 1)}>−</button>
      <span class="count">{recipe.servings} pers.</span>
      <button type="button" aria-label="Plus" onclick={() => read(recipe.servings + 1)}>+</button>
    </div>
    {#if recipe.servings !== recipe.written_for}
      <small>écrite pour {recipe.written_for}</small>
    {/if}
  </div>

  <button type="button" class="primary" onclick={addToList}>
    {#if partial}
      Ajouter le reste à la liste
    {:else}
      {added ? 'Ajoutée à la liste' : 'Ajouter à la liste'}
    {/if}
  </button>

  <h2 class="display">Ingrédients</h2>
  {#if recipe.components.length === 0}
    <p class="empty bubble">Aucun ingrédient.</p>
  {:else}
    <p class="hint">
      {#if partial}
        {chosen.size} sur {recipe.components.length} sur la liste. Touchez une ligne pour
        l'ajouter ou la retirer.
      {:else}
        Touchez une ligne pour n'ajouter qu'elle : pratique quand on a déjà le reste.
      {/if}
    </p>
  {/if}
  <ul class="components">
    {#each recipe.components as component (component.usage)}
      {@const on = asked(component.usage)}
      <li>
        <!-- The whole row is the control (DECISIONS 0091): a line of a recipe
             is read and decided about in one glance, and a separate checkbox
             beside it would take width off the one thing a phone has little
             of. -->
        <button
          type="button"
          class="pick"
          class:on
          aria-pressed={on}
          aria-label="{component.name ?? 'Ligne supprimée'} : {on
            ? 'sur la liste'
            : 'pas sur la liste'}"
          onclick={() => toggleLine(component.usage)}
        >
          <span class="tick" aria-hidden="true">{on ? '✓' : '+'}</span>
          {#if component.name === null}
            <span class="gone">
              {component.kind === 'ingredient' ? 'Ingrédient supprimé' : 'Sous-recette supprimée'}
            </span>
          {:else}
            <span class="what">{component.name}</span>
          {/if}
          <span class="amount">
            {#if component.kind === 'ingredient'}
              {formatQuantity(component.quantity)}
            {:else if component.amount.kind === 'factor'}
              ×{decimal(component.amount.factor)}
            {:else}
              {formatQuantity(component.amount.quantity)}
            {/if}
          </span>
        </button>
      </li>
    {/each}
  </ul>

  {#if recipe.steps.length > 0}
    <h2 class="display">Préparation</h2>
    <ol class="steps" role="list">
      {#each recipe.steps as step, index (index)}
        <li><p class="prose">{#each step.segments as segment, position (position)}{#if segment.kind === 'text'}{segment.text}{:else if segment.kind === 'missing'}<span class="gone">ligne supprimée</span>{:else}<span class="ref">{#if segment.name !== null}{segment.name}{/if}{#if segment.name !== null && segment.quantity !== null}&nbsp;{/if}{#if segment.quantity !== null}<span class="refamount">{formatQuantity(segment.quantity)}</span>{/if}</span>{/if}{/each}</p></li>
      {/each}
    </ol>
  {/if}

  <div class="buttons">
    <button type="button" class="edit" onclick={onedit}>Modifier</button>
    <button type="button" class="delete" class:confirming={confirmingDelete} onclick={remove}>
      {confirmingDelete ? 'Confirmer la suppression' : 'Supprimer'}
    </button>
  </div>
</Screen>

<style>
  .close {
    padding: var(--space-2) var(--space-3);
    border: 2px solid var(--border-strong);
    border-radius: var(--radius-pill);
    background: var(--surface-raised);
    color: var(--text);
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
    white-space: nowrap;
    cursor: pointer;
  }

  .servings {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex-wrap: wrap;
    margin: var(--space-4) 0;
  }

  /* The same pink pill as `AmountDialog`'s, because it is the same control:
     how many people this is being read at. */
  .stepper {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    min-height: var(--tapsize);
    padding: 0 var(--space-4);
    border-radius: var(--radius-pill);
    background: var(--ring);
    color: var(--on-ring);
  }

  .stepper button {
    width: var(--tapsize);
    height: var(--tapsize);
    border: 0;
    background: none;
    color: inherit;
    font-size: var(--text-xl);
    font-weight: var(--weight-bold);
    line-height: 1;
    cursor: pointer;
  }

  .count {
    font-family: var(--font-numeric);
    font-size: var(--text-lg);
    font-weight: var(--weight-bold);
    font-variant-numeric: tabular-nums;
  }

  .servings small {
    padding: var(--space-1) var(--space-3);
    border-radius: var(--radius-pill);
    background: var(--bubble);
    color: var(--text-muted);
    font-size: var(--text-xs);
    font-weight: var(--weight-bold);
  }

  .primary {
    width: 100%;
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

  /* Section headings lean, like every other title in the app. They sit on the
     cloth, which the contrast rule allows at this size and weight. */
  h2 {
    margin: var(--space-5) 0 var(--space-2);
    color: var(--display-ink);
    font-size: var(--text-lg);
  }

  .empty {
    margin: 0;
    color: var(--text-muted);
  }

  /* On the cloth at body size, so it needs a surface under it — the global
     `.bubble` is the wrong shape here (it is padded like a card and this is
     one line), so it takes the cream and a small radius of its own. */
  .hint {
    margin: 0 0 var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    background: var(--bubble);
    color: var(--text-muted);
    font-size: var(--text-xs);
  }

  /* One cream card holding the whole list, with hairlines inside it rather
     than a rule between free-standing rows. */
  .components {
    margin: 0;
    padding: 0 var(--space-4);
    list-style: none;
    border-radius: var(--radius-md);
    background: var(--bubble);
  }

  .components li + li {
    border-top: 1px solid var(--border);
  }

  /* One row, one control (DECISIONS 0091). `--tapsize` tall because it is
     pressed in a kitchen, and the whole width because that is what a line of
     a recipe is. */
  .pick {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) 0;
    min-height: var(--tapsize);
    border: 0;
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }

  /* The tick, in the ring that means "a quantity" everywhere else — because
     that is what pressing it puts on the list. Unticked it is an outline, so
     the row reads the same width either way and nothing shifts on a tap. */
  .tick {
    flex: none;
    display: grid;
    place-items: center;
    width: var(--space-6);
    height: var(--space-6);
    border: 2px solid var(--border-strong);
    border-radius: var(--radius-pill);
    color: var(--text-muted);
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
    line-height: 1;
  }

  .pick.on .tick {
    border-color: transparent;
    background: var(--done);
    color: var(--on-done);
  }

  .what {
    flex: 1;
    min-width: 0;
    font-weight: var(--weight-medium);
  }

  .amount {
    flex: none;
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--ring);
    color: var(--on-ring);
    font-family: var(--font-numeric);
    font-size: var(--text-xs);
    font-weight: var(--weight-bold);
    white-space: nowrap;
  }

  /* The steps, in their own cream card. The number is drawn rather than left
     to `::marker`, because a lilac disc is the shape this system uses for a
     small ordinal — `role="list"` is what keeps the list semantics Safari
     drops the moment `list-style` goes away. */
  .steps {
    margin: 0;
    padding: var(--space-4);
    list-style: none;
    counter-reset: step;
    border-radius: var(--radius-md);
    background: var(--bubble);
  }

  .steps li {
    display: flex;
    gap: var(--space-3);
  }

  .steps li + li {
    margin-top: var(--space-3);
  }

  .steps li::before {
    counter-increment: step;
    content: counter(step);
    flex: none;
    display: grid;
    place-items: center;
    width: 1.625rem;
    height: 1.625rem;
    border-radius: var(--radius-pill);
    background: var(--surface-raised);
    color: var(--on-accent);
    font-size: var(--text-xs);
    font-weight: var(--weight-bold);
    font-variant-numeric: tabular-nums;
  }

  /* The authored spacing is the cook's: a step is a run of segments and the
     spaces live inside the text ones (DECISIONS 0022). */
  .prose {
    flex: 1;
    min-width: 0;
    margin: 0;
    line-height: var(--leading-normal);
    white-space: pre-wrap;
  }

  /* An anis pill carrying the name and the amount together, unbreakable. The
     quantity arrives already scaled to the servings on screen — nothing here
     recomputes it. */
  .ref {
    padding: 2px 7px;
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    color: var(--on-accent);
    font-weight: var(--weight-semibold);
    white-space: normal;
  }

  .refamount {
    font-family: var(--font-numeric);
    font-size: var(--text-sm);
  }

  .gone {
    color: var(--danger);
    font-size: var(--text-sm);
  }

  .buttons {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-6);
  }

  .edit {
    flex: 1;
    padding: var(--space-3);
    border: 2px solid var(--border-strong);
    border-radius: var(--radius-pill);
    background: var(--surface-raised);
    color: var(--text);
    font-weight: var(--weight-bold);
    cursor: pointer;
  }

  .delete {
    flex: none;
    padding: var(--space-3) var(--space-4);
    border: 2px solid var(--danger);
    border-radius: var(--radius-pill);
    background: var(--bubble);
    color: var(--danger);
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
    cursor: pointer;
  }

  .delete.confirming {
    background: var(--danger);
    color: var(--on-danger);
  }
</style>

