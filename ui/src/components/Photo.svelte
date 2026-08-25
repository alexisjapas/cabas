<script lang="ts">
  /**
   * One photo, wherever one is shown (DECISIONS 0062).
   *
   * The document names a photo; the bytes live beside it and are fetched here.
   * Three states, and the third is the one worth designing for:
   *
   * - no id — nothing to show, so nothing is drawn at all;
   * - held — the picture;
   * - **named but not here yet** — the photo the other phone took, whose bytes
   *   have not arrived. It renders as a quiet frame, never as an error and
   *   never as a broken image (Rule 6).
   *
   * The object URL is created and revoked by the effect that made it, so
   * nothing leaks and there is no cache to keep coherent: Svelte reuses this
   * component while its `photo` prop is unchanged, so a cart that re-renders
   * on every tick re-reads nothing.
   */
  import type { Session } from '../lib/session.svelte';

  let {
    session,
    photo,
    alt = '',
    size = 'thumb',
  }: {
    session: Session;
    photo: string | null;
    /** What a screen reader says. The name of what is pictured, or nothing. */
    alt?: string;
    /** `thumb` on a row, `full` where the photo is the subject. */
    size?: 'thumb' | 'full';
  } = $props();

  let url = $state<string | null>(null);

  $effect(() => {
    const id = photo;
    url = null;
    if (id === null) return;

    let current = true;
    let created: string | undefined;

    void session
      .photo(id)
      .then((bytes) => {
        // The effect may have been torn down or pointed at another photo
        // while the read was in flight; either way this answer is stale.
        if (!current || bytes === undefined) return;
        created = URL.createObjectURL(new Blob([bytes], { type: 'image/jpeg' }));
        url = created;
      })
      .catch(() => {
        // Storage refused to answer. A missing photo is already a state this
        // renders, so it renders that one rather than shouting.
      });

    return () => {
      current = false;
      if (created !== undefined) URL.revokeObjectURL(created);
    };
  });
</script>

{#if photo !== null}
  <span class="photo {size}" class:waiting={url === null} data-photo={photo}>
    {#if url !== null}
      <img src={url} {alt} loading="lazy" />
    {/if}
  </span>
{/if}

<style>
  .photo {
    display: block;
    flex: none;
    overflow: hidden;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
  }

  .thumb {
    width: var(--photo-thumb);
    height: var(--photo-thumb);
  }

  .full {
    width: 100%;
    aspect-ratio: 4 / 3;
    border-radius: var(--radius-lg);
  }

  img {
    display: block;
    width: 100%;
    height: 100%;
    /* The frame is fixed and photos are not: filling it beats letterboxing a
       row of them into different shapes. */
    object-fit: cover;
  }

  /* Named by the document, not here yet — the other phone took it and the
     bytes are still on their way. A quiet frame, deliberately not a spinner:
     there is nothing to wait for on this device. */
  .waiting {
    border: 1px dashed var(--border);
  }
</style>
