<script lang="ts">
  /**
   * What was edited and deleted, and by whom.
   *
   * It exists for one question — "where did that recipe go?" — which the data
   * cannot answer on its own: once a thing is deleted there is no field left
   * to hold who deleted it, so those two verbs are recorded and nothing else
   * is (DECISIONS 0024). Creations are absent on purpose; they are already
   * attributed on the object.
   *
   * A courtesy, capped at two hundred lines, and **not** an audit trail: the
   * family shares one key, so any device can write any of these under any
   * name (Rule 7). The screen says so at the bottom rather than pretending
   * otherwise.
   */
  import Screen from '../components/Screen.svelte';
  import { relativeTime } from '../lib/format';
  import { ACTION_LABEL, SUBJECT_LABEL } from '../lib/labels';
  import type { Session } from '../lib/session.svelte';

  let { session, onback }: { session: Session; onback: () => void } = $props();

  /** Already newest first: the core reverses the log, which is in merge order
   *  rather than clock order — two devices' clocks disagree, and sorting by a
   *  time nobody agrees on would invent a timeline (DECISIONS 0024). */
  let events = $derived(session.state.events);
</script>

<Screen title="Journal">
  {#snippet actions()}
    <button type="button" class="back" onclick={onback}>Retour</button>
  {/snippet}

  {#if events.length === 0}
    <p class="empty">
      Rien pour l'instant. Les modifications et les suppressions apparaîtront ici.
    </p>
  {:else}
    <ul>
      {#each events as event, index (`${event.at}-${index}`)}
        <li>
          <p class="what">
            <span class="who">{event.by ?? 'Quelqu’un'}</span>
            <!-- Marked rather than reworded: "Vous avez" and "Alexis a" do not
                 conjugate the same, and one sentence per pair of tags is a
                 table that grows by multiplication. -->
            {#if event.by_me}<span class="tag">vous</span>{/if}
            {ACTION_LABEL[event.action]}
            {SUBJECT_LABEL[event.subject]}
            <span class="label">{event.label}</span>
          </p>
          <p class="when">{relativeTime(event.at)}</p>
        </li>
      {/each}
    </ul>
  {/if}

  <p class="note">
    Les deux cents dernières modifications, et seulement celles que la liste elle-même ne peut pas
    montrer. Les noms disent qui l'a probablement fait : la clé est partagée, donc rien ici n'est
    une preuve.
  </p>
</Screen>

<style>
  .back {
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font-size: var(--text-sm);
    cursor: pointer;
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
  }

  li + li {
    margin-top: var(--space-2);
  }

  .what {
    margin: 0;
    font-size: var(--text-sm);
  }

  .who,
  .label {
    font-weight: var(--weight-semibold);
  }

  .tag {
    padding: 0 var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--accent-soft);
    color: var(--accent);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
  }

  .when {
    flex: none;
    margin: 0;
    color: var(--text-faint);
    font-size: var(--text-xs);
    white-space: nowrap;
  }

  .empty {
    margin: 0;
    color: var(--text-muted);
  }

  .note {
    margin: var(--space-4) 0 0;
    color: var(--text-faint);
    font-size: var(--text-sm);
  }
</style>
