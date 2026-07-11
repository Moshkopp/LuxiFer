<script lang="ts">
  // Anordnen-Toolbar: Ausrichten/Verteilen + Nesting. Die Geometrie-Werkzeuge
  // (Boolean/Fillet/Offset/Muster) liegen wie in der Referenz in der
  // WERKZEUGLEISTE (Gruppe 3), nicht hier.
  import type { AlignKind, DistributeKind } from "./core";
  let {
    selCount,
    onalign,
    ondistribute,
    onnest,
    onnestfill,
    ongroup,
    onungroup,
  }: {
    selCount: number;
    onalign: (k: AlignKind) => void;
    ondistribute: (k: DistributeKind) => void;
    onnest: (gap: number) => void;
    onnestfill: (gap: number) => void;
    ongroup: () => void;
    onungroup: () => void;
  } = $props();

  // Nest-Abstand (mm).
  let nestGapMm = $state(2.0);
  let nestOpen = $state(false);
  let nestMode = $state<"pack" | "fill">("pack");

  function canApplyNest(): boolean {
    return nestMode === "fill" ? selCount >= 1 : selCount >= 2;
  }
  function applyNest() {
    if (!canApplyNest()) return;
    if (nestMode === "fill") onnestfill(nestGapMm);
    else onnest(nestGapMm);
    nestOpen = false;
  }
</script>

<div class="toolbar">
  <div class="section">
    <div class="group">
      <button class="gbtn" disabled={selCount < 1} onclick={() => onalign("left")} title="Links ausrichten">⇤</button>
      <button class="gbtn" disabled={selCount < 1} onclick={() => onalign("hcenter")} title="Horizontal zentrieren">⇔</button>
      <button class="gbtn" disabled={selCount < 1} onclick={() => onalign("right")} title="Rechts ausrichten">⇥</button>
      <span class="mini-sep"></span>
      <button class="gbtn" disabled={selCount < 1} onclick={() => onalign("top")} title="Oben ausrichten">⤒</button>
      <button class="gbtn" disabled={selCount < 1} onclick={() => onalign("vcenter")} title="Vertikal zentrieren">⇕</button>
      <button class="gbtn" disabled={selCount < 1} onclick={() => onalign("bottom")} title="Unten ausrichten">⤓</button>
      <button class="gbtn" disabled={selCount < 1} onclick={() => onalign("center")} title="Auf beiden Achsen zentrieren">◎</button>
    </div>
  </div>

  <div class="vsep"></div>
  <div class="section">
    <div class="group">
      <button class="gbtn" disabled={selCount < 3} onclick={() => ondistribute("h")} title="Mitten horizontal gleichmäßig verteilen">⋯</button>
      <button class="gbtn" disabled={selCount < 3} onclick={() => ondistribute("space-h")} title="Horizontale Zwischenräume angleichen">↔</button>
      <button class="gbtn" disabled={selCount < 3} onclick={() => ondistribute("v")} title="Mitten vertikal gleichmäßig verteilen">⋮</button>
      <button class="gbtn" disabled={selCount < 3} onclick={() => ondistribute("space-v")} title="Vertikale Zwischenräume angleichen">↕</button>
    </div>
  </div>

  <div class="vsep"></div>
  <div class="section">
    <div class="group">
      <button class="gbtn" disabled={selCount < 2} onclick={ongroup} title="Gruppieren (Strg+G)">⧉</button>
      <button class="gbtn" disabled={selCount < 1} onclick={onungroup} title="Gruppierung lösen (Strg+Umschalt+G)">⧎</button>
    </div>
  </div>

  <div class="vsep"></div>
  <div class="nest-wrap">
    <button class="gbtn wide" disabled={selCount < 1} onclick={() => (nestOpen = !nestOpen)} title="Nesting-Optionen">
      Nesting
    </button>
    {#if nestOpen}
      <div class="nest-menu glass">
        <label>
          Modus
          <select bind:value={nestMode}>
            <option value="pack">Auswahl packen</option>
            <option value="fill">Bett füllen</option>
          </select>
        </label>
        <label>
          Abstand
          <input class="mm" type="number" step="0.5" min="0" bind:value={nestGapMm} />
        </label>
        <button class="gbtn primary" disabled={!canApplyNest()} onclick={applyNest}>Fertig</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
    padding-bottom: 1px;
    min-width: 0;
  }
  .group {
    display: flex;
    align-items: center;
    gap: 5px;
    flex: 0 0 auto;
    min-width: max-content;
  }
  .section {
    display: flex;
    align-items: center;
    gap: 5px;
    flex: 0 0 auto;
    min-width: max-content;
  }
  button {
    flex: 0 0 30px;
    aspect-ratio: 1;
    font-size: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }
  button.wide {
    aspect-ratio: auto;
    flex-basis: auto;
    height: 30px;
    padding: 0 12px;
    font-size: 12px;
  }
  .mm {
    min-width: 34px;
    width: 52px;
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text);
    font-size: 12px;
    padding: 4px 4px;
    text-align: right;
  }
  button.primary {
    width: 100%;
    flex: none;
    aspect-ratio: auto;
    background: var(--accent);
    color: white;
    height: 30px;
  }
  .vsep {
    flex: 0 0 1px;
    height: 24px;
    background: var(--border);
    margin: 3px 3px;
  }
  .mini-sep {
    width: 1px;
    height: 18px;
    background: var(--border);
    margin: 0 1px;
  }
  .nest-wrap {
    position: relative;
    flex: 0 0 auto;
  }
  .toolbar::-webkit-scrollbar { display: none; }
  .nest-menu {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    width: 190px;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 80;
  }
  .nest-menu label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    color: var(--muted);
    font-size: 11px;
  }
  select {
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text);
    padding: 5px 7px;
  }
</style>
