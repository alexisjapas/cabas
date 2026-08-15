<script lang="ts">
  import { tick } from 'svelte';

  import IngredientPicker from '../components/IngredientPicker.svelte';
  import QuantityField from '../components/QuantityField.svelte';
  import Screen from '../components/Screen.svelte';
  import type { RecipeInput } from '../lib/bindings/RecipeInput';
  import type { SegmentInput } from '../lib/bindings/SegmentInput';
  import type { StepInput } from '../lib/bindings/StepInput';
  import { mintUsageId } from '../lib/core';
  import { byName } from '../lib/format';
  import { keyboard, reveal } from '../lib/keyboard.svelte';
  import { REF_DISPLAY_LABEL, REF_DISPLAYS } from '../lib/labels';
  import type { Session } from '../lib/session.svelte';

  /**
   * Writing a recipe.
   *
   * The draft **is** a `RecipeInput` — the exact shape `SaveRecipe` takes, and
   * the exact shape `focus.edit` hands back — so editing is filling in a form
   * rather than rebuilding a structure out of rendered text. Nothing here
   * parses a quantity: amounts stay the characters that were typed all the way
   * into the core, which turns them into exact rationals (Rule 4).
   *
   * Every line carries its id from the moment it is added, minted by the core
   * (DECISIONS 0039). That is what lets a step mention a line the recipe has
   * never been saved with, and it is why the whole recipe — lines and the
   * prose pointing at them — goes out in a single command.
   */
  let {
    session,
    recipe = $bindable(),
    onsave,
    oncancel,
  }: {
    session: Session;
    recipe: RecipeInput;
    /** Returns whether it was accepted; a refusal keeps the form open. */
    onsave: (recipe: RecipeInput) => boolean;
    oncancel: () => void;
  } = $props();

  let ingredientNames = $derived(
    new Map(session.state.ingredients.map((ingredient) => [ingredient.id, ingredient.name])),
  );
  /** Every recipe but this one: a recipe containing itself is a cycle. */
  let subRecipes = $derived(
    [...session.state.recipes].filter((summary) => summary.id !== recipe.id).sort(byName),
  );

  // --- the ingredient lines --------------------------------------------------

  function addIngredient(): void {
    recipe.components.push({
      kind: 'ingredient',
      id: mintUsageId(),
      ingredient: '',
      quantity: { amount: '', unit: 'g' },
    });
  }

  function addSubRecipe(): void {
    recipe.components.push({
      kind: 'sub_recipe',
      id: mintUsageId(),
      recipe: '',
      amount: { kind: 'factor', factor: '1' },
    });
  }

  /**
   * Removing a line takes its mentions with it.
   *
   * A step pointing at a line that no longer exists renders as a warning, and
   * that is the right answer when *another device* deleted it (DECISIONS
   * 0022). Leaving one behind here would be manufacturing the same damage on
   * purpose, in a form the person is looking at.
   */
  function removeComponent(index: number): void {
    const [gone] = recipe.components.splice(index, 1);
    if (gone === undefined || gone.id === null) return;
    for (const step of recipe.steps) {
      step.segments = step.segments.filter(
        (segment) => segment.kind !== 'ingredient' || segment.usage !== gone.id,
      );
    }
  }

  function setSubRecipeMode(index: number, mode: 'factor' | 'of_yield'): void {
    const component = recipe.components[index];
    if (component === undefined || component.kind !== 'sub_recipe') return;
    component.amount =
      mode === 'factor'
        ? { kind: 'factor', factor: '1' }
        : { kind: 'of_yield', quantity: { amount: '', unit: 'g' } };
  }

  // --- the steps -------------------------------------------------------------

  function addStep(): void {
    recipe.steps.push({ segments: [{ kind: 'text', text: '' }] });
  }

  function moveStep(index: number, by: number): void {
    const to = index + by;
    if (to < 0 || to >= recipe.steps.length) return;
    const [step] = recipe.steps.splice(index, 1);
    if (step !== undefined) recipe.steps.splice(to, 0, step);
  }

  /** Tapping a mention cycles what it shows where it stands (DECISIONS 0022). */
  function cycleDisplay(step: number, segment: number): void {
    const current = recipe.steps[step]?.segments[segment];
    if (current === undefined || current.kind !== 'ingredient') return;
    const next = (REF_DISPLAYS.indexOf(current.display) + 1) % REF_DISPLAYS.length;
    current.display = REF_DISPLAYS[next] ?? 'full';
  }

  function removeSegment(step: number, segment: number): void {
    recipe.steps[step]?.segments.splice(segment, 1);
  }

  // --- the @ mention ---------------------------------------------------------

  type Mention = {
    step: number;
    segment: number;
    /** Where the "@" sits, and where the caret was — the run to replace. */
    at: number;
    end: number;
    query: string;
  };

  let mention = $state<Mention | null>(null);
  let form = $state<HTMLFormElement | null>(null);

  /** Accent- and case-insensitive, because nobody types "é" to find "épinard". */
  function fold(text: string): string {
    return text
      .normalize('NFD')
      .replace(/\p{Diacritic}/gu, '')
      .toLowerCase();
  }

  /** Only ingredient lines: a step can reference nothing else (`Recipe::usage`). */
  let mentionable = $derived(
    recipe.components.flatMap((component) =>
      component.kind === 'ingredient' && component.ingredient !== '' && component.id !== null
        ? [
            {
              usage: component.id,
              name: ingredientNames.get(component.ingredient) ?? 'Ingrédient inconnu',
            },
          ]
        : [],
    ),
  );

  /** The same lines by id, for the chips — one walk rather than one per chip. */
  let mentionNames = $derived(new Map(mentionable.map((line) => [line.usage, line.name])));

  let candidates = $derived.by(() => {
    if (mention === null) return [];
    const query = fold(mention.query);
    return mentionable.filter((candidate) => fold(candidate.name).includes(query));
  });

  /**
   * A mention is being typed when the caret sits in an unbroken run after an
   * "@" that starts a word. "@far" is a mention in progress; "a@b" is prose,
   * and so is "@deux cuillères" the moment the space is typed.
   */
  function detect(step: number, segment: number, text: string, caret: number): Mention | null {
    const before = text.slice(0, caret);
    const at = before.lastIndexOf('@');
    if (at === -1) return null;
    const preceding = at === 0 ? '' : before.slice(at - 1, at);
    if (preceding !== '' && /[\p{L}\p{N}]/u.test(preceding)) return null;
    const query = before.slice(at + 1);
    if (/\s/.test(query)) return null;
    return { step, segment, at, end: caret, query };
  }

  function editText(step: number, segment: number, event: Event): void {
    const node = event.currentTarget as HTMLTextAreaElement;
    const current = recipe.steps[step]?.segments[segment];
    if (current === undefined || current.kind !== 'text') return;
    current.text = node.value;
    // A recipe with no ingredient lines has nothing to mention, so an "@" in
    // it is prose and offering an empty picker would only be in the way.
    mention =
      mentionable.length === 0
        ? null
        : detect(step, segment, node.value, node.selectionStart ?? node.value.length);
  }

  /**
   * Turns "@far" into a reference, by splitting the text around it.
   *
   * The prose is never re-parsed after this: the mention is a segment of its
   * own from here on, so editing the words around it cannot break it, and
   * deleting it is deleting a thing rather than finding a marker (DECISIONS
   * 0022).
   */
  async function insert(usage: string): Promise<void> {
    if (mention === null) return;
    const { step, segment, at, end } = mention;
    const current = recipe.steps[step]?.segments[segment];
    if (current === undefined || current.kind !== 'text') return;

    const before = current.text.slice(0, at);
    const after = current.text.slice(end);
    recipe.steps[step]?.segments.splice(
      segment,
      1,
      { kind: 'text', text: before },
      { kind: 'ingredient', usage, display: 'full' },
      { kind: 'text', text: after },
    );
    mention = null;

    // Back into the prose, where the caret was, rather than leaving focus on a
    // button that has just disappeared.
    await tick();
    const next = form?.querySelector<HTMLTextAreaElement>(`[data-seg="${step}-${segment + 2}"]`);
    next?.focus();
    next?.setSelectionRange(0, 0);
  }

  /** `null` when the line itself is gone — which is what the chip warns about. */
  function mentionName(usage: string): string | null {
    return mentionNames.get(usage) ?? null;
  }

  /**
   * Keeps the picker out from under the keyboard.
   *
   * It is the one control in the app that appears *because* of what was typed,
   * which is exactly the situation where the keyboard is already up and the
   * caret is on the last line above it — so the list is drawn straight into the
   * keys. No CSS can tell: to the layout viewport there is plenty of room down
   * there (`lib/keyboard.svelte.ts`).
   *
   * It depends on the inset as well as on the mention, because the two arrive
   * in either order. Typing "@" into a field that already has focus opens the
   * picker under a keyboard that is up; tapping into an empty step opens both
   * at once, and the viewport resizes a beat after the focus lands.
   *
   * The node is queried rather than bound, like the textarea in `insert`: one
   * picker exists at a time, but it belongs to whichever step is being written,
   * and a `bind:this` inside an each block is at the mercy of the order Svelte
   * creates and destroys them in.
   */
  $effect(() => {
    if (mention === null) return;
    const covered = keyboard.inset;
    void tick().then(() => {
      const picker = form?.querySelector<HTMLElement>('.picker') ?? null;
      if (picker !== null) reveal(picker, covered);
    });
  });

  /**
   * Grows with what is typed: a step is prose, and prose does not fit one row.
   *
   * The text is passed in so that `update` fires when a split rewrites the
   * value from script — typing refits through the listener, but nothing types
   * when a mention is inserted.
   */
  function autogrow(node: HTMLTextAreaElement, _text: string) {
    const fit = (): void => {
      node.style.height = 'auto';
      node.style.height = `${node.scrollHeight}px`;
    };
    fit();
    node.addEventListener('input', fit);
    return {
      update: (_next: string): void => fit(),
      destroy: (): void => node.removeEventListener('input', fit),
    };
  }

  // --- saving ----------------------------------------------------------------

  /**
   * Drops the empty text segments the splitting leaves behind and rejoins what
   * ends up adjacent, so the saved recipe holds what was written rather than
   * the shape the editing took.
   */
  function normalize(steps: StepInput[]): StepInput[] {
    return steps
      .map((step) => {
        const segments: SegmentInput[] = [];
        for (const segment of step.segments) {
          const last = segments[segments.length - 1];
          if (segment.kind === 'text' && last !== undefined && last.kind === 'text') {
            last.text += segment.text;
          } else {
            segments.push({ ...segment });
          }
        }
        return {
          segments: segments.filter((segment) => segment.kind !== 'text' || segment.text !== ''),
        };
      })
      .filter((step) => step.segments.length > 0);
  }

  function submit(event: SubmitEvent): void {
    event.preventDefault();
    // A snapshot, not the proxy: what crosses to wasm should be a plain value.
    const payload = $state.snapshot(recipe) as RecipeInput;
    payload.name = payload.name.trim();
    payload.steps = normalize(payload.steps);
    onsave(payload);
  }
</script>

<Screen title={recipe.id === null ? 'Nouvelle recette' : 'Modifier'}>
  {#snippet actions()}
    <button type="button" class="cancel" onclick={oncancel}>Annuler</button>
  {/snippet}

  <form bind:this={form} onsubmit={submit}>
    <label>
      Nom
      <input bind:value={recipe.name} required placeholder="Tarte aux tomates" autocomplete="off" />
    </label>

    <fieldset class="servings">
      <legend>Pour</legend>
      <div>
        <button
          type="button"
          aria-label="Moins"
          onclick={() => (recipe.servings = Math.max(1, recipe.servings - 1))}>−</button
        >
        <span>{recipe.servings} pers.</span>
        <button type="button" aria-label="Plus" onclick={() => (recipe.servings += 1)}>+</button>
      </div>
    </fieldset>

    <label class="check">
      <input
        type="checkbox"
        checked={recipe.yields !== null}
        onchange={(event) =>
          (recipe.yields = event.currentTarget.checked ? { amount: '', unit: 'g' } : null)}
      />
      <span>
        Rendement
        <small>Ce que la recette produit. Nécessaire pour en prendre une quantité dans une autre recette.</small>
      </span>
    </label>

    {#if recipe.yields !== null}
      {@const yields = recipe.yields}
      <QuantityField bind:amount={yields.amount} bind:unit={yields.unit} label="Donne" required />
    {/if}

    <!-- the ingredient lines -->

    <h2>Ingrédients</h2>

    {#each recipe.components as component, index (component.id)}
      <div class="line">
        {#snippet removeLine()}
          <button
            type="button"
            class="remove"
            aria-label="Retirer la ligne"
            onclick={() => removeComponent(index)}>×</button
          >
        {/snippet}

        {#if component.kind === 'ingredient'}
          {@const quantity = component.quantity}
          <!-- The picker is keyed with the line, so removing one takes its own
               half-typed ingredient and leaves every other line's alone. -->
          <IngredientPicker {session} bind:value={component.ingredient} required trailing={removeLine} />
          <QuantityField bind:amount={quantity.amount} bind:unit={quantity.unit} label="Quantité" required />
        {:else}
          {@const amount = component.amount}
          <div class="linehead">
            <select bind:value={component.recipe} required aria-label="Sous-recette">
              <option value="" disabled>Choisir…</option>
              {#each subRecipes as summary (summary.id)}
                <option value={summary.id}>{summary.name}</option>
              {/each}
            </select>
            {@render removeLine()}
          </div>
          <div class="modes">
            <button
              type="button"
              class:on={amount.kind === 'factor'}
              onclick={() => setSubRecipeMode(index, 'factor')}>Multiple</button
            >
            <button
              type="button"
              class:on={amount.kind === 'of_yield'}
              onclick={() => setSubRecipeMode(index, 'of_yield')}>Quantité</button
            >
          </div>
          {#if amount.kind === 'factor'}
            <label class="factor">
              Multiple
              <input
                bind:value={amount.factor}
                inputmode="decimal"
                placeholder="0,5"
                required
                autocomplete="off"
              />
              <small>La recette entière, ou une fraction : 0,5 pour la moitié.</small>
            </label>
          {:else}
            {@const quantity = amount.quantity}
            <QuantityField
              bind:amount={quantity.amount}
              bind:unit={quantity.unit}
              label="Quantité prise"
              required
            />
          {/if}
        {/if}
      </div>
    {/each}

    <div class="adders">
      <button type="button" onclick={addIngredient}>+ Ingrédient</button>
      <button type="button" onclick={addSubRecipe} disabled={subRecipes.length === 0}
        >+ Sous-recette</button
      >
    </div>

    <!-- the steps -->

    <h2>Préparation</h2>
    {#if mentionable.length > 0}
      <p class="hint">
        Tapez « @ » pour citer un ingrédient de la recette. Touchez une citation pour changer ce
        qu'elle affiche.
      </p>
    {/if}

    {#each recipe.steps as step, index (index)}
      <div class="step">
        <div class="stepbar">
          <span class="number">{index + 1}</span>
          <button type="button" aria-label="Monter" onclick={() => moveStep(index, -1)}>↑</button>
          <button type="button" aria-label="Descendre" onclick={() => moveStep(index, 1)}>↓</button>
          <button
            type="button"
            class="remove"
            aria-label="Retirer l'étape"
            onclick={() => recipe.steps.splice(index, 1)}>×</button
          >
        </div>

        <div class="segments">
          {#each step.segments as segment, position (position)}
            {#if segment.kind === 'text'}
              <textarea
                rows="1"
                data-seg="{index}-{position}"
                value={segment.text}
                use:autogrow={segment.text}
                oninput={(event) => editText(index, position, event)}
                onkeydown={(event) => {
                  if (event.key === 'Escape') mention = null;
                }}
                aria-label="Texte de l'étape {index + 1}"
              ></textarea>
            {:else}
              {@const name = mentionName(segment.usage)}
              <span class="chip" class:gone={name === null}>
                <button
                  type="button"
                  class="cycle"
                  title={REF_DISPLAY_LABEL[segment.display]}
                  onclick={() => cycleDisplay(index, position)}
                >
                  {name ?? 'ligne supprimée'}
                  <small>{REF_DISPLAY_LABEL[segment.display]}</small>
                </button>
                <button
                  type="button"
                  class="unchip"
                  aria-label="Retirer la citation"
                  onclick={() => removeSegment(index, position)}>×</button
                >
              </span>
            {/if}
          {/each}
        </div>

        {#if mention !== null && mention.step === index}
          <ul class="picker">
            {#if candidates.length === 0}
              <li class="none">Aucun ingrédient ne correspond.</li>
            {/if}
            {#each candidates as candidate (candidate.usage)}
              <li>
                <button type="button" onclick={() => insert(candidate.usage)}>{candidate.name}</button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    {/each}

    <div class="adders">
      <button type="button" onclick={addStep}>+ Étape</button>
    </div>

    <button type="submit" class="submit">Enregistrer</button>
  </form>
</Screen>

<style>
  .cancel {
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

  h2 {
    margin: var(--space-3) 0 0;
    font-size: var(--text-sm);
    font-weight: var(--weight-semibold);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .hint {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .servings {
    margin: 0;
    padding: 0;
    border: 0;
  }

  .servings legend {
    padding: 0 0 var(--space-2);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  .servings div {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .servings button {
    width: var(--tapsize);
    height: var(--space-6);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-raised);
    cursor: pointer;
  }

  .servings span {
    font-variant-numeric: tabular-nums;
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

  .line {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
  }

  .linehead {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .linehead select {
    flex: 1;
    min-width: 0;
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

  .factor input {
    font-family: var(--font-numeric);
  }

  .adders {
    display: flex;
    gap: var(--space-2);
  }

  .adders button {
    flex: 1;
    padding: var(--space-3);
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-md);
    background: none;
    color: var(--text-muted);
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .adders button:disabled {
    opacity: 0.5;
  }

  .step {
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
  }

  .stepbar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
  }

  .number {
    flex: 1;
    color: var(--text-faint);
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
  }

  .stepbar button {
    flex: none;
    width: var(--space-6);
    height: var(--space-6);
    border: 0;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-muted);
    cursor: pointer;
  }

  /* The segments read left to right like the sentence they are. A lone text
     segment takes the whole width and grows downwards; short runs between two
     mentions sit inline. */
  .segments {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-1);
  }

  textarea {
    flex: 1 1 8ch;
    min-width: 8ch;
    padding: var(--space-2);
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: inherit;
    font: inherit;
    line-height: var(--leading-normal);
    resize: none;
    overflow: hidden;
  }

  textarea:focus {
    border-color: var(--accent);
    outline: none;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    border-radius: var(--radius-pill);
    background: var(--accent-soft);
    color: var(--accent);
  }

  .chip.gone {
    background: var(--danger-soft);
    color: var(--danger);
  }

  .cycle {
    display: flex;
    flex-direction: column;
    padding: var(--space-1) var(--space-2);
    border: 0;
    background: none;
    color: inherit;
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    text-align: left;
    cursor: pointer;
  }

  .cycle small {
    color: inherit;
    opacity: 0.7;
    font-size: var(--text-xs);
  }

  .unchip {
    padding: 0 var(--space-2) 0 0;
    border: 0;
    background: none;
    color: inherit;
    cursor: pointer;
  }

  .picker {
    margin: var(--space-2) 0 0;
    padding: 0;
    /* The room to leave between the list and the top of the keyboard. Read by
       `reveal`, which is scrolling for a viewport the browser thinks is not
       obscured — but the value is a visual one and stays here (Rule 10). */
    scroll-margin-bottom: var(--space-4);
    list-style: none;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    box-shadow: var(--shadow-md);
    overflow: hidden;
  }

  .picker li + li {
    border-top: 1px solid var(--border);
  }

  .picker button {
    width: 100%;
    padding: var(--space-3);
    min-height: var(--tapsize);
    border: 0;
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }

  .picker .none {
    padding: var(--space-3);
    color: var(--text-muted);
    font-size: var(--text-sm);
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
</style>
