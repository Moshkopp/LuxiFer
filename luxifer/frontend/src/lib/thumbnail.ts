// Thumbnail-Erzeugung fuer die Projektverwaltung (ADR 0003 §5).
//
// Reine DARSTELLUNG, kein Wahrheits-Zustand: rendert die aktuelle Szene in ein
// Offscreen-Canvas und liefert PNG-Bytes. Die echte Geometrie bleibt im Core;
// hier wird nur gezeichnet (konform mit CLAUDE.md Regel 2). Bewusst schlanker
// als Canvas.svelte (kein Grid, keine Auswahl/Handles) — nur Bett + Formen.
import { rgb, imageRender, type Scene, type Shape, type ImageParams } from "./core";

const W = 240;
const H = 180;

// BBox eines Shapes in mm (wie in Canvas.svelte, aber lokal gehalten).
function shapeBBox(s: Shape): [number, number, number, number] {
  if ("Rect" in s.geo) {
    const { x, y, w, h } = s.geo.Rect;
    return [x, y, w, h];
  }
  if ("Image" in s.geo) {
    const { x, y, w, h } = s.geo.Image;
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

// Lädt die (Graustufen-)Bitmaps aller Bild-Shapes vorab als HTMLImageElement.
// invert_editor wird berücksichtigt, damit das Thumbnail den Canvas-Stand zeigt.
async function loadImages(scene: Scene): Promise<Map<string, HTMLImageElement>> {
  const out = new Map<string, HTMLImageElement>();
  for (const s of scene.shapes) {
    if (!("Image" in s.geo)) continue;
    const { asset, params } = s.geo.Image;
    if (out.has(asset)) continue;
    // Neutral rendern (roh, nur invert_editor) — wie das Canvas (ADR 0004 §3).
    const neutral: ImageParams = {
      mode: "Grayscale",
      threshold: 128,
      brightness: 0,
      contrast: 0,
      gamma: 1.0,
      invert_editor: params.invert_editor,
      invert_laser: false,
    };
    const url = await imageRender(asset, neutral, params.invert_editor);
    if (!url) continue;
    const el = await new Promise<HTMLImageElement | null>((resolve) => {
      const img = new Image();
      img.onload = () => resolve(img);
      img.onerror = () => resolve(null);
      img.src = url;
    });
    if (el) out.set(asset, el);
  }
  return out;
}

// Zeichnet die Szene ins gegebene 2D-Context, eingepasst in W×H.
function render(ctx: CanvasRenderingContext2D, scene: Scene, images: Map<string, HTMLImageElement>) {
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
    // Bild: echtes Bitmap zeichnen (falls geladen), sonst Box in Layer-Farbe.
    if ("Image" in s.geo) {
      const [bx, by, w, h] = shapeBBox(s);
      const img = images.get(s.geo.Image.asset);
      if (img) {
        ctx.drawImage(img, sx(bx), sy(by), w * zoom, h * zoom);
      } else {
        const [r, g, b] = l ? l.color : [255, 92, 98];
        ctx.fillStyle = `rgba(${r}, ${g}, ${b}, 0.3)`;
        ctx.fillRect(sx(bx), sy(by), w * zoom, h * zoom);
      }
      ctx.strokeStyle = color;
      ctx.lineWidth = 1;
      ctx.strokeRect(sx(bx), sy(by), w * zoom, h * zoom);
      continue;
    }
    const filled = !!l && (l.mode === "Fill" || l.mode === "Raster" || l.mode === "Image");
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
  const images = await loadImages(scene);
  render(ctx, scene, images);
  const blob = await new Promise<Blob | null>((resolve) =>
    canvas.toBlob((b) => resolve(b), "image/png"),
  );
  if (!blob) return [];
  const buf = await blob.arrayBuffer();
  return Array.from(new Uint8Array(buf));
}
