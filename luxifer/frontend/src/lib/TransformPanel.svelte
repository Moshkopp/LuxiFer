<script lang="ts">
  type Anchor = "nw" | "n" | "ne" | "w" | "c" | "e" | "sw" | "s" | "se";
  let { bbox, ontransform }: {
    bbox: [number, number, number, number] | null;
    ontransform: (start: [number, number, number, number], target: [number, number, number, number]) => void;
  } = $props();

  let anchor = $state<Anchor>("c");
  let locked = $state(true);
  let x = $state(0), y = $state(0), w = $state(0), h = $state(0);

  const factors: Record<Anchor, [number, number]> = {
    nw:[0,0], n:[.5,0], ne:[1,0], w:[0,.5], c:[.5,.5], e:[1,.5],
    sw:[0,1], s:[.5,1], se:[1,1],
  };
  function round(v: number) { return Math.round(v * 100) / 100; }
  function sync() {
    if (!bbox) { x = y = w = h = 0; return; }
    const [fx, fy] = factors[anchor];
    x = round(bbox[0] + bbox[2] * fx);
    y = round(bbox[1] + bbox[3] * fy);
    w = round(bbox[2]); h = round(bbox[3]);
  }
  $effect(() => { bbox; anchor; sync(); });

  function applyPosition() {
    if (!bbox) return;
    const [fx, fy] = factors[anchor];
    ontransform(bbox, [x - bbox[2] * fx, y - bbox[3] * fy, bbox[2], bbox[3]]);
  }
  function applyWidth() {
    if (!bbox || w <= 0) return;
    // Bei einer vertikalen Linie ist die Ausgangsbreite null; dann kann kein
    // belastbares Seitenverhältnis abgeleitet werden.
    const nh = locked && bbox[2] > 1e-9 ? w * bbox[3] / bbox[2] : h;
    applySize(w, nh);
  }
  function applyHeight() {
    if (!bbox || h <= 0) return;
    const nw = locked && bbox[3] > 1e-9 ? h * bbox[2] / bbox[3] : w;
    applySize(nw, h);
  }
  function applySize(nw: number, nh: number) {
    if (!bbox || nw <= 0 || nh <= 0) return;
    const [fx, fy] = factors[anchor];
    const ax = bbox[0] + bbox[2] * fx, ay = bbox[1] + bbox[3] * fy;
    ontransform(bbox, [ax - nw * fx, ay - nh * fy, nw, nh]);
  }
</script>

<div class="transform" aria-label="Position und Größe">
  <div class="fields">
    <label>X<input type="number" step="0.5" bind:value={x} disabled={!bbox} onchange={applyPosition} onkeydown={(e) => e.key === "Enter" && applyPosition()} /></label>
    <label>Y<input type="number" step="0.5" bind:value={y} disabled={!bbox} onchange={applyPosition} onkeydown={(e) => e.key === "Enter" && applyPosition()} /></label>
  </div>
  <span class="sep"></span>
  <div class="anchor9" title="Bezugspunkt für Position und Größe">
    {#each (["nw","n","ne","w","c","e","sw","s","se"] as Anchor[]) as a}
      <button class:active={anchor === a} onclick={() => (anchor = a)} title={`Anker ${a}`} aria-label={`Anker ${a}`}></button>
    {/each}
  </div>
  <span class="sep"></span>
  <div class="fields">
    <label>B<input type="number" min="0.1" step="0.5" bind:value={w} disabled={!bbox} onchange={applyWidth} onkeydown={(e) => e.key === "Enter" && applyWidth()} /></label>
    <label>H<input type="number" min="0.1" step="0.5" bind:value={h} disabled={!bbox} onchange={applyHeight} onkeydown={(e) => e.key === "Enter" && applyHeight()} /></label>
  </div>
  <button class="lock gbtn" class:active={locked} onclick={() => (locked = !locked)} title="Seitenverhältnis sperren">{locked ? "🔒" : "🔓"}</button>
</div>

<style>
  .transform { display:flex; align-items:center; gap:6px; flex:0 0 auto; }
  .fields { display:grid; grid-template-rows:repeat(2, 20px); gap:2px; }
  label { display:grid; grid-template-columns:12px 54px; align-items:center; color:var(--muted); font-size:10px; }
  input { width:54px; height:20px; box-sizing:border-box; border:1px solid var(--border); border-radius:5px; background:rgba(0,0,0,.25); color:var(--text); padding:1px 4px; text-align:right; font-size:11px; }
  .sep { width:1px; height:38px; background:var(--border); }
  .anchor9 { display:grid; grid-template-columns:repeat(3, 10px); grid-template-rows:repeat(3, 10px); gap:2px; padding:2px; }
  .anchor9 button { width:10px; height:10px; min-width:0; padding:0; border:1px solid color-mix(in srgb, var(--text) 45%, transparent); border-radius:50%; background:transparent; }
  .anchor9 button.active { background:var(--accent); border-color:var(--accent); box-shadow:0 0 4px var(--accent); }
  .lock { width:28px; height:42px; padding:0; font-size:13px; }
  .lock.active { border-color:var(--accent); }
</style>
