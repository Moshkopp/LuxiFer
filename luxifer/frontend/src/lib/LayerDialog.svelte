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
  // Öffnen — der Dialog soll nicht live mitziehen, sondern erst beim Speichern
  // zurückschreiben. Daher hier die state_referenced_locally-Warnung ignorieren.
  /* svelte-ignore state_referenced_locally */
  let name = $state(layer.name);
  /* svelte-ignore state_referenced_locally */
  let mode = $state<"Cut" | "Fill" | "Raster">(layer.mode);
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

  const isRaster = $derived(mode === "Raster");
  const isFill = $derived(mode === "Fill" || mode === "Raster");

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
    });
  }
</script>

<div
  class="backdrop"
  onclick={oncancel}
  onkeydown={(e) => e.key === "Escape" && oncancel()}
  role="button"
  tabindex="-1"
>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="dialog" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1">
    <h3>Ebene bearbeiten</h3>

    <label>Name<input bind:value={name} /></label>

    <label>
      Modus
      <select bind:value={mode}>
        <option value="Cut">Schneiden</option>
        <option value="Fill">Füllen</option>
        <option value="Raster">Raster</option>
      </select>
    </label>

    <div class="row">
      <label>Speed mm/s<input type="number" bind:value={speed} min="1" /></label>
      <label>Passes<input type="number" bind:value={passes} min="1" /></label>
    </div>

    <div class="row">
      <label>Power %<input type="number" bind:value={power} min="0" max="100" /></label>
      <label>Min-Power %<input type="number" bind:value={minPower} min="0" max="100" /></label>
    </div>

    {#if isFill}
      <label>Linienabstand mm<input type="number" step="0.01" bind:value={lineStep} min="0.01" /></label>
    {/if}
    {#if isRaster}
      <label>DPI<input type="number" bind:value={dpi} min="1" /></label>
    {/if}

    <label class="check"><input type="checkbox" bind:checked={airAssist} /> Air-Assist</label>

    <div class="actions">
      <button class="ghost" onclick={oncancel}>Abbrechen</button>
      <button class="primary" onclick={save}>Speichern</button>
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
  .dialog {
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 14px;
    padding: 20px;
    width: 340px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    box-shadow: 0 20px 60px -8px rgba(0, 0, 0, 0.6);
  }
  h3 {
    margin: 0 0 4px;
    font-size: 15px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: var(--muted);
  }
  .row {
    display: flex;
    gap: 10px;
  }
  .row label {
    flex: 1;
  }
  input,
  select {
    background: #16171b;
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text);
    padding: 7px 9px;
    font-size: 13px;
  }
  input:focus,
  select:focus {
    outline: none;
    border-color: var(--accent);
  }
  .check {
    flex-direction: row;
    align-items: center;
    gap: 8px;
    color: var(--text);
    font-size: 13px;
  }
  .check input {
    width: auto;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 6px;
  }
  button {
    border: none;
    border-radius: 8px;
    padding: 8px 16px;
    cursor: pointer;
    font-size: 13px;
  }
  .ghost {
    background: #26282d;
    color: var(--text);
  }
  .primary {
    background: var(--accent);
    color: white;
  }
</style>
