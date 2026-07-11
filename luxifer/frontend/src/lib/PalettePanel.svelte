<script lang="ts">
  // Farbpalette: Farbe klicken -> aktiviert Farb-Layer (Farbe = Layer). Reiner
  // Inhalt fuer einen Grid-Slot.
  import { rgb } from "./core";
  let {
    swatches,
    active,
    onpick,
  }: {
    swatches: [number, number, number][];
    /** Aktive Zeichenfarbe — markiert den passenden Swatch. */
    active: [number, number, number] | null;
    onpick: (c: [number, number, number]) => void;
  } = $props();

  // Ist dieser Swatch die aktive Farbe? (Komponentenweiser RGB-Vergleich.)
  const isActive = (c: [number, number, number]) =>
    active != null && c[0] === active[0] && c[1] === active[1] && c[2] === active[2];
</script>

<div class="palette">
  <span class="label">Farbe</span>
  <div class="swatches">
    {#each swatches as c}
      <button
        class="swatch"
        class:on={isActive(c)}
        style="background: {rgb(c)}"
        title={rgb(c)}
        onclick={() => onpick(c)}
        aria-label={rgb(c)}
      ></button>
    {/each}
  </div>
</div>

<style>
  .palette {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .label {
    font-size: 11px;
    letter-spacing: 1px;
    color: var(--muted);
    text-transform: uppercase;
  }
  .swatches {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .swatch {
    width: 22px;
    height: 22px;
    border-radius: 11px;
    border: 2px solid transparent;
    padding: 0;
    cursor: pointer;
    transition: transform 0.1s ease, box-shadow 0.1s ease;
  }
  .swatch:hover {
    border-color: white;
    transform: scale(1.15);
  }
  /* Aktive Farbe: heller Ring mit dunklem Absatz — hebt sich auf jeder
     Swatch-Farbe ab (hell wie dunkel), größer als die anderen. */
  .swatch.on {
    transform: scale(1.15);
    box-shadow:
      0 0 0 2px var(--panel),
      0 0 0 4px var(--text);
  }
  .swatch.on:hover {
    border-color: transparent;
  }
</style>
