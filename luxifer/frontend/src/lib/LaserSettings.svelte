<script lang="ts">
  // Laser-Verwaltung als eigenständiges Modal — Schnellzugriff aus dem Laserpanel.
  // Der Inhalt steckt in LaserSettingsContent (auch vom zentralen SettingsModal
  // genutzt); hier nur der Modal-Rahmen.
  import type { LaserRegistry, LaserProfile } from "./core";
  import LaserSettingsContent from "./LaserSettingsContent.svelte";

  let {
    registry,
    onsave,
    ondelete,
    onclose,
  }: {
    registry: LaserRegistry | null;
    onsave: (profile: LaserProfile) => void;
    ondelete: (id: string) => void;
    onclose: () => void;
  } = $props();
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
      <h2>Laser verwalten</h2>
      <button class="x" onclick={onclose} aria-label="Schließen">✕</button>
    </header>
    <LaserSettingsContent {registry} {onsave} {ondelete} />
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
    width: min(720px, 92vw);
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
</style>
