<script lang="ts">
  // Farbpalette: Farbe klicken -> aktiviert Farb-Layer (Farbe = Layer). Reiner
  // Inhalt fuer einen Grid-Slot.
  import { rgb } from "./core";
  let {
    swatches,
    onpick,
  }: {
    swatches: [number, number, number][];
    onpick: (c: [number, number, number]) => void;
  } = $props();
</script>

<div class="palette">
  <span class="label">Farbe</span>
  <div class="swatches">
    {#each swatches as c}
      <button
        class="swatch"
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
  }
  .swatch:hover {
    border-color: white;
    transform: scale(1.15);
  }
</style>
