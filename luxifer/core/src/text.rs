//! Text → Vektorpfade: Glyph-Outlines eines Fonts als geschlossene Konturen.
//!
//! Reine Geometrie (UI-frei, testbar): der Aufrufer liefert die Font-Bytes
//! (TTF/OTF), der Core parst die Glyphen (`ttf-parser`), reiht sie mit ihrem
//! horizontalen Advance auf, flattet die Bézier-Kurven adaptiv und liefert
//! Konturen in **mm** (y nach unten, Ursprung = linke Oberkante der Zeile).
//! Buchstaben-Innenräume (Löcher wie im „O") sind eigene Konturen — die
//! Even-Odd-Füllung spart sie automatisch aus.
//!
//! Layout (Ausrichtung, Zeilen-/Zeichenabstand) lebt hier im Core, nicht im
//! Frontend: `layout_text` nimmt `TextOptions`, `text_to_contours` bleibt als
//! Wrapper mit Standardwerten für bestehende Aufrufer.
//!
//! Nach v3-Analyse neu gebaut (CLAUDE.md Regel 6); dieselbe Bibliothekswahl
//! (`ttf-parser`), eigene Umsetzung. Mehrzeilig über `\n`.

use crate::geometry::Pt;
use serde::{Deserialize, Serialize};
use ttf_parser::{Face, OutlineBuilder};

/// Fehler beim Font-Parsen.
#[derive(Debug)]
pub struct TextError(pub String);

impl std::fmt::Display for TextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for TextError {}

/// Horizontale Ausrichtung der Zeilen innerhalb des Textblocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Layout-Parameter für `layout_text`. Zeilenhöhe = `size_mm * line_spacing`;
/// `letter_spacing_mm` wird zwischen (nicht nach) den Zeichen eingefügt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextOptions {
    /// Em-Größe in mm (wie Punktgröße).
    pub size_mm: f64,
    pub align: TextAlign,
    /// Zeilenabstand als Faktor der Em-Größe.
    pub line_spacing: f64,
    /// Zusätzlicher Abstand zwischen Zeichen in mm (auch negativ erlaubt).
    pub letter_spacing_mm: f64,
}

/// Standard-Zeilenabstand (Faktor der Em-Größe).
pub const DEFAULT_LINE_SPACING: f64 = 1.25;

impl Default for TextOptions {
    fn default() -> Self {
        Self {
            size_mm: 20.0,
            align: TextAlign::Left,
            line_spacing: DEFAULT_LINE_SPACING,
            letter_spacing_mm: 0.0,
        }
    }
}

/// Wandelt `text` mit dem Font (`font_data`: TTF/OTF-Bytes) in geschlossene
/// Konturen um — Wrapper um `layout_text` mit Standard-Layout (linksbündig,
/// Zeilenabstand 1,25). `size_mm` = Em-Größe (wie Punktgröße).
pub fn text_to_contours(
    font_data: &[u8],
    text: &str,
    size_mm: f64,
) -> Result<Vec<(Vec<Pt>, bool)>, TextError> {
    layout_text(
        font_data,
        text,
        &TextOptions {
            size_mm,
            ..TextOptions::default()
        },
    )
}

/// Setzt `text` mit Layout-Parametern in geschlossene Konturen um.
/// Ursprung (0,0) = linke Oberkante des Blocks; y wächst nach unten.
/// Bei Center/Right beziehen sich die Zeilen auf die breiteste Zeile.
pub fn layout_text(
    font_data: &[u8],
    text: &str,
    opts: &TextOptions,
) -> Result<Vec<(Vec<Pt>, bool)>, TextError> {
    let face = Face::parse(font_data, 0).map_err(|e| TextError(format!("Font unlesbar: {e}")))?;
    let upm = face.units_per_em() as f64;
    if upm <= 0.0 {
        return Err(TextError("Font ohne units_per_em".into()));
    }
    let scale = opts.size_mm / upm;
    let ascender = face.ascender() as f64 * scale;
    let line_height = opts.size_mm * opts.line_spacing;

    // Erst alle Zeilenbreiten messen, damit Center/Right relativ zur
    // breitesten Zeile ausgerichtet werden können.
    let lines: Vec<&str> = text.split('\n').collect();
    let widths: Vec<f64> = lines
        .iter()
        .map(|line| line_width(&face, line, scale, opts))
        .collect();
    let block_w = widths.iter().cloned().fold(0.0, f64::max);

    let mut out: Vec<(Vec<Pt>, bool)> = Vec::new();
    let mut y_line = 0.0_f64;
    for (line, width) in lines.iter().zip(&widths) {
        let mut x_pen = match opts.align {
            TextAlign::Left => 0.0,
            TextAlign::Center => (block_w - width) / 2.0,
            TextAlign::Right => block_w - width,
        };
        let mut first = true;
        for ch in line.chars() {
            if !first {
                x_pen += opts.letter_spacing_mm;
            }
            first = false;
            // Outline sammeln (Leerzeichen haben keine).
            let mut b = Flattener {
                scale,
                x_off: x_pen,
                // Font: y nach oben. Unser System: y nach unten, Zeilen-
                // Oberkante = Ascender-Linie.
                y_base: y_line + ascender,
                cur: Vec::new(),
                contours: Vec::new(),
                start: (0.0, 0.0),
            };
            if let Some(gid) = face.glyph_index(ch) {
                face.outline_glyph(gid, &mut b);
                for c in b.contours {
                    if c.len() >= 3 {
                        out.push((c, true));
                    }
                }
            }
            x_pen += char_advance(&face, ch, scale, opts.size_mm);
        }
        y_line += line_height;
    }
    Ok(out)
}

/// Advance eines Zeichens in mm; unbekannte Zeichen bekommen Em/2 Leerraum.
fn char_advance(face: &Face, ch: char, scale: f64, size_mm: f64) -> f64 {
    face.glyph_index(ch)
        .and_then(|gid| face.glyph_hor_advance(gid))
        .map(|a| a as f64 * scale)
        .unwrap_or(size_mm * 0.5)
}

/// Breite einer Zeile in mm (Advances + Zeichenabstände zwischen den Zeichen).
fn line_width(face: &Face, line: &str, scale: f64, opts: &TextOptions) -> f64 {
    let mut w = 0.0;
    let mut count = 0usize;
    for ch in line.chars() {
        w += char_advance(face, ch, scale, opts.size_mm);
        count += 1;
    }
    if count > 1 {
        w += opts.letter_spacing_mm * (count - 1) as f64;
    }
    w
}

/// Sammelt Glyph-Outlines als geflattete Polylinien. Quadratische und kubische
/// Béziers werden mit fester Unterteilung angenähert (fein genug für mm-Maße;
/// die Segmentzahl skaliert die Punktdichte, nicht die Korrektheit).
struct Flattener {
    scale: f64,
    x_off: f64,
    y_base: f64,
    cur: Vec<Pt>,
    contours: Vec<Vec<Pt>>,
    start: Pt,
}

impl Flattener {
    fn map(&self, x: f32, y: f32) -> Pt {
        (
            self.x_off + x as f64 * self.scale,
            self.y_base - y as f64 * self.scale,
        )
    }
}

/// Unterteilungen je Kurvensegment.
const CURVE_SEGS: usize = 8;

impl OutlineBuilder for Flattener {
    fn move_to(&mut self, x: f32, y: f32) {
        if self.cur.len() >= 3 {
            self.contours.push(std::mem::take(&mut self.cur));
        } else {
            self.cur.clear();
        }
        self.start = self.map(x, y);
        self.cur.push(self.start);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.cur.push(self.map(x, y));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let p0 = *self.cur.last().unwrap_or(&self.start);
        let c = self.map(x1, y1);
        let p1 = self.map(x, y);
        for i in 1..=CURVE_SEGS {
            let t = i as f64 / CURVE_SEGS as f64;
            let u = 1.0 - t;
            let px = u * u * p0.0 + 2.0 * u * t * c.0 + t * t * p1.0;
            let py = u * u * p0.1 + 2.0 * u * t * c.1 + t * t * p1.1;
            self.cur.push((px, py));
        }
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let p0 = *self.cur.last().unwrap_or(&self.start);
        let c1 = self.map(x1, y1);
        let c2 = self.map(x2, y2);
        let p1 = self.map(x, y);
        for i in 1..=CURVE_SEGS {
            let t = i as f64 / CURVE_SEGS as f64;
            let u = 1.0 - t;
            let px = u * u * u * p0.0
                + 3.0 * u * u * t * c1.0
                + 3.0 * u * t * t * c2.0
                + t * t * t * p1.0;
            let py = u * u * u * p0.1
                + 3.0 * u * u * t * c1.1
                + 3.0 * u * t * t * c2.1
                + t * t * t * p1.1;
            self.cur.push((px, py));
        }
    }

    fn close(&mut self) {
        if self.cur.len() >= 3 {
            self.contours.push(std::mem::take(&mut self.cur));
        } else {
            self.cur.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Einen System-Font finden, der echte Outlines liefert (Color-Emoji-
    /// Fonts z. B. haben keine). Test überspringt, wenn keiner da ist —
    /// CI-sicher; auf dem Zielsystem ist immer einer vorhanden.
    fn any_system_font() -> Option<Vec<u8>> {
        for dir in ["/usr/share/fonts", "/usr/local/share/fonts"] {
            let mut stack = vec![std::path::PathBuf::from(dir)];
            while let Some(d) = stack.pop() {
                let Ok(rd) = std::fs::read_dir(&d) else {
                    continue;
                };
                for e in rd.flatten() {
                    let p = e.path();
                    if p.is_dir() {
                        stack.push(p);
                    } else if p.extension().is_some_and(|x| x == "ttf" || x == "otf") {
                        if let Ok(b) = std::fs::read(&p) {
                            // Nur Fonts, die für "A" wirklich Konturen liefern.
                            if text_to_contours(&b, "A", 10.0)
                                .map(|c| !c.is_empty())
                                .unwrap_or(false)
                            {
                                return Some(b);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn bbox(cs: &[(Vec<Pt>, bool)]) -> (f64, f64, f64, f64) {
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for (c, _) in cs {
            for &(x, y) in c {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
            }
        }
        (x0, y0, x1, y1)
    }

    #[test]
    fn text_liefert_konturen_in_erwarteter_groesse() {
        let Some(font) = any_system_font() else {
            eprintln!("kein Systemfont — Test übersprungen");
            return;
        };
        let out = text_to_contours(&font, "LuxiFer", 20.0).unwrap();
        assert!(!out.is_empty(), "Buchstaben ergeben Konturen");
        // Bounding-Box: Höhe grob in der Größenordnung der Em-Größe.
        let (_, y0, _, y1) = bbox(&out);
        let h = y1 - y0;
        assert!(h > 5.0 && h < 30.0, "Texthöhe ~Em-Größe, war {h:.1}");
        // Alle Konturen geschlossen.
        assert!(out.iter().all(|(_, closed)| *closed));
    }

    #[test]
    fn o_hat_aussen_und_innenkontur() {
        let Some(font) = any_system_font() else {
            return;
        };
        let out = text_to_contours(&font, "O", 20.0).unwrap();
        assert!(
            out.len() >= 2,
            "O = Außenrand + Innenloch, war {}",
            out.len()
        );
    }

    #[test]
    fn mehrzeilig_versetzt_nach_unten() {
        let Some(font) = any_system_font() else {
            return;
        };
        let one = text_to_contours(&font, "A", 10.0).unwrap();
        let two = text_to_contours(&font, "A\nA", 10.0).unwrap();
        let max_y = |cs: &[(Vec<Pt>, bool)]| {
            cs.iter()
                .flat_map(|(c, _)| c.iter().map(|p| p.1))
                .fold(f64::MIN, f64::max)
        };
        assert!(max_y(&two) > max_y(&one) + 5.0, "zweite Zeile liegt tiefer");
    }

    #[test]
    fn zeilenabstand_faktor_wirkt() {
        let Some(font) = any_system_font() else {
            return;
        };
        let opts = |ls: f64| TextOptions {
            size_mm: 10.0,
            line_spacing: ls,
            ..TextOptions::default()
        };
        let tight = layout_text(&font, "A\nA", &opts(1.0)).unwrap();
        let wide = layout_text(&font, "A\nA", &opts(2.0)).unwrap();
        let (_, _, _, y_tight) = bbox(&tight);
        let (_, _, _, y_wide) = bbox(&wide);
        // Doppelter Faktor → zweite Zeile liegt ~10 mm tiefer.
        assert!(
            (y_wide - y_tight - 10.0).abs() < 0.5,
            "Δ war {:.2}",
            y_wide - y_tight
        );
    }

    #[test]
    fn zeichenabstand_verbreitert_die_zeile() {
        let Some(font) = any_system_font() else {
            return;
        };
        let opts = |sp: f64| TextOptions {
            size_mm: 10.0,
            letter_spacing_mm: sp,
            ..TextOptions::default()
        };
        let normal = layout_text(&font, "AAA", &opts(0.0)).unwrap();
        let spaced = layout_text(&font, "AAA", &opts(3.0)).unwrap();
        let w = |cs: &_| {
            let (x0, _, x1, _) = bbox(cs);
            x1 - x0
        };
        // 2 Lücken × 3 mm = 6 mm breiter.
        assert!(
            (w(&spaced) - w(&normal) - 6.0).abs() < 0.5,
            "Δ war {:.2}",
            w(&spaced) - w(&normal)
        );
    }

    #[test]
    fn ausrichtung_verschiebt_kurze_zeile() {
        let Some(font) = any_system_font() else {
            return;
        };
        let opts = |align: TextAlign| TextOptions {
            size_mm: 10.0,
            align,
            ..TextOptions::default()
        };
        // Kurze zweite Zeile: bei Right muss deren linker Rand deutlich
        // weiter rechts liegen als bei Left; Center liegt dazwischen.
        let min_x_line2 = |cs: &[(Vec<Pt>, bool)]| {
            cs.iter()
                .flat_map(|(c, _)| c.iter())
                .filter(|p| p.1 > 10.0) // Punkte der zweiten Zeile
                .map(|p| p.0)
                .fold(f64::MAX, f64::min)
        };
        let left = layout_text(&font, "MMMMM\nA", &opts(TextAlign::Left)).unwrap();
        let center = layout_text(&font, "MMMMM\nA", &opts(TextAlign::Center)).unwrap();
        let right = layout_text(&font, "MMMMM\nA", &opts(TextAlign::Right)).unwrap();
        let (l, c, r) = (
            min_x_line2(&left),
            min_x_line2(&center),
            min_x_line2(&right),
        );
        assert!(l < c && c < r, "Left {l:.1} < Center {c:.1} < Right {r:.1}");
    }

    #[test]
    fn kaputte_bytes_geben_fehler() {
        assert!(text_to_contours(&[1, 2, 3], "x", 10.0).is_err());
    }
}
