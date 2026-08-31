<script lang="ts">
  /**
   * The library, out to a file and back in (DECISIONS 0076).
   *
   * Four things people asked of one mechanism: a backup they hold themselves,
   * a way to type a library on a keyboard rather than a thumb, a way to carry
   * everything into a new group, and a way to send recipes to somebody else.
   * All four are the same file, which is why there is one screen and not four.
   *
   * Nothing here reads the file or writes one. The core decides the format,
   * its text and every rule about merging it (Rule 9); this hands over a
   * string and shows what came back.
   */
  import Screen from '../components/Screen.svelte';
  import type { ImportReport } from '../lib/bindings/ImportReport';
  import type { Session } from '../lib/session.svelte';

  let { session, onback }: { session: Session; onback: () => void } = $props();

  /**
   * Off by default, and that is the important half. Without photos the file
   * is small enough to open in a text editor — which is what makes it a
   * document you can correct — and with them it is a backup measured in tens
   * of megabytes (DECISIONS 0062).
   */
  let withPhotos = $state(false);

  let busy = $state(false);
  let done = $state<string | null>(null);
  let report = $state<ImportReport | null>(null);

  let picker: HTMLInputElement;

  /** `cabas-2026-08-26.json` — the date is what tells two backups apart. */
  function filename(): string {
    const now = new Date();
    const pad = (n: number): string => String(n).padStart(2, '0');
    return `cabas-${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}.json`;
  }

  /**
   * Hands the file to the phone.
   *
   * The share sheet first, because on an installed iOS app that is the only
   * route to iCloud, to Files and to somebody else's phone — and a download
   * from a standalone PWA is the awkward one there. It is tried rather than
   * detected: Safari wants `navigator.share` inside the tap that caused it,
   * and the export before it is asynchronous, so the permission can be gone
   * by the time we ask. A refusal falls through to a download; a *cancel*
   * does not, because the person already said no.
   */
  async function deliver(json: string): Promise<void> {
    const name = filename();
    const file = new File([json], name, { type: 'application/json' });

    if (navigator.canShare?.({ files: [file] })) {
      try {
        await navigator.share({ files: [file], title: 'Bibliothèque cabas' });
        return;
      } catch (cause) {
        if (cause instanceof Error && cause.name === 'AbortError') return;
      }
    }

    const url = URL.createObjectURL(file);
    const link = document.createElement('a');
    link.href = url;
    link.download = name;
    link.click();
    // Revoked on the next turn: the click has to have been dispatched first.
    setTimeout(() => URL.revokeObjectURL(url), 0);
  }

  async function exportLibrary(): Promise<void> {
    busy = true;
    done = null;
    report = null;
    try {
      await deliver(await session.exportLibrary(withPhotos));
      // Deliberately vague about *where* it went: the share sheet and the
      // download fallback end in two different places, and only the phone
      // knows which one happened.
      done = 'Fichier exporté.';
    } catch (cause) {
      session.error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      busy = false;
    }
  }

  async function chosen(event: Event): Promise<void> {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    // Clearing it is what lets the same file be picked twice — a second
    // `change` never fires for an unchanged value.
    input.value = '';
    if (file === undefined) return;

    busy = true;
    done = null;
    report = null;
    try {
      report = await session.importLibrary(await file.text());
    } catch {
      // `Session.importLibrary` has already put the reason in the banner;
      // there is nothing to add and nothing to swallow.
    } finally {
      busy = false;
    }
  }

  /** "3 ajoutés, 1 mis à jour", or nothing at all when neither happened. */
  function tally(added: number, updated: number): string | null {
    const parts: string[] = [];
    if (added > 0) parts.push(`${added} ajouté${added > 1 ? 's' : ''}`);
    if (updated > 0) parts.push(`${updated} mis à jour`);
    return parts.length === 0 ? null : parts.join(', ');
  }

  let lines = $derived.by(() => {
    if (report === null) return [];
    const rows: [string, string | null][] = [
      ['Ingrédients', tally(report.ingredients_added, report.ingredients_updated)],
      ['Recettes', tally(report.recipes_added, report.recipes_updated)],
      ['Magasins', tally(report.shops_added, report.shops_updated)],
      ['Photos', report.photos_added > 0 ? `${report.photos_added} reçues` : null],
    ];
    return rows.filter(([, value]) => value !== null) as [string, string][];
  });
</script>

<Screen title="Données" {onback}>
  <section>
    <h2 class="display">Exporter</h2>
    <p class="note">
      Tout ce qui a été saisi — ingrédients, recettes, magasins — dans un seul fichier lisible.
      À garder ailleurs que sur ce téléphone, ou à envoyer à quelqu'un.
    </p>

    <label class="toggle">
      <input type="checkbox" bind:checked={withPhotos} data-field="with-photos" />
      <span>
        Inclure les photos
        <small>
          Le fichier devient beaucoup plus lourd et n'est plus lisible à l'œil. Utile pour une
          sauvegarde, pas pour une relecture.
        </small>
      </span>
    </label>

    <button type="button" disabled={busy} onclick={exportLibrary} data-action="export">
      Exporter la bibliothèque
    </button>
    {#if done !== null}<p class="done">{done}</p>{/if}
  </section>

  <section>
    <h2 class="display">Importer</h2>
    <p class="note">
      Ce que contient le fichier est ajouté, et ce qui existe déjà sous le même nom est mis à
      jour. <strong>Rien n'est jamais supprimé</strong> — ce que le fichier ne mentionne pas est
      laissé tel quel.
    </p>

    <input
      bind:this={picker}
      type="file"
      accept="application/json,.json"
      hidden
      onchange={chosen}
      data-field="import"
    />
    <button
      type="button"
      class="secondary"
      disabled={busy}
      onclick={() => picker.click()}
      data-action="import"
    >
      Choisir un fichier
    </button>

    {#if report !== null}
      <dl data-report>
        {#each lines as [label, value] (label)}
          <div>
            <dt>{label}</dt>
            <dd>{value}</dd>
          </div>
        {/each}
        {#if lines.length === 0}
          <div>
            <dt>Rien à reprendre</dt>
            <dd>—</dd>
          </div>
        {/if}
      </dl>

      {#if report.incomplete.length > 0}
        <p class="warn">
          {report.incomplete.length === 1 ? 'Une recette cite' : 'Des recettes citent'} un
          ingrédient qui n'existe pas ici : {report.incomplete.join(', ')}. La ligne est gardée et
          signalée dans la recette.
        </p>
      {/if}
    {/if}
  </section>

  <p class="note bubble">
    Les deux téléphones du groupe partagent la même bibliothèque : ce qui est importé ici arrive
    aussi sur l'autre.
  </p>
</Screen>

<style>
  /* A tile, and the fields inside it are cream (DECISIONS 0081). */
  section {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    margin-bottom: var(--space-6);
    padding: var(--space-4);
    border: 2px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
  }

  /* `--text` and not `--display-ink`: this heading sits on a lilac tile. */
  h2 {
    margin: 0;
    color: var(--text);
    font-size: var(--text-lg);
  }

  .note {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .toggle {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  .toggle input {
    margin-top: var(--space-1);
    flex: none;
  }

  .toggle small {
    display: block;
    margin-top: var(--space-1);
    color: var(--text-muted);
    font-weight: var(--weight-normal);
  }

  button {
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
  button:disabled {
    background: var(--surface-sunken);
    color: var(--text-muted);
    box-shadow: none;
    cursor: default;
  }

  .secondary {
    border: 2px solid var(--border-strong);
    background: var(--bubble);
    color: var(--text);
    box-shadow: none;
  }

  .done {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  dl {
    margin: 0;
    padding: 0 var(--space-3);
    border-radius: var(--radius-md);
    background: var(--bubble);
  }

  dl div {
    display: flex;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-3) 0;
  }

  dl div + div {
    border-top: 1px solid var(--border);
  }

  dt {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  dd {
    margin: 0;
    font-family: var(--font-numeric);
    font-variant-numeric: tabular-nums;
  }

  .warn {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }
</style>
