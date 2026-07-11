// Theming-Helfer (ADR 0002 §3): setzt die CSS-Variablen fuer Akzent- und
// Button-Farbe aus den GUI-Settings. Eine Quelle, ueberall wirksam.
//
// Damit das Glas-Design (Frosted Depth) mit Transparenzen arbeiten kann, setzen
// wir die Farben nicht nur als fertiges hsl(), sondern auch die H/S/L-Kanaele
// einzeln als Variablen. So laesst sich im CSS jede Deckung frei bauen:
//   background: hsl(var(--accent-h) var(--accent-s) var(--accent-l) / 0.14)
// Der Farbton kommt als RGB, die "Kraeftigkeit" als Intensitaet (0.3…0.9, im
// Core auf lesbaren Korridor geklemmt).

import type { Theme, ThemeColor } from "./core";

/** RGB (0…255) -> HSL (h in Grad, s/l in 0…1). */
function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  let h = 0;
  let s = 0;
  const d = max - min;
  if (d !== 0) {
    s = d / (1 - Math.abs(2 * l - 1));
    switch (max) {
      case r:
        h = ((g - b) / d) % 6;
        break;
      case g:
        h = (b - r) / d + 2;
        break;
      default:
        h = (r - g) / d + 4;
    }
    h *= 60;
    if (h < 0) h += 360;
  }
  return [h, s, l];
}

/** HSL-Kanaele einer ThemeColor nach Intensitaets-Modulation (fuer CSS-Vars). */
export function themeColorHsl(c: ThemeColor): { h: number; s: number; l: number } {
  const [h, , l] = rgbToHsl(c.hue[0], c.hue[1], c.hue[2]);
  // Intensitaet (0.3…0.9) auf Saettigung ~35…95 % abbilden.
  const s = clamp(Math.round((0.35 + (c.intensity - 0.3)) * 100), 30, 95);
  // Helligkeit um den Ausgangswert leicht nach Intensitaet verschieben.
  const li = clamp(Math.round(l * 100 * (0.9 + c.intensity * 0.25)), 25, 70);
  return { h: Math.round(h), s, l: li };
}

/** Fertige hsl()-Farbe einer ThemeColor. */
export function themeColorToCss(c: ThemeColor): string {
  const { h, s, l } = themeColorHsl(c);
  return `hsl(${h}, ${s}%, ${l}%)`;
}

function clamp(v: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, v));
}

/**
 * Setzt die Theme-Variablen am Wurzelelement:
 *  --accent / --btn                  fertige Farben (Rueckwaertskompatibel)
 *  --accent-h/-s/-l, --btn-h/-s/-l   Kanaele fuer freie Transparenzen im CSS
 */
export function applyTheme(theme: Theme, root: HTMLElement = document.documentElement): void {
  const a = themeColorHsl(theme.accent);
  root.style.setProperty("--accent", `hsl(${a.h}, ${a.s}%, ${a.l}%)`);
  root.style.setProperty("--accent-h", `${a.h}`);
  root.style.setProperty("--accent-s", `${a.s}%`);
  root.style.setProperty("--accent-l", `${a.l}%`);
  // Die Bedien-Grundfläche (Toolbar/Buttons) bleibt bewusst FIX neutral-blaugrau
  // (Werte aus app.css, --btn-*). Die frei wählbare Button-Farbe färbt die
  // Oberfläche nicht mehr — nur das aktive Element trägt den Akzent. Deshalb
  // wird theme.button hier nicht mehr auf die CSS-Variablen geschrieben.
}
