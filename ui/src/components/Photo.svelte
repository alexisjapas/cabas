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
    /**
     * `thumb` on a row, `dish` on a recipe row, `full` where the photo is the
     * subject. `dish` is its own size rather than a bigger `thumb` because it
     * is also a different shape — see the token (DECISIONS 0086).
     */
    size?: 'thumb' | 'dish' | 'full';
  } = $props();

  let url = $state<string | null>(null);

  $effect(() => {
    const id = photo;
    // Read, not used: it is what makes a placeholder become a picture the
    // moment the transfer lands one (DECISIONS 0092). The counter changes,
    // this effect re-runs, and a photo that has just arrived is read again.
    // Photos already held cost one IndexedDB read each; ones still missing
    // draw nothing either way.
    void session.photos.generation;
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
  /* The pink ring is the shape a photo takes in this app (DECISIONS 0081):
     round on a row, a rounded 4/3 frame where the picture is the subject.
     `box-sizing: border-box` means the ring eats into the fixed thumb size
     rather than growing the row. */
  .photo {
    display: block;
    flex: none;
    overflow: hidden;
    background: var(--surface-sunken);
  }

  .thumb {
    width: var(--photo-thumb);
    height: var(--photo-thumb);
    border: var(--photo-ring) solid var(--ring);
    border-radius: var(--radius-pill);
  }

  /* The dish on a shelf row: square, and rounded like a card rather than into
     a bead. A round crop of a plate throws away the corners of the one photo
     in the app that is *of* something rather than about it (DECISIONS 0086). */
  .dish {
    width: var(--photo-dish);
    height: var(--photo-dish);
    border: var(--photo-ring) solid var(--ring);
    border-radius: var(--radius-md);
  }

  /* Where the photo is the subject: the top of a recipe being read. Wider than
     4/3 and carrying the flat shadow every raised thing here does, because
     this is the first thing on the screen and it should read as one
     (DECISIONS 0086). */
  .full {
    width: 100%;
    aspect-ratio: 3 / 2;
    border: var(--photo-ring-full) solid var(--ring);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-md);
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
     bytes are still on their way. A quiet dashed frame, deliberately not a
     spinner and deliberately not the pink ring: there is nothing to wait for
     on this device, and it must not read as a photo that failed. */
  .waiting {
    border-style: dashed;
    border-color: var(--border);
  }
</style>
