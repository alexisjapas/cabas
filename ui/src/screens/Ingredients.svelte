<script lang="ts">
  import Screen from '../components/Screen.svelte';
  import type { AisleTag } from '../lib/bindings/AisleTag';
  import type { IngredientView } from '../lib/bindings/IngredientView';
  import { AISLE_LABEL, AISLES } from '../lib/labels';
  import type { Session } from '../lib/session.svelte';

  /**
   * The library: the canonical ingredients everything else refers to.
   *
   * The two coefficient fields are the interesting part. Without a density,
   * mass and volume of the same ingredient stay on separate cart lines
   * forever — that is the rule, not a bug (Rule 5). Filling one in is how you
   * tell the app that 300 g of flour and 2 tablespoons of it can be added up.
   */
  let { session }: { session: Session } = $props();

  let ingredients = $derived(
    [...session.state.ingredients].sort((a, b) => a.name.localeCompare(b.name, 'fr')),
  );

  type Draft = {
    id: string | null;
    name: string;
    aliases: string;
    aisle: AisleTag;
    staple: boolean;
    density: string;
    unitWeight: string;
  };

  function blank(): Draft {
    return { id: null, name: '', aliases: '', aisle: 'grocery', staple: false, density: '', unitWeight: '' };
  }

  function from(ingredient: IngredientView): Draft {
    return {
      id: ingredient.id,
      name: ingredient.name,
      aliases: ingredient.aliases.join(', '),
      aisle: ingredient.aisle,
      staple: ingredient.staple,
      density: ingredient.density ?? '',
      unitWeight: ingredient.unit_weight ?? '',
    };
  }

  let draft = $state<Draft | null>(null);
  let confirmingDelete = $state(false);

  function open(ingredient: IngredientView | null): void {
    draft = ingredient === null ? blank() : from(ingredient);
    confirmingDelete = false;
  }

  /** Empty means "not known", which is not the same as zero (Rule 5). */
  function orNull(text: string): string | null {
    const trimmed = text.trim();
    return trimmed === '' ? null : trimmed;
  }

  function save(event: SubmitEvent): void {
    event.preventDefault();
    if (draft === null || draft.name.trim() === '') return;
    const accepted = session.run({
      command: 'save_ingredient',
      ingredient: {
        id: draft.id,
        name: draft.name.trim(),
        aliases: draft.aliases
          .split(',')
          .map((alias) => alias.trim())
          .filter((alias) => alias !== ''),
        aisle: draft.aisle,
        staple: draft.staple,
        density: orNull(draft.density),
        unit_weight: orNull(draft.unitWeight),
      },
    });
    if (accepted) draft = null;
  }

  function remove(): void {
    if (draft === null || draft.id === null) return;
    if (!confirmingDelete) {
      confirmingDelete = true;
      return;
    }
    // Recipes still using it are not rewritten — the dangling reference is
    // reported and rendered as a warning, never a reason to refuse
    // (DECISIONS 0022).
    if (session.run({ command: 'delete_ingredient', ingredient: draft.id })) draft = null;
  }
</script>

<Screen
  title="Ingrédients"
  subtitle={ingredients.length === 0 ? 'Bibliothèque vide' : `${ingredients.length} référencés`}
>
  {#snippet actions()}
    <button type="button" class="add" onclick={() => open(null)}>Nouveau</button>
  {/snippet}

  {#if draft !== null}
    {@const editing = draft}
    <form onsubmit={save}>
      <label>
        Nom
        <input bind:value={editing.name} required placeholder="Farine T55" autocomplete="off" />
      </label>

      <label>
        Autres noms
        <input bind:value={editing.aliases} placeholder="farine, farine de blé" autocomplete="off" />
        <small>Séparés par des virgules.</small>
      </label>

      <label>
        Rayon
        <select bind:value={editing.aisle}>
          {#each AISLES as aisle (aisle)}
            <option value={aisle}>{AISLE_LABEL[aisle]}</option>
          {/each}
        </select>
      </label>

      <label class="check">
        <input type="checkbox" bind:checked={editing.staple} />
        <span>
          Ingrédient de base
          <small>Supposé présent à la maison, donc coché d'avance dans le panier.</small>
        </span>
      </label>

      <div class="pair">
        <label>
          Densité
          <input bind:value={editing.density} inputmode="decimal" placeholder="0,6" autocomplete="off" />
          <small>g/ml — permet de convertir volume et masse.</small>
        </label>
        <label>
          Poids unitaire
          <input bind:value={editing.unitWeight} inputmode="decimal" placeholder="120" autocomplete="off" />
          <small>g/pièce — permet de convertir pièces et masse.</small>
        </label>
      </div>

      <div class="buttons">
        <button type="submit" class="submit">Enregistrer</button>
        <button type="button" class="cancel" onclick={() => (draft = null)}>Annuler</button>
      </div>

      {#if editing.id !== null}
        <button type="button" class="delete" class:confirming={confirmingDelete} onclick={remove}>
          {confirmingDelete ? 'Confirmer la suppression' : 'Supprimer'}
        </button>
      {/if}
    </form>
  {/if}

  {#if ingredients.length === 0 && draft === null}
    <p class="empty">Aucun ingrédient. Créez le premier avec « Nouveau ».</p>
  {/if}

  <ul>
    {#each ingredients as ingredient (ingredient.id)}
      <li>
        <button type="button" onclick={() => open(ingredient)}>
          <span class="text">
            <span class="name">{ingredient.name}</span>
            <span class="meta">
              {AISLE_LABEL[ingredient.aisle]}
              {#if ingredient.aliases.length > 0}· {ingredient.aliases.join(', ')}{/if}
            </span>
          </span>
          {#if ingredient.staple}<span class="badge">base</span>{/if}
        </button>
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

  input,
  select {
    padding: var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-weight: var(--weight-normal);
  }

  small {
    color: var(--text-muted);
    font-weight: var(--weight-normal);
  }

  .check {
    flex-direction: row;
    align-items: flex-start;
    gap: var(--space-3);
  }

  .check input {
    flex: none;
    width: var(--space-5);
    height: var(--space-5);
    margin: 0;
    accent-color: var(--accent);
  }

  .check span {
    display: flex;
    flex-direction: column;
  }

  .pair {
    display: flex;
    gap: var(--space-3);
  }

  .pair label {
    flex: 1;
    min-width: 0;
  }

  .pair input {
    min-width: 0;
  }

  .buttons {
    display: flex;
    gap: var(--space-2);
  }

  .submit {
    flex: 1;
    padding: var(--space-3);
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--on-accent);
    font-weight: var(--weight-semibold);
    cursor: pointer;
  }

  .cancel {
    flex: none;
    padding: var(--space-3) var(--space-4);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    cursor: pointer;
  }

  .delete {
    padding: var(--space-2);
    border: 1px solid var(--danger);
    border-radius: var(--radius-md);
    background: none;
    color: var(--danger);
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .delete.confirming {
    background: var(--danger);
    color: var(--on-danger);
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

  .badge {
    flex: none;
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--accent-soft);
    color: var(--accent);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
  }
</style>
