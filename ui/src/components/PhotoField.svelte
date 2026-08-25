<script lang="ts">
  /**
   * Taking a photo, wherever one is attached — an ingredient's form, a
   * recipe's (DECISIONS 0062).
   *
   * # It is not a `<form>`, and it lives inside several
   *
   * Same constraint as `IngredientForm`: this renders inside the list's add
   * panel and inside the one big form the recipe editor is. So every button
   * says `type="button"`, and the file input is `hidden` and driven by a
   * button rather than being a control the surrounding form can submit.
   *
   * # The camera is the OS's, not ours
   *
   * `capture="environment"` asks for the back camera and the OS answers with
   * its own picker — no permission prompt of ours, no video element, nothing
   * that rots across iOS versions (the reasoning of DECISIONS 0047, restated
   * in 0062).
   *
   * # Two inputs, because `capture` is not a suggestion
   *
   * An input carrying `capture` opens the camera and *only* the camera on a
   * phone: the photo already in the roll — the one taken last week, the one
   * received from someone else, the label photographed in the shop before
   * cabas was open — is unreachable through it. Dropping the attribute
   * instead would take the camera away from the common case. So there are two
   * hidden inputs and two buttons, which is also why the attribute is not
   * flipped on one input between clicks: `capture` is read when the picker
   * opens, and an input that means something different depending on which
   * button was last pressed is a bug waiting for a slow phone (DECISIONS
   * 0065).
   *
   * # The bytes are stored before the form is saved
   *
   * `putPhoto` writes the photo and hands back an id, and that id is what the
   * draft carries. A form abandoned afterwards therefore leaves a photo
   * nothing references — which is what the sweep in `Photos::forget_
   * unreferenced` is for, and why it is not an error.
   */
  import Photo from './Photo.svelte';
  import { encodePhoto } from '../lib/photo';
  import type { Session } from '../lib/session.svelte';

  let {
    session,
    photo = $bindable(),
    label = 'Photo',
  }: {
    session: Session;
    /** The id on the draft. `null` while there is none. */
    photo: string | null;
    label?: string;
  } = $props();

  let camera = $state<HTMLInputElement | null>(null);
  let library = $state<HTMLInputElement | null>(null);
  let busy = $state(false);
  let problem = $state<string | null>(null);

  async function chosen(event: Event): Promise<void> {
    const target = event.currentTarget as HTMLInputElement;
    const file = target.files?.[0];
    // Resetting now rather than after: picking the same file twice in a row
    // fires no `change` at all if the value is still there.
    target.value = '';
    if (file === undefined) return;

    busy = true;
    problem = null;
    try {
      photo = await session.putPhoto(await encodePhoto(file));
    } catch (cause) {
      // Shown next to the button rather than in the app's error banner: it is
      // this field's problem, and everything else on the form is still fine.
      problem = cause instanceof Error ? cause.message : String(cause);
    } finally {
      busy = false;
    }
  }
</script>

<div class="photo-field">
  <span class="label">{label}</span>

  <div class="row">
    {#if photo !== null}
      <Photo {session} {photo} alt="" />
    {/if}

    <div class="buttons">
      <button type="button" data-action="take-photo" disabled={busy} onclick={() => camera?.click()}>
        {#if busy}Traitement…{:else if photo !== null}Reprendre{:else}Prendre une photo{/if}
      </button>
      <button
        type="button"
        data-action="import-photo"
        disabled={busy}
        onclick={() => library?.click()}
      >
        Importer
      </button>
      {#if photo !== null}
        <button
          type="button"
          class="remove"
          data-action="remove-photo"
          disabled={busy}
          onclick={() => {
            photo = null;
            problem = null;
          }}
        >
          Retirer
        </button>
      {/if}
    </div>
  </div>

  {#if problem !== null}<small class="problem">{problem}</small>{/if}

  <input
    bind:this={camera}
    type="file"
    accept="image/*"
    capture="environment"
    data-field="photo"
    hidden
    onchange={(event) => void chosen(event)}
  />

  <!-- The same field without `capture`: the OS offers the photo library, and
       on a desktop it is the only one of the two that means anything. -->
  <input
    bind:this={library}
    type="file"
    accept="image/*"
    data-field="photo-import"
    hidden
    onchange={(event) => void chosen(event)}
  />
</div>

<style>
  .photo-field {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  /* Three buttons beside a thumbnail do not fit across a 390 px phone, so
     they are allowed to take a second row rather than shrink until their
     labels are unreadable. */
  .buttons {
    display: flex;
    flex: 1;
    flex-wrap: wrap;
    gap: var(--space-2);
    min-width: 0;
  }

  button {
    flex: 1;
    min-width: 0;
    padding: var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-weight: var(--weight-medium);
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.5;
  }

  .remove {
    flex: none;
    color: var(--danger);
  }

  .problem {
    color: var(--danger);
    font-weight: var(--weight-normal);
  }
</style>
