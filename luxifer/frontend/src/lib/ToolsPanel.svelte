<script lang="ts">
  // Werkzeugleiste: Auswahl/Zeichnen + Undo/Redo/Loeschen. Reiner Inhalt fuer
  // einen Grid-Slot; die Anordnung (Zeilen/Spalten) folgt der Slot-Breite.
  type Tool = "select" | "rect" | "ellipse";
  let {
    tool,
    onpick,
    onundo,
    onredo,
    ondelete,
  }: {
    tool: Tool;
    onpick: (t: Tool) => void;
    onundo: () => void;
    onredo: () => void;
    ondelete: () => void;
  } = $props();
</script>

<div class="tools">
  <button class="gbtn" class:active={tool === "select"} onclick={() => onpick("select")} title="Auswählen">▲</button>
  <button class="gbtn" class:active={tool === "rect"} onclick={() => onpick("rect")} title="Rechteck">▭</button>
  <button class="gbtn" class:active={tool === "ellipse"} onclick={() => onpick("ellipse")} title="Ellipse">◯</button>
  <div class="sep"></div>
  <button class="gbtn" onclick={onundo} title="Rückgängig">↶</button>
  <button class="gbtn" onclick={onredo} title="Wiederholen">↷</button>
  <button class="gbtn" onclick={ondelete} title="Löschen">🗑</button>
</div>

<style>
  /* Buttons fliessen und duerfen schrumpfen, damit sich das Panel schmal
     ziehen laesst und die Toolbar 1- oder 2-spaltig wird (ADR 0002 §1). */
  .tools {
    display: flex;
    flex-wrap: wrap;
    align-content: flex-start;
    gap: 4px;
  }
  button {
    flex: 1 1 34px;
    min-width: 30px;
    max-width: 46px;
    aspect-ratio: 1;
    font-size: 18px;
  }
  .sep {
    flex-basis: 100%;
    height: 1px;
    background: var(--border);
    margin: 2px 0;
  }
</style>
