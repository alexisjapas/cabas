<script lang="ts">
  /**
   * A QR code, as an SVG.
   *
   * Horizontal runs are merged into one rect each, which turns roughly seven
   * hundred nodes into two hundred and costs nothing to read. `crispEdges`
   * matters more than it looks: antialiasing between two touching rects leaves
   * a grey seam down the middle of a bar, and a scanner reads grey as neither.
   */
  import { SIZE, encode } from '../lib/qr';

  let { text, label }: { text: string; label: string } = $props();

  /** Four modules of nothing on every side. A scanner needs it to find where
   *  the symbol stops, and it is padding rather than data, so it lives here
   *  and not in the encoder. */
  const QUIET = 4;
  const SPAN = SIZE + QUIET * 2;

  let runs = $derived.by(() => {
    const modules = encode(text);
    const bars: { x: number; y: number; width: number }[] = [];
    for (let y = 0; y < SIZE; y += 1) {
      const row = modules[y] ?? [];
      let x = 0;
      while (x < SIZE) {
        if (row[x] !== true) {
          x += 1;
          continue;
        }
        let width = 1;
        while (x + width < SIZE && row[x + width] === true) width += 1;
        bars.push({ x, y, width });
        x += width;
      }
    }
    return bars;
  });
</script>

<svg class="qr" viewBox="{-QUIET} {-QUIET} {SPAN} {SPAN}" role="img" aria-label={label}>
  <rect class="field" x={-QUIET} y={-QUIET} width={SPAN} height={SPAN} />
  {#each runs as run (`${run.y}:${run.x}`)}
    <rect class="module" x={run.x} y={run.y} width={run.width} height="1" />
  {/each}
</svg>

<style>
  .qr {
    display: block;
    width: 100%;
    max-width: 15rem;
    height: auto;
    margin: 0 auto;
    border-radius: var(--radius-md);
    shape-rendering: crispedges;
  }

  .field {
    fill: var(--surface-raised);
  }

  .module {
    fill: var(--text);
  }
</style>
