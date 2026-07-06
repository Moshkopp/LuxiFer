<script lang="ts">
  // Anordnen-Toolbar: Ausrichten/Verteilen. Knoepfe je nach Auswahl aktiv.
  import type { AlignKind, DistributeKind } from "./core";
  let {
    selCount,
    onalign,
    ondistribute,
  }: {
    selCount: number;
    onalign: (k: AlignKind) => void;
    ondistribute: (k: DistributeKind) => void;
  } = $props();
</script>

<div class="arrange">
  <button class="gbtn" disabled={selCount < 2} onclick={() => onalign("left")} title="Links ausrichten">⇤</button>
  <button class="gbtn" disabled={selCount < 2} onclick={() => onalign("hcenter")} title="Horizontal zentrieren">⇔</button>
  <button class="gbtn" disabled={selCount < 2} onclick={() => onalign("right")} title="Rechts ausrichten">⇥</button>
  <div class="vsep"></div>
  <button class="gbtn" disabled={selCount < 2} onclick={() => onalign("top")} title="Oben ausrichten">⤒</button>
  <button class="gbtn" disabled={selCount < 2} onclick={() => onalign("vcenter")} title="Vertikal zentrieren">⇕</button>
  <button class="gbtn" disabled={selCount < 2} onclick={() => onalign("bottom")} title="Unten ausrichten">⤓</button>
  <div class="vsep"></div>
  <button class="gbtn" disabled={selCount < 3} onclick={() => ondistribute("h")} title="Horizontal verteilen">⋯</button>
  <button class="gbtn" disabled={selCount < 3} onclick={() => ondistribute("v")} title="Vertikal verteilen">⋮</button>
</div>

<style>
  .arrange {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 3px;
  }
  button {
    flex: 0 0 auto;
    width: 34px;
    height: 34px;
    font-size: 16px;
  }
  .vsep {
    width: 1px;
    align-self: stretch;
    background: var(--border);
    margin: 3px 4px;
  }
</style>
