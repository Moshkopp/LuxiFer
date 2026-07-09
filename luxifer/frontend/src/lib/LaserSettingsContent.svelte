<script lang="ts">
  // Reiner Inhalt der Laser-Verwaltung (ADR 0007) — ohne eigenes Overlay/Modal,
  // damit ihn sowohl das Laserpanel-Modal (LaserSettings) als auch das zentrale
  // Settings-Modal (SettingsModal) einbetten können. Auswahl des aktiven Lasers
  // passiert im Laserpanel-Dropdown, nicht hier.
  import type {
    LaserRegistry,
    LaserProfile,
    DriverKind,
  } from "./core";

  let {
    registry,
    onsave,
    ondelete,
  }: {
    registry: LaserRegistry | null;
    onsave: (profile: LaserProfile) => void;
    ondelete: (id: string) => void;
  } = $props();

  const profiles = $derived(registry?.profiles ?? []);

  // Aktuell editiertes Profil (Kopie; leer = keins gewählt).
  let edit = $state<LaserProfile | null>(null);

  function fresh(): LaserProfile {
    return {
      id: "",
      name: "Neuer Laser",
      kind: "Ruida",
      connection: { art: "netz", ip: "192.168.1.100", port: null },
      bed_mm: [600, 400],
      scan_offset: { enabled: false, points: [] },
    };
  }

  function pick(p: LaserProfile) {
    edit = structuredClone($state.snapshot(p)) as LaserProfile;
  }
  function neu() {
    edit = fresh();
  }
  function save() {
    if (edit) onsave(edit);
    edit = null;
  }
  function del() {
    if (edit?.id) ondelete(edit.id);
    edit = null;
  }

  // Verbindungsart umschalten (Netz ↔ Seriell) ohne die andere zu verlieren.
  function setKind(k: DriverKind) {
    if (!edit) return;
    edit.kind = k;
    // Ruida = Netz, GRBL/MiniGRBL = Seriell (Vorschlag, editierbar).
    if (k === "Ruida" && edit.connection.art !== "netz") {
      edit.connection = { art: "netz", ip: "192.168.1.100", port: null };
    } else if (k !== "Ruida" && edit.connection.art !== "seriell") {
      edit.connection = { art: "seriell", port: "/dev/ttyUSB0", baud: 115200 };
    }
  }

  function addPoint() {
    if (!edit) return;
    edit.scan_offset.points = [
      ...edit.scan_offset.points,
      { speed_mm_s: 100, offset_mm: 0.1 },
    ];
  }
  function removePoint(i: number) {
    if (!edit) return;
    edit.scan_offset.points = edit.scan_offset.points.filter((_, k) => k !== i);
  }
</script>

<div class="body">
  <!-- Liste -->
  <aside class="list">
    {#each profiles as p (p.id)}
      <button class="item" class:on={edit?.id === p.id} onclick={() => pick(p)}>
        <span class="nm">{p.name}</span>
        <span class="kd">{p.kind}</span>
      </button>
    {/each}
    <button class="item add" onclick={neu}>＋ Neuer Laser</button>
  </aside>

  <!-- Editor -->
  <div class="form">
    {#if edit}
      <label class="f">Name<input bind:value={edit.name} /></label>

      <label class="f">
        Treiber
        <select value={edit.kind} onchange={(e) => setKind((e.currentTarget as HTMLSelectElement).value as DriverKind)}>
          <option value="Ruida">Ruida</option>
          <option value="Grbl">GRBL</option>
          <option value="MiniGrbl">miniGRBL</option>
        </select>
      </label>

      {#if edit.connection.art === "netz"}
        <label class="f">IP-Adresse<input bind:value={edit.connection.ip} /></label>
      {:else}
        <label class="f">Serieller Port<input bind:value={edit.connection.port} /></label>
        <label class="f">Baudrate<input type="number" bind:value={edit.connection.baud} /></label>
      {/if}

      <div class="row">
        <label class="f">Bett B (mm)<input type="number" bind:value={edit.bed_mm[0]} /></label>
        <label class="f">Bett H (mm)<input type="number" bind:value={edit.bed_mm[1]} /></label>
      </div>

      <!-- Scan-Offset-Kalibrierung -->
      <div class="cal">
        <label class="chk">
          <input type="checkbox" bind:checked={edit.scan_offset.enabled} />
          Scan-Offset (Reversal-Korrektur) aktiv
        </label>
        {#if edit.scan_offset.enabled}
          <p class="hint">
            Tabelle Geschwindigkeit → Versatz. Zwischen den Punkten wird
            interpoliert. Korrigiert das Ausfransen beim bidirektionalen Rastern.
          </p>
          {#each edit.scan_offset.points as pt, i (i)}
            <div class="prow">
              <input type="number" bind:value={pt.speed_mm_s} /> <span>mm/s →</span>
              <input type="number" step="0.01" bind:value={pt.offset_mm} /> <span>mm</span>
              <button class="rm" onclick={() => removePoint(i)} aria-label="Punkt entfernen">✕</button>
            </div>
          {/each}
          <button class="addpt" onclick={addPoint}>＋ Stützpunkt</button>
        {/if}
      </div>

      <div class="actions">
        {#if edit.id}
          <button class="del" onclick={del}>Löschen</button>
        {/if}
        <button class="save" onclick={save}>Speichern</button>
      </div>
    {:else}
      <p class="empty">Wähle links einen Laser oder lege einen neuen an.</p>
    {/if}
  </div>
</div>

<style>
  .body {
    display: grid;
    grid-template-columns: 200px 1fr;
    min-height: 0;
    flex: 1;
  }
  .list {
    border-right: 1px solid var(--border);
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    overflow-y: auto;
  }
  .item {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    text-align: left;
    gap: 1px;
    padding: 8px 10px;
    border-radius: 8px;
    background: transparent;
    border: 1px solid transparent;
    color: var(--text);
    cursor: pointer;
  }
  .item:hover {
    background: rgba(255, 255, 255, 0.05);
  }
  .item.on {
    background: rgba(255, 255, 255, 0.08);
    border-color: var(--accent);
  }
  .item .nm {
    font-size: 13px;
  }
  .item .kd {
    font-size: 10px;
    color: var(--muted);
  }
  .item.add {
    color: var(--muted);
    margin-top: 4px;
  }
  .form {
    padding: 16px 18px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .f {
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
  .row .f {
    flex: 1;
  }
  input,
  select {
    background: rgba(0, 0, 0, 0.22);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    color: var(--text);
    padding: 6px 8px;
    font-size: 13px;
  }
  .cal {
    border-top: 1px solid var(--border);
    padding-top: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .chk {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }
  .hint {
    font-size: 11px;
    color: var(--muted);
    margin: 0;
  }
  .prow {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--muted);
  }
  .prow input {
    width: 70px;
  }
  .rm,
  .addpt {
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--muted);
    cursor: pointer;
    padding: 4px 8px;
    font-size: 12px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 6px;
  }
  .save {
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 8px;
    padding: 8px 16px;
    cursor: pointer;
    font-weight: 600;
  }
  .del {
    background: transparent;
    border: 1px solid #b5463c;
    color: #e07a70;
    border-radius: 8px;
    padding: 8px 14px;
    cursor: pointer;
  }
  .empty {
    color: var(--muted);
    font-size: 13px;
  }
</style>
