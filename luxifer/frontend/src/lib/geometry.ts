// Geometrie-Primitive fürs Frontend: EINE Quelle für die Punkt-Erzeugung je
// Form. Vorher rechnete jede Ansicht (WebGL-Kontur, BBox, 2D-Füllung, Thumbnail)
// ihre eigenen Ellipsen-Punkte — teils mit verschiedener Auflösung (96 vs 64).
// Das war stille Duplizierung (CLAUDE.md §1). Hier gebündelt: eine Änderung an
// der Auflösung wirkt überall gleich.
//
// Reine Mathematik (UI-frei). Die maßgebliche Geometrie liegt weiterhin im
// Rust-Core; dies ist die Frontend-Darstellungsauflösung fürs Zeichnen.

export type Pt = [number, number];

/** Segmente je Vollkreis für die Ellipsen-/Kreis-Tesselierung im Frontend. */
export const ELLIPSE_SEGS = 96;

/**
 * Punkte einer Ellipse (geschlossene Kontur, gegen den Uhrzeigersinn), `segs`
 * gleichmäßig verteilt. Default `ELLIPSE_SEGS` — überall dieselbe Auflösung.
 */
export function ellipsePoints(cx: number, cy: number, rx: number, ry: number, segs = ELLIPSE_SEGS): Pt[] {
  return Array.from({ length: segs }, (_, i) => {
    const a = (i / segs) * Math.PI * 2;
    return [cx + rx * Math.cos(a), cy + ry * Math.sin(a)] as Pt;
  });
}

/** Eckpunkte eines achsparallelen Rechtecks (im Uhrzeigersinn ab oben-links). */
export function rectPoints(x: number, y: number, w: number, h: number): Pt[] {
  return [[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
}

/**
 * Rotiert Punkte um die Mitte ihrer eigenen achsparallelen Bounding-Box (Grad).
 * Identisch zur Konvention in Core (Shape::bbox) und im 2D-Zeichenpfad.
 */
export function rotateAroundBBoxCenter(pts: Pt[], deg: number): Pt[] {
  if (Math.abs(deg) <= Number.EPSILON || pts.length === 0) return pts;
  let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
  for (const [x, y] of pts) {
    x0 = Math.min(x0, x); y0 = Math.min(y0, y);
    x1 = Math.max(x1, x); y1 = Math.max(y1, y);
  }
  const cx = (x0 + x1) / 2, cy = (y0 + y1) / 2;
  const r = (deg * Math.PI) / 180, co = Math.cos(r), si = Math.sin(r);
  return pts.map(([x, y]) => [cx + (x - cx) * co - (y - cy) * si, cy + (x - cx) * si + (y - cy) * co] as Pt);
}

/** Achsparallele Bounding-Box [x, y, w, h] einer Punktmenge. */
export function boundsOf(pts: Pt[]): [number, number, number, number] {
  if (pts.length === 0) return [0, 0, 0, 0];
  let a = Infinity, b = Infinity, c = -Infinity, d = -Infinity;
  for (const [x, y] of pts) { a = Math.min(a, x); b = Math.min(b, y); c = Math.max(c, x); d = Math.max(d, y); }
  return [a, b, c - a, d - b];
}
