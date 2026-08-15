<script lang="ts">
  import Screen from '../components/Screen.svelte';
  import SearchField from '../components/SearchField.svelte';
  import type { RecipeInput } from '../lib/bindings/RecipeInput';
  import { byName, formatQuantity, matches } from '../lib/format';
  import type { Session } from '../lib/session.svelte';
  import RecipeEditor from './RecipeEditor.svelte';
  import RecipeReader from './RecipeReader.svelte';

  /**
   * Recipes: the shelf, the one being read, and the one being written.
   *
   * Three views behind one tab, because they are one thing at three moments
   * and a phone has room for exactly one of them. Which recipe is *open* is
   * core state — `OpenRecipe` sets it and it is never synced, since two people
   * reading different recipes is not a conflict — while the recipe being
   * *edited* is a draft that lives here until it is saved (DECISIONS 0039).
   */
  let { session }: { session: Session } = $props();

  /** Sorted for a reader; the core sorts by id, which is stable, not legible. */
  let recipes = $derived([...session.state.recipes].sort(byName));
  let focus = $derived(session.state.focus);

  /** The shelf narrows the same way every other list of names does (0058). */
  let query = $state('');
  let shown = $derived(recipes.filter((recipe) => matches(recipe.name, query)));

  function blank(): RecipeInput {
    return { id: null, name: '', servings: 4, yields: null, components: [], steps: [] };
  }

  /**
   * The draft lives here rather than inside the editor, and is always a whole
   * recipe rather than `RecipeInput | null` — `writing` is what says whether
   * the editor is open. That is what lets the editor take it as a `$bindable`
   * and mutate it in place: a form seeded from a prop would capture the value
   * once and quietly ignore the next one.
   */
  let draft = $state<RecipeInput>(blank());
  let writing = $state(false);

  function write(recipe: RecipeInput): void {
    draft = recipe;
    writing = true;
  }

  /**
   * Saves, and lands the reader on what was just written.
   *
   * A new recipe has no id until the core mints one, so the way to find it is
   * to notice which one is new — reliable, because ids are unique and this
   * runs between two synchronous states (DECISIONS 0033). Returning `false`
   * keeps the editor open on a refusal.
   */
  function save(recipe: RecipeInput): boolean {
    const before = new Set(session.state.recipes.map((summary) => summary.id));
    if (!session.run({ command: 'save_recipe', recipe })) return false;

    const created = session.state.recipes.find((summary) => !before.has(summary.id));
    if (created !== undefined) {
      session.run({ command: 'open_recipe', recipe: created.id, servings: null });
    }
    writing = false;
    return true;
  }

  function remove(recipe: string): void {
    if (session.run({ command: 'delete_recipe', recipe })) {
      session.run({ command: 'close_recipe' });
    }
  }

  function count(n: number, one: string, many: string): string {
    return `${n} ${n > 1 ? many : one}`;
  }
</script>

{#if writing}
  <RecipeEditor {session} bind:recipe={draft} onsave={save} oncancel={() => (writing = false)} />
{:else if focus !== null}
  {@const open = focus}
  <RecipeReader
    {session}
    focus={open}
    onedit={() => write(structuredClone(open.edit))}
    ondelete={() => remove(open.recipe.id)}
    onclose={() => session.run({ command: 'close_recipe' })}
  />
{:else}
  <Screen
    title="Recettes"
    subtitle={recipes.length === 0 ? 'Aucune recette' : count(recipes.length, 'recette', 'recettes')}
  >
    {#snippet actions()}
      <button type="button" class="add" onclick={() => write(blank())}>Nouvelle</button>
    {/snippet}

    {#if recipes.length === 0}
      <p class="empty">Aucune recette. Écrivez la première avec « Nouvelle ».</p>
    {:else}
      <SearchField bind:value={query} placeholder="Chercher une recette…" />
      {#if shown.length === 0}
        <p class="empty">Aucune recette ne correspond.</p>
      {/if}
    {/if}

    <ul>
      {#each shown as recipe (recipe.id)}
        <li>
          <button
            type="button"
            onclick={() => session.run({ command: 'open_recipe', recipe: recipe.id, servings: null })}
          >
            <span class="text">
              <span class="name">{recipe.name}</span>
              <span class="meta">
                {count(recipe.servings, 'personne', 'personnes')}
                {#if recipe.ingredients > 0}
                  · {count(recipe.ingredients, 'ingrédient', 'ingrédients')}
                {/if}
                {#if recipe.sub_recipes > 0}
                  · {count(recipe.sub_recipes, 'sous-recette', 'sous-recettes')}
                {/if}
              </span>
            </span>
            {#if recipe.yields !== null}
              <span class="yield">{formatQuantity(recipe.yields)}</span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  </Screen>
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

  .empty {
    margin: var(--space-6) 0;
    color: var(--text-muted);
    text-align: center;
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li + li {
    border-top: 1px solid var(--border);
  }

  li button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3);
    min-height: var(--tapsize);
    border: 0;
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }

  li button:active {
    background: var(--surface-sunken);
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    color: var(--text-muted);
    font-size: var(--text-xs);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .yield {
    flex: none;
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--accent-soft);
    color: var(--accent);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
    font-family: var(--font-numeric);
  }
</style>
