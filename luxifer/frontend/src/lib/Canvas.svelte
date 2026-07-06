<script lang="ts">
  import { rgb, type Scene, type Shape } from "./core";

  // Props (Svelte 5 Runes).
  let {
    scene,
    onpick,
  }: {
    scene: Scene;
    onpick: (xmm: number, ymm: number) => void;
  } = $props();

  let canvasEl: HTMLCanvasElement;
  let wrapEl: HTMLDivElement;

  // Ansicht: px pro mm + Pan-Offset (px). Einfacher Startwert.
  let zoom = $state(1.2);
  let panX = $state(40);
  let panY = $state(40);

  const toScreen = (xmm: number, ymm: number): [number, number] => [
    xmm * zoom + panX,
    ymm * zoom + panY,
  ];
  const toMm = (px: number, py: number): [number, number] => [
    (px - panX) / zoom,
    (py - panY) / zoom,
  ];

  function draw() {
    if (!canvasEl) return;
    const ctx = canvasEl.getContext("2d");
    if (!ctx) return;
    const w = canvasEl.width;
    const h = canvasEl.height;

    ctx.clearRect(0, 0, w, h);
    ctx.fillStyle = "#141518";
    ctx.fillRect(0, 0, w, h);

    drawGrid(ctx, w, h);
    drawBed(ctx);
    for (const s of scene.shapes) drawShape(ctx, s);
    drawSelection(ctx);
  }

  function drawGrid(ctx: CanvasRenderingContext2D, w: number, h: number) {
    let step = 50;
    while (step * zoom < 8) step *= 2;
    const [tlx, tly] = toMm(0, 0);
    const [brx, bry] = toMm(w, h);
    ctx.lineWidth = 1;
    ctx.strokeStyle = "rgba(255,255,255,0.06)";
    ctx.beginPath();
    for (let x = Math.floor(tlx / step) * step; x <= brx; x += step) {
      const sx = toScreen(x, 0)[0];
      ctx.moveTo(sx, 0);
      ctx.lineTo(sx, h);
    }
    for (let y = Math.floor(tly / step) * step; y <= bry; y += step) {
      const sy = toScreen(0, y)[1];
      ctx.moveTo(0, sy);
      ctx.lineTo(w, sy);
    }
    ctx.stroke();
  }

  function drawBed(ctx: CanvasRenderingContext2D) {
    const [x0, y0] = toScreen(0, 0);
    const bw = scene.bed_w_mm * zoom;
    const bh = scene.bed_h_mm * zoom;
    ctx.fillStyle = "rgba(90,150,220,0.10)";
    ctx.fillRect(x0, y0, bw, bh);
    ctx.strokeStyle = "rgba(90,150,220,0.9)";
    ctx.lineWidth = 1.5;
    ctx.strokeRect(x0, y0, bw, bh);
    // Nullpunkt-Markierung
    ctx.strokeStyle = "rgb(240,180,60)";
    ctx.lineWidth = 2.5;
    ctx.beginPath();
    ctx.moveTo(x0, y0);
    ctx.lineTo(x0 + 18, y0);
    ctx.moveTo(x0, y0);
    ctx.lineTo(x0, y0 + 18);
    ctx.stroke();
  }

  function layerColor(s: Shape): string {
    const l = scene.layers[s.layer_id];
    return l ? rgb(l.color) : "#ff5c62";
  }
  function layerFilled(s: Shape): boolean {
    const l = scene.layers[s.layer_id];
    return !!l && (l.mode === "Fill" || l.mode === "Raster");
  }

  function drawShape(ctx: CanvasRenderingContext2D, s: Shape) {
    const color = layerColor(s);
    ctx.save();
    // Rotation um den Bounding-Box-Mittelpunkt.
    if (s.rotation) {
      const c = shapeCenter(s);
      const [scx, scy] = toScreen(c[0], c[1]);
      ctx.translate(scx, scy);
      ctx.rotate((s.rotation * Math.PI) / 180);
      ctx.translate(-scx, -scy);
    }
    ctx.strokeStyle = color;
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    if ("Rect" in s.geo) {
      const { x, y, w, h } = s.geo.Rect;
      const [sx, sy] = toScreen(x, y);
      ctx.rect(sx, sy, w * zoom, h * zoom);
    } else if ("Ellipse" in s.geo) {
      const { cx, cy, rx, ry } = s.geo.Ellipse;
      const [scx, scy] = toScreen(cx, cy);
      ctx.ellipse(scx, scy, rx * zoom, ry * zoom, 0, 0, Math.PI * 2);
    } else if ("Polyline" in s.geo) {
      const { pts, closed } = s.geo.Polyline;
      pts.forEach((p, i) => {
        const [sx, sy] = toScreen(p[0], p[1]);
        if (i === 0) ctx.moveTo(sx, sy);
        else ctx.lineTo(sx, sy);
      });
      if (closed) ctx.closePath();
    }
    if (layerFilled(s)) {
      ctx.fillStyle = color + "48"; // ~28% Alpha (Hex)
      ctx.fill();
    }
    ctx.stroke();
    ctx.restore();
  }

  function shapeCenter(s: Shape): [number, number] {
    if ("Rect" in s.geo) {
      const { x, y, w, h } = s.geo.Rect;
      return [x + w / 2, y + h / 2];
    }
    if ("Ellipse" in s.geo) {
      const { cx, cy } = s.geo.Ellipse;
      return [cx, cy];
    }
    const { pts } = s.geo.Polyline;
    let minx = Infinity,
      miny = Infinity,
      maxx = -Infinity,
      maxy = -Infinity;
    for (const [px, py] of pts) {
      minx = Math.min(minx, px);
      miny = Math.min(miny, py);
      maxx = Math.max(maxx, px);
      maxy = Math.max(maxy, py);
    }
    return [(minx + maxx) / 2, (miny + maxy) / 2];
  }

  function drawSelection(ctx: CanvasRenderingContext2D) {
    if (!scene.selected.length) return;
    ctx.strokeStyle = "#4c82f7";
    ctx.lineWidth = 1;
    ctx.setLineDash([4, 3]);
    for (const idx of scene.selected) {
      const s = scene.shapes[idx];
      if (!s) continue;
      // Vereinfachte, achsenparallele Auswahlbox (ohne Rotation) fürs Erste.
      const b = shapeBBox(s);
      const [x, y] = toScreen(b[0], b[1]);
      ctx.strokeRect(x - 3, y - 3, b[2] * zoom + 6, b[3] * zoom + 6);
    }
    ctx.setLineDash([]);
  }

  function shapeBBox(s: Shape): [number, number, number, number] {
    if ("Rect" in s.geo) {
      const { x, y, w, h } = s.geo.Rect;
      return [x, y, w, h];
    }
    if ("Ellipse" in s.geo) {
      const { cx, cy, rx, ry } = s.geo.Ellipse;
      return [cx - rx, cy - ry, rx * 2, ry * 2];
    }
    const { pts } = s.geo.Polyline;
    let minx = Infinity,
      miny = Infinity,
      maxx = -Infinity,
      maxy = -Infinity;
    for (const [px, py] of pts) {
      minx = Math.min(minx, px);
      miny = Math.min(miny, py);
      maxx = Math.max(maxx, px);
      maxy = Math.max(maxy, py);
    }
    return [minx, miny, maxx - minx, maxy - miny];
  }

  function onClick(ev: MouseEvent) {
    const rect = canvasEl.getBoundingClientRect();
    const [xmm, ymm] = toMm(ev.clientX - rect.left, ev.clientY - rect.top);
    onpick(xmm, ymm);
  }

  function onWheel(ev: WheelEvent) {
    ev.preventDefault();
    const rect = canvasEl.getBoundingClientRect();
    const px = ev.clientX - rect.left;
    const py = ev.clientY - rect.top;
    const [wx, wy] = toMm(px, py);
    zoom = Math.max(0.05, Math.min(40, zoom * (ev.deltaY < 0 ? 1.15 : 0.85)));
    // Punkt unter der Maus fix halten.
    panX = px - wx * zoom;
    panY = py - wy * zoom;
  }

  function resize() {
    if (!wrapEl || !canvasEl) return;
    canvasEl.width = wrapEl.clientWidth;
    canvasEl.height = wrapEl.clientHeight;
    draw();
  }

  // Neu zeichnen, wenn sich Szene oder Ansicht ändert.
  $effect(() => {
    // Abhängigkeiten registrieren:
    scene;
    zoom;
    panX;
    panY;
    draw();
  });

  $effect(() => {
    resize();
    const ro = new ResizeObserver(resize);
    if (wrapEl) ro.observe(wrapEl);
    return () => ro.disconnect();
  });
</script>

<div class="wrap" bind:this={wrapEl}>
  <canvas
    bind:this={canvasEl}
    onclick={onClick}
    onwheel={onWheel}
  ></canvas>
</div>

<style>
  .wrap {
    position: absolute;
    inset: 0;
  }
  canvas {
    display: block;
    cursor: crosshair;
  }
</style>
