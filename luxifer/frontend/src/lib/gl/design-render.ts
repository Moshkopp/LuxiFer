// Wandelt die Design-Scene (Vektor-Shapes) in GPU-Line-Batches (ADR 0008).
// Reine Datenumformung — UI-frei und testbar (CLAUDE.md §1). Das Frontend
// zeichnet damit die Konturen in EINEM Draw-Call statt CPU-stroke() je Shape.
//
// Bilder (Image-Geo) und Overlays (Handles, Lineale, Mess-/Node-Griffe) sind
// NICHT hier — Bilder laufen als Texturen, Overlays auf dem 2D-Layer darüber.

import type { Scene, Shape } from "../core";
import type { LineBatch } from "./renderer";

/** Ein Weltpunkt-Transformer (mm→mm), z. B. für Live-Drag selektierter Shapes. */
export type PointXf = (x: number, y: number, shapeIdx: number) => [number, number];

const IDENTITY: PointXf = (x, y) => [x, y];

/** Auflösung der Ellipsen-Tesselierung (Segmente je Vollkreis). */
const ELLIPSE_SEGS = 96;

/**
 * Baut den Konturen-Batch aller sichtbaren Vektor-Shapes. `xf` transformiert
 * einzelne Weltpunkte (Default: Identität) — so wandert die Live-Drag-Geste
 * mit, ohne die Scene zu verändern. Farbe je Segment = Layer-Farbe der Shape.
 */
export function shapesToBatch(scene: Scene, xf: PointXf = IDENTITY): LineBatch {
  const pos: number[] = [];
  const col: number[] = [];
  scene.shapes.forEach((s, idx) => {
    if ("Image" in s.geo) return; // Bilder als Textur, nicht als Linie
    const [r, g, b] = layerRgb(scene, s);
    const pts = contourPoints(s, idx, xf);
    if (pts.length < 2) return;
    const closed = isClosed(s);
    const n = pts.length;
    const segs = closed ? n : n - 1;
    for (let i = 0; i < segs; i++) {
      const a = pts[i];
      const bpt = pts[(i + 1) % n];
      pos.push(a[0], a[1], bpt[0], bpt[1]);
      col.push(r, g, b, 1, r, g, b, 1);
    }
  });
  return { positions: new Float32Array(pos), colors: new Float32Array(col) };
}

/** Layer-Farbe der Shape als RGBA-Anteile 0..1 (Fallback: LuxiFer-Rot). */
function layerRgb(scene: Scene, s: Shape): [number, number, number] {
  const l = scene.layers[s.layer_id];
  const c = l ? l.color : [255, 92, 98];
  return [c[0] / 255, c[1] / 255, c[2] / 255];
}

/** Ob die Kontur geschlossen ist (Rect/Ellipse immer, Polyline nach Flag). */
function isClosed(s: Shape): boolean {
  if ("Rect" in s.geo || "Ellipse" in s.geo) return true;
  if ("Polyline" in s.geo) return s.geo.Polyline.closed;
  return false;
}

/**
 * Weltpunkte der Kontur in mm, inklusive Shape-Rotation und `xf`. Die Rotation
 * dreht um die Mitte der ungedrehten Bounding-Box — identisch zum bisherigen
 * 2D-Pfad (drawShape) und zu shapeBBox in core.ts.
 */
function contourPoints(s: Shape, idx: number, xf: PointXf): [number, number][] {
  let pts: [number, number][];
  if ("Rect" in s.geo) {
    const { x, y, w, h } = s.geo.Rect;
    pts = [[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
  } else if ("Ellipse" in s.geo) {
    const { cx, cy, rx, ry } = s.geo.Ellipse;
    pts = Array.from({ length: ELLIPSE_SEGS }, (_, i) => {
      const a = (i / ELLIPSE_SEGS) * Math.PI * 2;
      return [cx + rx * Math.cos(a), cy + ry * Math.sin(a)] as [number, number];
    });
  } else if ("Polyline" in s.geo) {
    pts = s.geo.Polyline.pts.map(([a, b]) => [a, b] as [number, number]);
  } else {
    return [];
  }
  // Rotation um die Mitte der ungedrehten Box (wie drawShape/shapeBBox).
  const rot = s.rotation ?? 0;
  if (Math.abs(rot) > Number.EPSILON) {
    let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
    for (const [x, y] of pts) {
      x0 = Math.min(x0, x); y0 = Math.min(y0, y);
      x1 = Math.max(x1, x); y1 = Math.max(y1, y);
    }
    const cx = (x0 + x1) / 2, cy = (y0 + y1) / 2;
    const rad = (rot * Math.PI) / 180, co = Math.cos(rad), si = Math.sin(rad);
    pts = pts.map(([x, y]) => [
      cx + (x - cx) * co - (y - cy) * si,
      cy + (x - cx) * si + (y - cy) * co,
    ]);
  }
  // Live-Drag-Transform ganz zuletzt (arbeitet im Weltraum wie liveTransformPoint).
  return pts.map(([x, y]) => xf(x, y, idx));
}
