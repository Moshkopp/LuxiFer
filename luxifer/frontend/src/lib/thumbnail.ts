// Thumbnail-Erzeugung fuer die Projektverwaltung (ADR 0003 §5).
//
// Reine DARSTELLUNG, kein Wahrheits-Zustand: rendert die aktuelle Szene in ein
// Offscreen-Canvas und liefert PNG-Bytes. Die echte Geometrie bleibt im Core;
// hier wird nur gezeichnet (konform mit CLAUDE.md Regel 2). Bewusst schlanker
// als Canvas.svelte (kein Grid, keine Auswahl/Handles) — nur Bett + Formen.
import { rgb, type Scene, type Shape } from "./core";

const W = 240;
const H = 180;

// BBox eines Shapes in mm (wie in Canvas.svelte, aber lokal gehalten).
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
  let a = Infinity, b = Infinity, c = -Infinity, d = -Infinity;
  for (const [px, py] of pts) {
    a = Math.min(a, px); b = Math.min(b, py); c = Math.max(c, px); d = Math.max(d, py);
  }
  return [a, b, c - a, d - b];
}

// Zeichnet die Szene ins gegebene 2D-Context, eingepasst in W×H.
function render(ctx: CanvasRenderingContext2D, scene: Scene) {
  ctx.fillStyle = "#141518";
  ctx.fillRect(0, 0, W, H);

  // Einpassung: Bett zentriert mit etwas Rand.
  const bw = scene.bed_w_mm || 1;
  const bh = scene.bed_h_mm || 1;
  const margin = 0.86;
  const zoom = Math.min(W / bw, H / bh) * margin;
  const panX = (W - bw * zoom) / 2;
  const panY = (H - bh * zoom) / 2;
  const sx = (x: number) => x * zoom + panX;
  const sy = (y: number) => y * zoom + panY;

  // Bett als dezenter Rahmen.
  ctx.strokeStyle = "rgba(90,150,220,0.6)";
  ctx.lineWidth = 1;
  ctx.strokeRect(sx(0), sy(0), bw * zoom, bh * zoom);

  // Formen in Layer-Farbe.
  for (const s of scene.shapes) {
    const l = scene.layers[s.layer_id];
    const color = l ? rgb(l.color) : "#ff5c62";
    const filled = !!l && (l.mode === "Fill" || l.mode === "Raster");
    ctx.strokeStyle = color;
    ctx.lineWidth = 1.2;
    ctx.beginPath();
    if ("Ellipse" in s.geo) {
      const [bx, by, w, h] = shapeBBox(s);
      ctx.ellipse(sx(bx) + (w * zoom) / 2, sy(by) + (h * zoom) / 2, (w * zoom) / 2, (h * zoom) / 2, 0, 0, Math.PI * 2);
    } else if ("Polyline" in s.geo) {
      const { pts, closed } = s.geo.Polyline;
      pts.forEach((p, i) => {
        const px = sx(p[0]), py = sy(p[1]);
        if (i === 0) ctx.moveTo(px, py); else ctx.lineTo(px, py);
      });
      if (closed) ctx.closePath();
    } else {
      const [bx, by, w, h] = shapeBBox(s);
      ctx.rect(sx(bx), sy(by), w * zoom, h * zoom);
    }
    if (filled) {
      const [r, g, b] = l ? l.color : [255, 92, 98];
      ctx.fillStyle = `rgba(${r}, ${g}, ${b}, 0.3)`;
      ctx.fill();
    }
    ctx.stroke();
  }
}

// Erzeugt ein PNG-Thumbnail der Szene als Byte-Array (fuer den save-Command).
// Leeres Array, wenn kein Canvas/Blob moeglich ist (Aufrufer speichert dann ohne).
export async function renderThumbnail(scene: Scene): Promise<number[]> {
  const canvas = document.createElement("canvas");
  canvas.width = W;
  canvas.height = H;
  const ctx = canvas.getContext("2d");
  if (!ctx) return [];
  render(ctx, scene);
  const blob = await new Promise<Blob | null>((resolve) =>
    canvas.toBlob((b) => resolve(b), "image/png"),
  );
  if (!blob) return [];
  const buf = await blob.arrayBuffer();
  return Array.from(new Uint8Array(buf));
}
