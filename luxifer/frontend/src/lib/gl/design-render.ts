// Wandelt die Design-Scene (Vektor-Shapes) in GPU-Line-Batches (ADR 0008).
// Reine Datenumformung — UI-frei und testbar (CLAUDE.md §1). Das Frontend
// zeichnet damit die Konturen in EINEM Draw-Call statt CPU-stroke() je Shape.
//
// Bilder (Image-Geo) und Overlays (Handles, Lineale, Mess-/Node-Griffe) sind
// NICHT hier — Bilder laufen als Texturen, Overlays auf dem 2D-Layer darüber.

import type { Scene, Shape } from "../core";
import type { LineBatch } from "./renderer";
import { type Pt, ellipsePoints, rectPoints, rotateAroundBBoxCenter } from "../geometry";

/** Ein Weltpunkt-Transformer (mm→mm), z. B. für Live-Drag selektierter Shapes. */
export type PointXf = (x: number, y: number, shapeIdx: number) => [number, number];

const IDENTITY: PointXf = (x, y) => [x, y];

/** Prädikat: soll die Shape mit diesem Index in den Batch? (Default: alle). */
export type ShapeFilter = (shapeIdx: number) => boolean;

const ALL: ShapeFilter = () => true;

export interface FillLayerData {
  positions: Float32Array;
  ranges: { shapeIdx: number; start: number; count: number }[];
  bounds: [number, number, number, number];
  color: [number, number, number, number];
  allSelected: boolean;
  layerId: number;
}

/**
 * Baut den Konturen-Batch aller sichtbaren Vektor-Shapes. `xf` transformiert
 * einzelne Weltpunkte (Default: Identität) — so wandert die Live-Drag-Geste
 * mit, ohne die Scene zu verändern. `include` filtert die Shapes (Default: alle),
 * damit der Aufrufer statischen und bewegten Teil getrennt batchen kann (Live-
 * Drag: nur die selektierten Shapes wandern pro Frame, der Rest bleibt gecacht).
 * Farbe je Segment = Layer-Farbe der Shape.
 */
export function shapesToBatch(
  scene: Scene,
  xf: PointXf = IDENTITY,
  include: ShapeFilter = ALL,
): LineBatch {
  const pos: number[] = [];
  const col: number[] = [];
  scene.shapes.forEach((s, idx) => {
    if ("Image" in s.geo) return; // Bilder als Textur, nicht als Linie
    if (!include(idx)) return;
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

/**
 * Baut die Konturenmarkierung echter Gruppen in der Auswahl als GPU-Batch.
 * Lose Mehrfachauswahl bleibt bei der gemeinsamen 2D-Auswahlbox; nur Shapes mit
 * group_id bekommen wie bisher einen eigenen Konturumriss.
 *
 * Die erste GPU-Fassung zeichnet bewusst durchgezogene 1-px-Linien. Die
 * bisherige 5/3-px-Strichelung benötigt bildschirmkonstante Distanzdaten und
 * wird als eigener Paritätsschritt ergänzt, ohne zum CPU-Canvas zurückzukehren.
 */
export function groupOutlinesToBatch(scene: Scene, xf: PointXf = IDENTITY): LineBatch {
  const pos: number[] = [];
  const col: number[] = [];
  const blue: [number, number, number, number] = [76 / 255, 130 / 255, 247 / 255, 0.9];
  for (const idx of scene.selected) {
    const s = scene.shapes[idx];
    if (!s || s.group_id == null || "Image" in s.geo) continue;
    const pts = contourPoints(s, idx, xf);
    if (pts.length < 2) continue;
    const segs = isClosed(s) ? pts.length : pts.length - 1;
    for (let i = 0; i < segs; i++) {
      const a = pts[i];
      const b = pts[(i + 1) % pts.length];
      pos.push(a[0], a[1], b[0], b[1]);
      col.push(...blue, ...blue);
    }
  }
  return { positions: new Float32Array(pos), colors: new Float32Array(col) };
}

/** Geschlossene Konturen gefüllter Layer für den Stencil-Even-Odd-Pfad. */
export function fillLayersToData(scene: Scene): FillLayerData[] {
  const selected = new Set(scene.selected);
  const layers = new Map<number, { pos: number[]; ranges: { shapeIdx: number; start: number; count: number }[]; minX: number; minY: number; maxX: number; maxY: number; allSelected: boolean }>();
  scene.shapes.forEach((s, idx) => {
    if ("Image" in s.geo || !isClosed(s)) return;
    const layer = scene.layers[s.layer_id];
    if (!layer || (layer.mode !== "Fill" && layer.mode !== "Raster")) return;
    const pts = contourPoints(s, idx, IDENTITY);
    if (pts.length < 3) return;
    let out = layers.get(s.layer_id);
    if (!out) {
      out = { pos: [], ranges: [], minX: Infinity, minY: Infinity, maxX: -Infinity, maxY: -Infinity, allSelected: true };
      layers.set(s.layer_id, out);
    }
    out.allSelected &&= selected.has(idx);
    for (const [x, y] of pts) {
      out.minX = Math.min(out.minX, x); out.minY = Math.min(out.minY, y);
      out.maxX = Math.max(out.maxX, x); out.maxY = Math.max(out.maxY, y);
    }
    // Fan einmalig zu Dreiecken expandieren. Im Stencil löschen sich
    // Überdeckungen per INVERT-Parität, aber alle Konturen eines Layers können
    // dadurch in EINEM drawArrays(TRIANGLES) statt n TRIANGLE_FAN-Calls laufen.
    const p0 = pts[0];
    const start = out.pos.length / 2;
    for (let i = 1; i + 1 < pts.length; i++) {
      const a = pts[i], b = pts[i + 1];
      out.pos.push(p0[0], p0[1], a[0], a[1], b[0], b[1]);
    }
    out.ranges.push({ shapeIdx: idx, start, count: out.pos.length / 2 - start });
  });
  return [...layers.entries()].map(([layerId, data]) => {
    const [r, g, b] = scene.layers[layerId].color;
    return {
      positions: new Float32Array(data.pos),
      ranges: data.ranges,
      bounds: [data.minX, data.minY, data.maxX - data.minX, data.maxY - data.minY],
      color: [r / 255, g / 255, b / 255, 0.32],
      allSelected: data.allSelected,
      layerId,
    };
  });
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
function contourPoints(s: Shape, idx: number, xf: PointXf): Pt[] {
  let pts: Pt[];
  if ("Rect" in s.geo) {
    const { x, y, w, h } = s.geo.Rect;
    pts = rectPoints(x, y, w, h);
  } else if ("Ellipse" in s.geo) {
    const { cx, cy, rx, ry } = s.geo.Ellipse;
    pts = ellipsePoints(cx, cy, rx, ry);
  } else if ("Polyline" in s.geo) {
    pts = s.geo.Polyline.pts.map(([a, b]) => [a, b] as Pt);
  } else {
    return [];
  }
  // Rotation um die Mitte der ungedrehten Box (wie drawShape/shapeBBox).
  pts = rotateAroundBBoxCenter(pts, s.rotation ?? 0);
  // Live-Drag-Transform ganz zuletzt (arbeitet im Weltraum wie liveTransformPoint).
  return pts.map(([x, y]) => xf(x, y, idx));
}
