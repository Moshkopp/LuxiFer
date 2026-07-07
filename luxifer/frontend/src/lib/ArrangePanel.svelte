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
  /* Einreihig; die Buttons teilen sich die Panelbreite und passen sich ihr an
     (kein Umbruch, kein Stauchen). So bleibt die Reihe intakt, egal wie schmal
     das Panel gezogen wird. */
  .arrange {
    display: flex;
    align-items: center;
    gap: 3px;
    width: 100%;
    container-type: inline-size;
  }
  button {
    flex: 1 1 0;
    min-width: 0;
    /* Quadratisch: Hoehe folgt der (mit dem Panel schrumpfenden) Breite,
       gedeckelt, damit die Buttons in breiten Panels nicht riesig werden. */
    aspect-ratio: 1;
    max-width: 34px;
    /* Icon-/Glyphgroesse skaliert mit der Buttonbreite. */
    font-size: clamp(10px, 2.6cqw, 16px);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }
  .vsep {
    flex: 0 0 1px;
    align-self: stretch;
    background: var(--border);
    margin: 3px 4px;
  }
</style>
