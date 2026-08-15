<script module lang="ts">
  /**
   * One thing chosen out of many, by typing part of its name.
   *
   * A `<select>` cannot do this, and that is the whole reason this exists
   * (DECISIONS 0058): the native control offers the list in the order it was
   * given and no way to narrow it, so choosing an ingredient out of a hundred
   * is a scroll through a hundred. It also draws itself — a dropdown wider
   * than the line it sits on, an arrow padded by whoever wrote the browser —
   * which is the other half of what was wrong with it.
   *
   * What replaces it is a text field that filters, and a list underneath. The
   * field is a real `<input>` so `required` still means required, and the
   * panel is ours so it aligns with everything else on the line.
   */
  export type PickerOption = {
    id: string;
    name: string;
    /** Beside the name, dimmed: what tells two similar rows apart. */
    hint?: string;
    /** Searched as well as the name — an ingredient's aliases. */
    terms?: readonly string[];
  };
</script>

<script lang="ts">
  import { tick, type Snippet } from 'svelte';

  import { matches } from '../lib/format';
  import { keyboard, reveal } from '../lib/keyboard.svelte';

  let {
    options,
    value = $bindable(),
    label = undefined,
    name = 'Choisir',
    placeholder = 'Rechercher…',
    required = false,
    trailing = undefined,
    empty = 'Aucun résultat.',
    doorLabel = undefined,
    ondoor = undefined,
  }: {
    options: readonly PickerOption[];
    /** The chosen option's id, `''` for none. */
    value: string;
    /** Shown above the field. Without one the field only names itself. */
    label?: string | undefined;
    /** What the field is called to a screen reader, and in a test. */
    name?: string;
    placeholder?: string;
    /** Native, and it holds: the field carries the chosen name as its value. */
    required?: boolean;
    /** What sits beside the field — a recipe line's remove button. */
    trailing?: Snippet | undefined;
    empty?: string;
    /** The last row of the panel, when there is somewhere else to go. */
    doorLabel?: string | undefined;
    ondoor?: (() => void) | undefined;
  } = $props();

  /** So the field can name the list it controls, once per instance. */
  const listId = $props.id();

  let open = $state(false);
  let query = $state('');
  /** Which row Enter would take. Reset whenever the list changes underneath. */
  let active = $state(0);

  let root = $state<HTMLElement | null>(null);
  let field = $state<HTMLInputElement | null>(null);
  let panel = $state<HTMLElement | null>(null);

  let chosen = $derived(options.find((option) => option.id === value) ?? null);

  let candidates = $derived(
    options.filter((option) => matches([option.name, ...(option.terms ?? [])].join(' '), query)),
  );

  // Clamped rather than reset on every keystroke: the list shrinks as it is
  // typed into, and an index left pointing past the end would make Enter do
  // nothing on a field showing exactly one answer.
  let highlighted = $derived(Math.min(active, Math.max(candidates.length - 1, 0)));

  /**
   * Closed shows what was chosen; open shows what is being typed.
   *
   * Which is why this is not `bind:value`: the field displays two different
   * things, and the one the form validates — and submits with — is the name
   * of a real option. Tapping in clears it to search and the placeholder
   * carries the current choice, so nothing is lost by looking.
   */
  let display = $derived(open ? query : (chosen?.name ?? ''));

  function show(): void {
    if (open) return;
    query = '';
    active = 0;
    open = true;
  }

  /**
   * The arrow opens what the field opens, and closes it again.
   *
   * `focus()` alone would not do: Escape closes the panel and leaves the field
   * focused, and focusing an already-focused element fires nothing — so the
   * arrow would answer the first press and ignore every one after it.
   */
  function toggle(): void {
    if (open) {
      close();
      return;
    }
    field?.focus();
    show();
  }

  function close(): void {
    open = false;
    query = '';
    active = 0;
  }

  function choose(id: string): void {
    value = id;
    close();
    // The keyboard goes with the panel. Left up it would cover the next field
    // of a form the person has not reached yet, and typing into a field whose
    // panel has closed would only open it again.
    field?.blur();
  }

  function door(): void {
    close();
    field?.blur();
    ondoor?.();
  }

  function search(event: Event & { currentTarget: HTMLInputElement }): void {
    query = event.currentTarget.value;
    active = 0;
    open = true;
  }

  /**
   * Enter chooses, and never submits.
   *
   * The panel opens inside the list's add form and inside the one big form the
   * recipe editor is, so an Enter that reached the form would save a recipe
   * halfway through naming a line — the same trap `IngredientForm` documents
   * (DECISIONS 0056), reached through a different control.
   */
  function key(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      close();
      return;
    }
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      show();
      const step = event.key === 'ArrowDown' ? 1 : -1;
      if (candidates.length > 0) {
        active = (highlighted + step + candidates.length) % candidates.length;
      }
      return;
    }
    if (event.key !== 'Enter') return;
    event.preventDefault();
    const picked = candidates[highlighted];
    if (open && picked !== undefined) choose(picked.id);
  }

  /**
   * Keeps the focus in the field while a row is being pressed.
   *
   * A pointer landing on a button blurs the input, which closes the panel, and
   * the click then arrives at nothing. Every picker in every framework solves
   * it here, and this is the only place in the app that needs it.
   */
  function hold(event: MouseEvent): void {
    event.preventDefault();
  }

  /**
   * Anywhere else is a way out.
   *
   * Deliberately in the capture phase and before the click: tapping "save"
   * with a half-typed query closes the panel first, so the field goes back to
   * showing the chosen name — which is empty when nothing was chosen, which is
   * what makes `required` refuse the submit that follows.
   */
  $effect(() => {
    if (!open) return;
    const outside = (event: PointerEvent): void => {
      if (root !== null && !root.contains(event.target as Node)) close();
    };
    window.addEventListener('pointerdown', outside, true);
    return () => window.removeEventListener('pointerdown', outside, true);
  });

  /**
   * The panel opens because something was typed, which means the keyboard is
   * already up and the field is sitting just above it — so the list is drawn
   * straight into the keys. The mention picker in `RecipeEditor` has the same
   * problem and the same answer; neither can be solved in CSS, because to the
   * layout viewport there is plenty of room down there (DECISIONS 0040).
   */
  $effect(() => {
    if (!open) return;
    const covered = keyboard.inset;
    void tick().then(() => {
      if (panel !== null) reveal(panel, covered);
    });
  });
</script>

<!-- Not `.picker`: the recipe editor's "@" mention list owns that name in the
     same document, and `ui-test` queries the DOM globally. -->
<div class="search-picker" bind:this={root}>
  {#if label !== undefined}
    <span class="label">{label}</span>
  {/if}

  <div class="row">
    <div class="field" class:open>
      <input
        bind:this={field}
        type="text"
        value={display}
        {required}
        role="combobox"
        aria-label={label ?? name}
        aria-expanded={open}
        aria-controls={listId}
        aria-autocomplete="list"
        placeholder={open && chosen !== null ? chosen.name : placeholder}
        autocomplete="off"
        autocapitalize="off"
        autocorrect="off"
        spellcheck="false"
        onfocus={show}
        onclick={show}
        oninput={search}
        onkeydown={key}
      />
      <button
        type="button"
        class="chevron"
        class:up={open}
        tabindex="-1"
        aria-label={open ? 'Fermer la liste' : 'Voir la liste'}
        onmousedown={hold}
        onclick={toggle}>▾</button
      >
    </div>
    {@render trailing?.()}
  </div>

  {#if open}
    <ul class="options" id={listId} bind:this={panel}>
      {#each candidates as option, index (option.id)}
        <li>
          <button
            type="button"
            class:on={index === highlighted}
            class:chosen={option.id === value}
            onmousedown={hold}
            onclick={() => choose(option.id)}
          >
            <span class="option-name">{option.name}</span>
            {#if option.hint !== undefined}<span class="hint">{option.hint}</span>{/if}
          </button>
        </li>
      {/each}

      {#if candidates.length === 0}
        <li class="none">{empty}</li>
      {/if}

      {#if doorLabel !== undefined && ondoor !== undefined}
        <li><button type="button" class="door" onmousedown={hold} onclick={door}>{doorLabel}</button></li>
      {/if}
    </ul>
  {/if}
</div>

<style>
  .search-picker {
    /* The panel is in flow rather than floating: a phone has no room for an
       overlay that can be scrolled out from under, and the layout below simply
       moves down while the list is open. */
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    min-width: 0;
  }

  .label {
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  /* The field and whatever sits beside it, on one baseline. The label is
     outside this row on purpose: a trailing button aligns with the control,
     not with the words above it. */
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }

  .field {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
  }

  .field.open {
    border-color: var(--accent);
  }

  input {
    flex: 1;
    min-width: 0;
    padding: var(--space-3);
    border: 0;
    border-radius: var(--radius-md);
    background: none;
    font-weight: var(--weight-normal);
  }

  input:focus {
    outline: none;
  }

  /* The arrow the native control drew badly. Sized as a tap target and padded
     by us, which is the point of drawing it. */
  .chevron {
    flex: none;
    width: var(--space-6);
    align-self: stretch;
    display: grid;
    place-items: center;
    padding: 0 var(--space-2) 0 0;
    border: 0;
    background: none;
    color: var(--text-muted);
    font-size: var(--text-sm);
    line-height: 1;
    cursor: pointer;
    transition: transform var(--duration-fast) var(--ease-out);
  }

  .chevron.up {
    transform: rotate(180deg);
  }

  .options {
    margin: 0;
    padding: 0;
    max-height: var(--picker-height);
    overflow-y: auto;
    /* The room to leave between the list and the top of the keyboard, read by
       `reveal` — which is scrolling for a viewport the browser believes is not
       obscured. A visual value, so it stays in CSS (Rule 10). */
    scroll-margin-bottom: var(--space-4);
    list-style: none;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    box-shadow: var(--shadow-md);
  }

  .options li + li {
    border-top: 1px solid var(--border);
  }

  .options button {
    width: 100%;
    min-height: var(--tapsize);
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border: 0;
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }

  .options button.on {
    background: var(--surface-sunken);
  }

  .options button.chosen .option-name {
    font-weight: var(--weight-semibold);
  }

  .option-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hint {
    flex: none;
    color: var(--text-muted);
    font-size: var(--text-xs);
  }

  .door {
    color: var(--accent);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  .none {
    padding: var(--space-3);
    color: var(--text-muted);
    font-size: var(--text-sm);
  }
</style>
