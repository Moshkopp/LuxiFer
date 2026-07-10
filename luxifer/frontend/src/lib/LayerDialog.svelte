<script lang="ts">
  import type { Layer, LayerParams } from "./core";

  let {
    layer,
    onsave,
    oncancel,
  }: {
    layer: Layer;
    onsave: (p: LayerParams) => void;
    oncancel: () => void;
  } = $props();

  // Lokale, editierbare Kopie der Layer-Werte. Bewusst nur der Startwert beim
  // Öffnen — der Dialog schreibt erst beim Speichern zurück.
  /* svelte-ignore state_referenced_locally */
  let name = $state(layer.name);
  /* svelte-ignore state_referenced_locally */
  let mode = $state<"Cut" | "Fill" | "Raster" | "Image">(layer.mode);
  /* svelte-ignore state_referenced_locally */
  let speed = $state(layer.speed_mm_s);
  /* svelte-ignore state_referenced_locally */
  let power = $state(layer.power_pct);
  /* svelte-ignore state_referenced_locally */
  let minPower = $state(layer.min_power_pct);
  /* svelte-ignore state_referenced_locally */
  let passes = $state(layer.passes);
  /* svelte-ignore state_referenced_locally */
  let airAssist = $state(layer.air_assist);
  /* svelte-ignore state_referenced_locally */
  let lineStep = $state(layer.line_step_mm);
  /* svelte-ignore state_referenced_locally */
  let dpi = $state(layer.dpi);
  /* svelte-ignore state_referenced_locally */
  let bidirectional = $state(layer.bidirectional);
  /* svelte-ignore state_referenced_locally */
  let fillAngle = $state(layer.fill_angle_deg ?? 0);
  /* svelte-ignore state_referenced_locally */
  let crossFill = $state(layer.cross_fill ?? false);

  // Bild-Layer (ADR 0004): Modus fest „Bild", eigene Parameter (DPI +
  // Bidirektional statt Linienabstand). Die Bildverarbeitung (Schwelle/Tonwert)
  // liegt im Doppelklick-Bild-Editor, nicht hier.
  const isImage = $derived(mode === "Image");
  const isRaster = $derived(mode === "Raster");
  const isFill = $derived(mode === "Fill" || mode === "Raster");
  // Farbe des Layers als CSS (für den Kopf-Punkt).
  const swatch = $derived(`rgb(${layer.color[0]}, ${layer.color[1]}, ${layer.color[2]})`);

  function save() {
    onsave({
      name,
      mode,
      speed_mm_s: speed,
      power_pct: power,
      min_power_pct: minPower,
      passes,
      air_assist: airAssist,
      line_step_mm: lineStep,
      dpi,
      bidirectional,
      fill_angle_deg: fillAngle,
      cross_fill: crossFill,
    });
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="backdrop"
  onclick={oncancel}
  onkeydown={(e) => e.key === "Escape" && oncancel()}
  role="button"
  tabindex="-1"
>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="dialog glass" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1">
    <div class="head">
      <span class="dot" style="background:{swatch}"></span>
      <h3>Ebene bearbeiten</h3>
      {#if isImage}<span class="badge">Bild</span>{/if}
    </div>

    <label class="field">
      <span>Name</span>
      <input bind:value={name} />
    </label>

    {#if isImage}
      <!-- Bild-Layer: Modus ist fest, nicht umschaltbar. -->
      <div class="field">
        <span>Modus</span>
        <div class="fixed-mode">Bild — Rastergravur</div>
      </div>
    {:else}
      <label class="field">
        <span>Modus</span>
        <select bind:value={mode}>
          <option value="Cut">Schneiden</option>
          <option value="Fill">Füllen</option>
          <option value="Raster">Raster</option>
        </select>
      </label>
    {/if}

    <div class="row">
      <label class="field"><span>Speed mm/s</span><input type="number" bind:value={speed} min="1" /></label>
      <label class="field"><span>Passes</span><input type="number" bind:value={passes} min="1" /></label>
    </div>

    <div class="row">
      <label class="field"><span>Power %</span><input type="number" bind:value={power} min="0" max="100" /></label>
      <label class="field"><span>Min-Power %</span><input type="number" bind:value={minPower} min="0" max="100" /></label>
    </div>

    {#if isImage || isRaster}
      <div class="row">
        <label class="field"><span>DPI</span><input type="number" bind:value={dpi} min="1" /></label>
        <label class="check"><input type="checkbox" bind:checked={bidirectional} /> Bidirektional</label>
      </div>
    {:else if isFill}
      <label class="field"><span>Linienabstand mm</span><input type="number" step="0.01" bind:value={lineStep} min="0.01" /></label>
      <div class="row">
        <label class="field"><span>Füllwinkel °</span><input type="number" step="5" min="-90" max="90" bind:value={fillAngle} /></label>
        <label class="check"><input type="checkbox" bind:checked={crossFill} /> Kreuzschraffur</label>
      </div>
    {/if}

    <label class="check"><input type="checkbox" bind:checked={airAssist} /> Air-Assist</label>

    <div class="actions">
      <button class="gbtn ghost" onclick={oncancel}>Abbrechen</button>
      <button class="gbtn primary" onclick={save}>Speichern</button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }
  /* Frosted-Glas wie der Rest der App (ADR 0002). */
  .dialog {
    width: 415px;
    padding: 20px;
    border-radius: 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .dot {
    width: 12px;
    height: 12px;
    border-radius: 4px;
    border: 1px solid rgba(255, 255, 255, 0.25);
    flex: none;
  }
  h3 {
    margin: 0;
    font-size: 15px;
    flex: 1;
  }
  .badge {
    font-size: 10px;
    letter-spacing: 1px;
    text-transform: uppercase;
    font-weight: 600;
    color: #fff;
    background: var(--accent);
    padding: 3px 9px;
    border-radius: 20px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12px;
    color: var(--muted);
  }
  .row {
    display: flex;
    gap: 10px;
    align-items: flex-end;
  }
  .row .field {
    flex: 1;
  }
  input,
  select {
    background: rgba(0, 0, 0, 0.28);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 8px;
    color: var(--text);
    padding: 8px 10px;
    font-size: 13px;
  }
  input:focus,
  select:focus {
    outline: none;
    border-color: var(--accent);
  }
  /* Fester Modus-Anzeiger für Bild-Layer (kein Dropdown). */
  .fixed-mode {
    background: rgba(0, 0, 0, 0.2);
    border: 1px dashed rgba(255, 255, 255, 0.14);
    border-radius: 8px;
    padding: 8px 10px;
    font-size: 13px;
    color: var(--text);
  }
  .check {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 8px;
    color: var(--text);
    font-size: 13px;
    flex: 1;
    padding-bottom: 8px;
  }
  .check input {
    width: auto;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
  }
  .gbtn {
    padding: 8px 16px;
    font-size: 13px;
    border-radius: 9px;
  }
  .ghost {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text);
    border: 1px solid rgba(255, 255, 255, 0.12);
  }
  .primary {
    background: linear-gradient(
      180deg,
      hsl(var(--accent-h) var(--accent-s) calc(var(--accent-l) + 8%)),
      var(--accent)
    );
    color: #fff;
    border: 1px solid hsl(var(--accent-h) var(--accent-s) 80% / 0.6);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.3),
      0 0 16px -3px hsl(var(--accent-h) var(--accent-s) var(--accent-l) / 0.55);
  }
</style>
