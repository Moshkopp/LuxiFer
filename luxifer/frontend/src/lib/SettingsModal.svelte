<script lang="ts">
  // Zentrales Einstellungs-Modal (Zahnrad oben rechts). Sektions-Navigation;
  // die Laser-Verwaltung (ADR 0007) ist die erste Sektion, weitere folgen.
  import type { LaserRegistry, LaserProfile } from "./core";
  import LaserSettingsContent from "./LaserSettingsContent.svelte";

  let {
    registry,
    onsave,
    ondelete,
    oneditlayout,
    onresettab,
    onclose,
  }: {
    registry: LaserRegistry | null;
    onsave: (profile: LaserProfile) => void;
    ondelete: (id: string) => void;
    /** „Oberfläche bearbeiten" starten (schließt das Modal, aktiviert Edit-Modus). */
    oneditlayout: () => void;
    /** Aktuellen Reiter auf Standard-Layout zurücksetzen. */
    onresettab: () => void;
    onclose: () => void;
  } = $props();

  type Section = "laser" | "oberflaeche";
  let section = $state<Section>("laser");
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />
<div class="overlay" onclick={onclose} role="presentation">
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    class="modal"
    onclick={(e) => e.stopPropagation()}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
  >
    <header>
      <h2>Einstellungen</h2>
      <button class="x" onclick={onclose} aria-label="Schließen">✕</button>
    </header>

    <div class="split">
      <!-- Sektions-Navigation -->
      <nav class="sections">
        <button class="sec" class:on={section === "laser"} onclick={() => (section = "laser")}>
          Laser
        </button>
        <button
          class="sec"
          class:on={section === "oberflaeche"}
          onclick={() => (section = "oberflaeche")}
        >
          Oberfläche
        </button>
      </nav>

      <!-- Sektions-Inhalt -->
      <div class="content">
        {#if section === "laser"}
          <LaserSettingsContent {registry} {onsave} {ondelete} />
        {:else}
          <div class="ui">
            <p class="hint">
              Panele lassen sich frei anordnen und ein-/ausblenden. Starte den
              Bearbeiten-Modus, um sie zu verschieben und in der Größe zu ändern.
            </p>
            <div class="actions">
              <button class="prim" onclick={oneditlayout}>Oberfläche bearbeiten</button>
              <button class="sec-btn" onclick={onresettab}>Reiter zurücksetzen</button>
            </div>
          </div>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(2px);
    display: grid;
    place-items: center;
    z-index: 1000;
  }
  .modal {
    width: min(760px, 92vw);
    max-height: 86vh;
    background: var(--panel, #1c1f26);
    border: 1px solid var(--border, rgba(255, 255, 255, 0.12));
    border-radius: 14px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 18px;
    border-bottom: 1px solid var(--border);
  }
  h2 {
    margin: 0;
    font-size: 15px;
  }
  .x {
    background: transparent;
    border: none;
    color: var(--muted);
    cursor: pointer;
    font-size: 15px;
  }
  .split {
    display: grid;
    grid-template-columns: 150px 1fr;
    min-height: 0;
    flex: 1;
  }
  .sections {
    border-right: 1px solid var(--border);
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .sec {
    text-align: left;
    padding: 8px 12px;
    border-radius: 8px;
    background: transparent;
    border: 1px solid transparent;
    color: var(--text);
    cursor: pointer;
    font-size: 13px;
  }
  .sec:hover {
    background: rgba(255, 255, 255, 0.05);
  }
  .sec.on {
    background: rgba(255, 255, 255, 0.08);
    border-color: var(--accent);
  }
  .content {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .ui {
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .hint {
    font-size: 13px;
    color: var(--muted);
    margin: 0;
    line-height: 1.5;
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  .prim {
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 8px;
    padding: 8px 16px;
    cursor: pointer;
    font-weight: 600;
  }
  .sec-btn {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: 8px;
    padding: 8px 14px;
    cursor: pointer;
  }
</style>
