<script lang="ts">
  // Anordnen-Toolbar: Ausrichten/Verteilen + Nesting. Die Geometrie-Werkzeuge
  // (Boolean/Fillet/Offset/Muster) liegen wie in der Referenz in der
  // WERKZEUGLEISTE (Gruppe 3), nicht hier.
  import type { AlignKind, DistributeKind } from "./core";
  import Icon from "./Icon.svelte";
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
  <div class="group">
    <button class="gbtn" disabled={selCount < 1} onclick={() => onalign("left")} title="Links ausrichten" aria-label="Links ausrichten"><Icon name="align-left" /></button>
    <button class="gbtn" disabled={selCount < 1} onclick={() => onalign("hcenter")} title="Horizontal zentrieren" aria-label="Horizontal zentrieren"><Icon name="align-hcenter" /></button>
    <button class="gbtn" disabled={selCount < 1} onclick={() => onalign("right")} title="Rechts ausrichten" aria-label="Rechts ausrichten"><Icon name="align-right" /></button>
    <span class="mini-sep"></span>
    <button class="gbtn" disabled={selCount < 1} onclick={() => onalign("top")} title="Oben ausrichten" aria-label="Oben ausrichten"><Icon name="align-top" /></button>
    <button class="gbtn" disabled={selCount < 1} onclick={() => onalign("vcenter")} title="Vertikal zentrieren" aria-label="Vertikal zentrieren"><Icon name="align-vcenter" /></button>
    <button class="gbtn" disabled={selCount < 1} onclick={() => onalign("bottom")} title="Unten ausrichten" aria-label="Unten ausrichten"><Icon name="align-bottom" /></button>
    <button class="gbtn" disabled={selCount < 1} onclick={() => onalign("center")} title="Auf beiden Achsen zentrieren" aria-label="Zentrieren"><Icon name="align-center" /></button>
  </div>

  <span class="vsep"></span>
  <div class="group">
    <button class="gbtn" disabled={selCount < 3} onclick={() => ondistribute("h")} title="Mitten horizontal gleichmäßig verteilen" aria-label="Horizontal verteilen"><Icon name="dist-h" /></button>
    <button class="gbtn" disabled={selCount < 3} onclick={() => ondistribute("space-h")} title="Horizontale Zwischenräume angleichen" aria-label="Horizontale Abstände"><Icon name="space-h" /></button>
    <button class="gbtn" disabled={selCount < 3} onclick={() => ondistribute("v")} title="Mitten vertikal gleichmäßig verteilen" aria-label="Vertikal verteilen"><Icon name="dist-v" /></button>
    <button class="gbtn" disabled={selCount < 3} onclick={() => ondistribute("space-v")} title="Vertikale Zwischenräume angleichen" aria-label="Vertikale Abstände"><Icon name="space-v" /></button>
  </div>

  <span class="vsep"></span>
  <div class="group">
    <button class="gbtn" disabled={selCount < 2} onclick={ongroup} title="Gruppieren (Strg+G)" aria-label="Gruppieren"><Icon name="group" /></button>
    <button class="gbtn" disabled={selCount < 1} onclick={onungroup} title="Gruppierung lösen (Strg+Umschalt+G)" aria-label="Gruppierung lösen"><Icon name="ungroup" /></button>
  </div>

  <span class="vsep"></span>
  <div class="nest-wrap">
    <button class="gbtn wide" disabled={selCount < 1} onclick={() => (nestOpen = !nestOpen)} title="Nesting-Optionen">
      <Icon name="nest" /> Nesting
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
    gap: var(--sp-2);
    width: 100%;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
    min-width: 0;
  }
  /* Buttons einer Kategorie eng zusammen; Kategorien durch vsep getrennt. */
  .group {
    display: flex;
    align-items: center;
    gap: var(--sp-1);
    flex: 0 0 auto;
    min-width: max-content;
  }
  button {
    flex: 0 0 30px;
    width: 30px;
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    color: var(--muted);
  }
  button:not(:disabled):hover { color: var(--text); }
  button.wide {
    flex-basis: auto;
    width: auto;
    gap: var(--sp-2);
    padding: 0 var(--sp-3);
    font-size: var(--fs-sm);
    color: var(--text);
  }
  .mm {
    min-width: 34px;
    width: 52px;
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    color: var(--text);
    font-size: var(--fs-sm);
    padding: 4px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  button.primary {
    width: 100%;
    flex: none;
    background: var(--accent);
    color: white;
    height: 30px;
  }
  /* Kräftiger Trenner ZWISCHEN Kategorien */
  .vsep {
    flex: 0 0 1px;
    height: 22px;
    background: var(--border);
    margin: 0 var(--sp-1);
  }
  /* Feiner Trenner INNERHALB einer Kategorie (H- vs V-Ausrichtung) */
  .mini-sep {
    width: 1px;
    height: 16px;
    background: var(--border-soft);
    margin: 0 2px;
  }
  .nest-wrap {
    position: relative;
    flex: 0 0 auto;
  }
  .toolbar::-webkit-scrollbar { display: none; }
  .nest-menu {
    position: absolute;
    top: calc(100% + var(--sp-2));
    right: 0;
    width: 190px;
    padding: var(--sp-3);
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    z-index: 80;
  }
  .nest-menu label {
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
    color: var(--muted);
    font-size: var(--fs-xs);
  }
  select {
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    color: var(--text);
    padding: 5px 7px;
  }
</style>
