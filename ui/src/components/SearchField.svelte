<script lang="ts">
  /**
   * A field that narrows a list on the screen it sits on.
   *
   * The other half of DECISIONS 0058: a picker searches what is about to be
   * chosen, and this searches what is already on display — the library and the
   * shelf of recipes, both of which are lists nobody scrolls twice.
   *
   * `type="search"` rather than `type="text"`: it is what gets the search key
   * on a soft keyboard and the browser's own clear button, and neither is
   * worth reimplementing.
   */
  let {
    value = $bindable(),
    placeholder = 'Rechercher…',
  }: {
    value: string;
    placeholder?: string;
  } = $props();
</script>

<div class="search-field">
  <input
    type="search"
    {value}
    {placeholder}
    aria-label={placeholder}
    autocomplete="off"
    autocapitalize="off"
    autocorrect="off"
    spellcheck="false"
    oninput={(event) => (value = event.currentTarget.value)}
  />
</div>

<style>
  .search-field {
    margin-bottom: var(--space-3);
  }

  /* Cream and pill-shaped (DECISIONS 0081). The page behind it is a
     tablecloth now: a field painted in the page colour would be indistinct
     from it, and one painted in the checks would be unreadable. */
  input {
    width: 100%;
    padding: var(--space-3) var(--space-4);
    border: 2px solid var(--border);
    border-radius: var(--radius-pill);
    background: var(--bubble);
    font-weight: var(--weight-normal);
  }

  input:focus {
    border-color: var(--accent);
  }
</style>
