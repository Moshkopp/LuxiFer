//! Geometrie-Werkzeuge: Boolean (Vereinigen/Schneiden/Abziehen), Offset
//! (parallele Kontur) und Fillet (Eckenverrundung). Reine mm-Geometrie,
//! UI-frei, testbar.
//!
//! Nach v3-Analyse neu aufgesetzt (CLAUDE.md Regel 6). Bewusste Abweichung:
//! v3 rollte Greiner-Hormann selbst (377 Zeilen Schnittpunkt-Topologie, ein
//! bekanntes Kantenfall-Minenfeld) — wir nutzen die erprobte `i_overlay`-
//! Bibliothek. Beim Offset traf schon v3 dieselbe Wahl (`cavalier_contours`).
//! Fillet ist überschaubare Trigonometrie und selbst implementiert.
//!
//! Löcher: Ergebnisse mit Innenkonturen kommen als **separate geschlossene
//! Polylinien** zurück — die Even-Odd-Scanline (scanline.rs) spart sie beim
//! Füllen automatisch aus; es braucht kein Loch-Konzept im Datenmodell.

use crate::geometry::Pt;
use cavalier_contours::polyline::{PlineSource, PlineSourceMut, Polyline};
use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;

/// Boolesche Operation zweier Polygon-Mengen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolOp {
    /// Vereinigen (A ∪ B).
    Union,
    /// Schneiden (A ∩ B).
    Intersect,
    /// Abziehen (A − B).
    Difference,
}

/// Führt die boolesche Operation aus. `subject`/`clip` sind geschlossene
/// Konturen in mm (Weltkoordinaten, Rotation bereits angewandt). Ergebnis:
/// geschlossene Konturen — Außenränder UND Löcher als eigene Polylinien.
pub fn boolean(subject: &[Vec<Pt>], clip: &[Vec<Pt>], op: BoolOp) -> Vec<Vec<Pt>> {
    let to_lib = |contours: &[Vec<Pt>]| -> Vec<Vec<[f64; 2]>> {
        contours
            .iter()
            .map(|c| c.iter().map(|&(x, y)| [x, y]).collect())
            .collect()
    };
    let subj = to_lib(subject);
    let clip = to_lib(clip);
    let rule = match op {
        BoolOp::Union => OverlayRule::Union,
        BoolOp::Intersect => OverlayRule::Intersect,
        BoolOp::Difference => OverlayRule::Difference,
    };
    // Ergebnis: Shapes → Konturen (erste = Außenrand, weitere = Löcher).
    // Wir flachen zu einer Konturliste ab (Even-Odd übernimmt die Löcher).
    let shapes = subj.overlay(&clip, rule, FillRule::EvenOdd);
    let mut out = Vec::new();
    for shape in shapes {
        for contour in shape {
            if contour.len() >= 3 {
                out.push(contour.into_iter().map(|p| (p[0], p[1])).collect());
            }
        }
    }
    out
}

/// Parallele Kontur im Abstand `dist` (mm): positiv = nach außen, negativ =
/// nach innen (bei geschlossenen Konturen). Kann mehrere Konturen liefern
/// (Selbstüberschneidungen werden aufgelöst) oder keine (Kontur kollabiert).
/// Bögen der Offset-Kurve werden zu Polylinien-Segmenten abgeflacht.
pub fn offset(points: &[Pt], closed: bool, dist: f64) -> Vec<Vec<Pt>> {
    if points.len() < 2 || dist == 0.0 {
        return Vec::new();
    }
    let mut pl: Polyline<f64> = if closed {
        Polyline::new_closed()
    } else {
        Polyline::new()
    };
    for &(x, y) in points {
        pl.add(x, y, 0.0);
    }
    // cavalier_contours: positives Offset = in Flächenrichtung. Damit
    // „positiv = außen" unabhängig vom Umlaufsinn gilt, normieren wir auf
    // gegen den Uhrzeigersinn (positive Fläche).
    if closed && pl.area() < 0.0 {
        pl.invert_direction_mut();
    }
    let mut out = Vec::new();
    for res in pl.parallel_offset(-dist) {
        // Bögen (bulge ≠ 0) zu Liniensegmenten abflachen (0,01 mm Toleranz).
        let flat = res.arcs_to_approx_lines(0.01).unwrap_or(res);
        let pts: Vec<Pt> = (0..flat.vertex_count())
            .map(|i| {
                let v = flat.at(i);
                (v.x, v.y)
            })
            .collect();
        if pts.len() >= 2 {
            out.push(pts);
        }
    }
    out
}

/// Verrundet die Ecken einer Polylinie mit Radius `r` (mm): jede Ecke wird
/// durch einen Kreisbogen (als Segmentzug) ersetzt. Ecken, deren Schenkel für
/// den Radius zu kurz sind, bleiben spitz. Offene Konturen behalten Anfangs-
/// und Endpunkt.
pub fn fillet(points: &[Pt], closed: bool, r: f64) -> Vec<Pt> {
    let n = points.len();
    if n < 3 || r <= 0.0 {
        return points.to_vec();
    }
    /// Segmente je Viertelkreis — fein genug für Laser-Konturen.
    const ARC_SEGS: usize = 8;

    let mut out: Vec<Pt> = Vec::new();
    let corner_count = if closed { n } else { n - 2 };
    if !closed {
        out.push(points[0]);
    }
    for k in 0..corner_count {
        // Ecke p mit Nachbarn a (davor) und b (danach).
        let (ia, ip, ib) = if closed {
            ((k + n - 1) % n, k, (k + 1) % n)
        } else {
            (k, k + 1, k + 2)
        };
        let (a, p, b) = (points[ia], points[ip], points[ib]);
        match corner_arc(a, p, b, r, ARC_SEGS) {
            Some(arc) => out.extend(arc),
            None => out.push(p), // zu kurze Schenkel/kollinear: Ecke bleibt
        }
    }
    if !closed {
        out.push(points[n - 1]);
    }
    out
}

/// Bogenpunkte für die Ecke `p` (Schenkel zu `a` und `b`) mit Radius `r`.
/// `None`, wenn die Schenkel zu kurz sind oder die Ecke (nahezu) gerade ist.
fn corner_arc(a: Pt, p: Pt, b: Pt, r: f64, segs: usize) -> Option<Vec<Pt>> {
    let (v1, l1) = unit(p, a)?;
    let (v2, l2) = unit(p, b)?;
    // Halber Eckwinkel über das Skalarprodukt.
    let cos_full = (v1.0 * v2.0 + v1.1 * v2.1).clamp(-1.0, 1.0);
    let full = cos_full.acos();
    if full < 1e-3 || (std::f64::consts::PI - full) < 1e-3 {
        return None; // spitz zusammengefaltet oder gerade — nichts zu runden
    }
    let half = full / 2.0;
    // Abstand der Tangentenpunkte von der Ecke.
    let t = r / (half.tan());
    if t > l1 * 0.5 || t > l2 * 0.5 {
        return None; // Schenkel zu kurz — Ecke bleibt spitz
    }
    let t1 = (p.0 + v1.0 * t, p.1 + v1.1 * t);
    let t2 = (p.0 + v2.0 * t, p.1 + v2.1 * t);
    // Bogenmittelpunkt: von der Ecke entlang der Winkelhalbierenden.
    let bis = ((v1.0 + v2.0) / 2.0, (v1.1 + v2.1) / 2.0);
    let bl = (bis.0 * bis.0 + bis.1 * bis.1).sqrt();
    if bl < 1e-12 {
        return None;
    }
    let d = r / half.sin();
    let c = (p.0 + bis.0 / bl * d, p.1 + bis.1 / bl * d);
    // Winkel von c zu den Tangentenpunkten; kurzen Bogen interpolieren.
    let a1 = (t1.1 - c.1).atan2(t1.0 - c.0);
    let a2 = (t2.1 - c.1).atan2(t2.0 - c.0);
    let mut sweep = a2 - a1;
    while sweep > std::f64::consts::PI {
        sweep -= std::f64::consts::TAU;
    }
    while sweep < -std::f64::consts::PI {
        sweep += std::f64::consts::TAU;
    }
    let steps = ((segs as f64) * (sweep.abs() / (std::f64::consts::PI / 2.0))).ceil() as usize;
    let steps = steps.max(2);
    let mut arc = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let ang = a1 + sweep * (i as f64 / steps as f64);
        arc.push((c.0 + r * ang.cos(), c.1 + r * ang.sin()));
    }
    Some(arc)
}

/// Einheitsvektor von `from` nach `to` + Länge; `None` bei (nahezu) Null.
fn unit(from: Pt, to: Pt) -> Option<((f64, f64), f64)> {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let l = (dx * dx + dy * dy).sqrt();
    if l < 1e-9 {
        return None;
    }
    Some(((dx / l, dy / l), l))
}

// ── AppState-Anbindung (Muster wie arrange.rs) ───────────────────────────────

use crate::geometry::{rotate_point, Geo};
use crate::state::AppState;

impl AppState {
    /// Weltkontur einer Shape (Rotation angewandt). `None` bei offenen
    /// Polylinien und Bildern — Boolean arbeitet nur auf geschlossenen Flächen.
    fn world_contour(&self, idx: usize) -> Option<Vec<Pt>> {
        let s = self.shapes.get(idx)?;
        if matches!(s.geo, Geo::Image { .. }) {
            return None;
        }
        let (mut pts, closed) = s.geo.outline_points();
        if !closed || pts.len() < 3 {
            return None;
        }
        if s.rotation != 0.0 {
            let (cx, cy) = s.bbox().center();
            for p in pts.iter_mut() {
                *p = rotate_point(p.0, p.1, cx, cy, s.rotation);
            }
        }
        Some(pts)
    }

    /// Ob die Auswahl boolesch verknüpfbar ist (≥ 2 geschlossene Vektor-Shapes).
    pub fn can_boolean(&self) -> bool {
        self.selected
            .iter()
            .filter(|&&i| self.world_contour(i).is_some())
            .count()
            >= 2
    }

    /// Boolesche Operation auf der Auswahl (ein Undo-Punkt). Subjekt ist die
    /// **zuerst** selektierte Shape, Clip sind die übrigen (bei `Difference`
    /// also: erste minus Rest). Die Eingabe-Shapes werden durch das Ergebnis
    /// (geschlossene Polylinien auf dem Layer des Subjekts) ersetzt.
    pub fn boolean_selected(&mut self, op: BoolOp) {
        let sel: Vec<usize> = self
            .selected
            .iter()
            .copied()
            .filter(|&i| self.world_contour(i).is_some())
            .collect();
        if sel.len() < 2 {
            return;
        }
        let subject = vec![self.world_contour(sel[0]).unwrap()];
        let clip: Vec<Vec<Pt>> = sel[1..]
            .iter()
            .map(|&i| self.world_contour(i).unwrap())
            .collect();
        let result = boolean(&subject, &clip, op);
        if result.is_empty() {
            return; // z. B. Schnitt ohne Überlappung — nichts kaputtmachen
        }

        self.push_undo();
        let layer_id = self.shapes[sel[0]].layer_id;
        // Eingaben entfernen (absteigend, Indizes bleiben gültig).
        let mut rm = sel.clone();
        rm.sort_unstable();
        for &i in rm.iter().rev() {
            self.shapes.remove(i);
        }
        // Ergebnis einfügen und selektieren.
        self.selected.clear();
        for contour in result {
            let idx = self.shapes.len();
            self.shapes.push(crate::model::Shape::new(
                layer_id,
                Geo::Polyline {
                    pts: contour,
                    closed: true,
                },
            ));
            self.selected.push(idx);
        }
        self.remove_empty_layers();
        self.dirty = true;
    }

    /// Parallele Kontur zu jeder selektierten Vektor-Shape hinzufügen (ein
    /// Undo-Punkt). Positiv = außen, negativ = innen. Die Originale bleiben —
    /// typischer Einsatz ist eine Schneidkontur um eine Gravur.
    pub fn offset_selected(&mut self, dist: f64) {
        let sel: Vec<usize> = self.selected.clone();
        let mut created: Vec<(usize, Geo)> = Vec::new();
        for &i in &sel {
            let Some(s) = self.shapes.get(i) else {
                continue;
            };
            if matches!(s.geo, Geo::Image { .. }) {
                continue;
            }
            let (mut pts, closed) = s.geo.outline_points();
            if pts.len() < 2 {
                continue;
            }
            if s.rotation != 0.0 {
                let (cx, cy) = s.bbox().center();
                for p in pts.iter_mut() {
                    *p = rotate_point(p.0, p.1, cx, cy, s.rotation);
                }
            }
            for contour in offset(&pts, closed, dist) {
                created.push((
                    s.layer_id,
                    Geo::Polyline {
                        pts: contour,
                        closed,
                    },
                ));
            }
        }
        if created.is_empty() {
            return;
        }
        self.push_undo();
        self.selected.clear();
        for (layer_id, geo) in created {
            let idx = self.shapes.len();
            self.shapes.push(crate::model::Shape::new(layer_id, geo));
            self.selected.push(idx);
        }
        self.dirty = true;
    }

    /// Verrundet die Ecken der selektierten Vektor-Shapes (ein Undo-Punkt).
    /// Die Shape wird durch die verrundete Polylinie ersetzt (Rotation wird
    /// dabei in die Punkte eingerechnet).
    pub fn fillet_selected(&mut self, radius: f64) {
        if radius <= 0.0 {
            return;
        }
        let sel: Vec<usize> = self.selected.clone();
        let mut any = false;
        // Erst prüfen, ob überhaupt eine Shape verrundbar ist (kein Undo umsonst).
        for &i in &sel {
            if let Some(s) = self.shapes.get(i) {
                if !matches!(s.geo, Geo::Image { .. } | Geo::Ellipse { .. }) {
                    any = true;
                }
            }
        }
        if !any {
            return;
        }
        self.push_undo();
        for &i in &sel {
            let Some(s) = self.shapes.get_mut(i) else {
                continue;
            };
            // Bilder nie; Ellipsen sind schon rund.
            if matches!(s.geo, Geo::Image { .. } | Geo::Ellipse { .. }) {
                continue;
            }
            let (mut pts, closed) = s.geo.outline_points();
            if pts.len() < 3 {
                continue;
            }
            if s.rotation != 0.0 {
                let bb = s.bbox();
                let (cx, cy) = bb.center();
                for p in pts.iter_mut() {
                    *p = rotate_point(p.0, p.1, cx, cy, s.rotation);
                }
                s.rotation = 0.0;
            }
            s.geo = Geo::Polyline {
                pts: fillet(&pts, closed, radius),
                closed,
            };
        }
        self.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f64, y: f64, w: f64, h: f64) -> Vec<Pt> {
        vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h)]
    }

    /// Fläche einer geschlossenen Kontur (Shoelace, Betrag).
    fn area(pts: &[Pt]) -> f64 {
        let n = pts.len();
        let mut a = 0.0;
        for i in 0..n {
            let (x1, y1) = pts[i];
            let (x2, y2) = pts[(i + 1) % n];
            a += x1 * y2 - x2 * y1;
        }
        (a / 2.0).abs()
    }

    #[test]
    fn union_zweier_ueberlappender_rechtecke() {
        // 10×10 und um (5,0) versetzt: Vereinigung = 10*10 + 10*10 − 5*10 = 150.
        let a = rect(0.0, 0.0, 10.0, 10.0);
        let b = rect(5.0, 0.0, 10.0, 10.0);
        let out = boolean(&[a], &[b], BoolOp::Union);
        assert_eq!(out.len(), 1, "eine zusammenhängende Kontur");
        assert!((area(&out[0]) - 150.0).abs() < 1e-6);
    }

    #[test]
    fn intersect_liefert_ueberlappung() {
        let a = rect(0.0, 0.0, 10.0, 10.0);
        let b = rect(5.0, 0.0, 10.0, 10.0);
        let out = boolean(&[a], &[b], BoolOp::Intersect);
        assert_eq!(out.len(), 1);
        assert!((area(&out[0]) - 50.0).abs() < 1e-6, "Schnitt = 5×10");
    }

    #[test]
    fn difference_zieht_ab() {
        let a = rect(0.0, 0.0, 10.0, 10.0);
        let b = rect(5.0, 0.0, 10.0, 10.0);
        let out = boolean(&[a], &[b], BoolOp::Difference);
        assert_eq!(out.len(), 1);
        assert!((area(&out[0]) - 50.0).abs() < 1e-6, "Rest = 5×10");
    }

    #[test]
    fn difference_mit_loch_liefert_zwei_konturen() {
        // Kleines Rechteck mittig aus großem ausstanzen → Außenrand + Loch.
        let a = rect(0.0, 0.0, 20.0, 20.0);
        let b = rect(5.0, 5.0, 10.0, 10.0);
        let out = boolean(&[a], &[b], BoolOp::Difference);
        assert_eq!(out.len(), 2, "Außenrand + Lochkontur");
        let sum: f64 = out.iter().map(|c| area(c)).sum();
        // Flächen beider Konturen: 400 (außen) + 100 (Loch) = 500.
        assert!((sum - 500.0).abs() < 1e-6);
    }

    #[test]
    fn getrennte_rechtecke_union_bleiben_zwei() {
        let a = rect(0.0, 0.0, 5.0, 5.0);
        let b = rect(20.0, 0.0, 5.0, 5.0);
        let out = boolean(&[a], &[b], BoolOp::Union);
        assert_eq!(out.len(), 2, "disjunkt bleibt zweiteilig");
    }

    #[test]
    fn offset_nach_aussen_vergroessert() {
        let sq = rect(0.0, 0.0, 10.0, 10.0);
        let out = offset(&sq, true, 2.0);
        assert_eq!(out.len(), 1);
        // 10×10 + 2mm außen: Fläche > 14×14 − Eckenrundung, sicher > 180.
        assert!(area(&out[0]) > 180.0, "war {}", area(&out[0]));
    }

    #[test]
    fn offset_nach_innen_verkleinert() {
        let sq = rect(0.0, 0.0, 10.0, 10.0);
        let out = offset(&sq, true, -2.0);
        assert_eq!(out.len(), 1);
        assert!(
            (area(&out[0]) - 36.0).abs() < 0.5,
            "innen 6×6, war {}",
            area(&out[0])
        );
    }

    #[test]
    fn offset_kollabiert_bei_zu_grossem_innenabstand() {
        let sq = rect(0.0, 0.0, 10.0, 10.0);
        let out = offset(&sq, true, -6.0);
        assert!(out.is_empty(), "6mm nach innen bei 10mm-Quadrat = weg");
    }

    #[test]
    fn offset_unabhaengig_vom_umlaufsinn() {
        // Gleiche Kontur, andersherum aufgezählt → gleiches Außen-Offset.
        let cw: Vec<Pt> = rect(0.0, 0.0, 10.0, 10.0).into_iter().rev().collect();
        let out = offset(&cw, true, 2.0);
        assert_eq!(out.len(), 1);
        assert!(area(&out[0]) > 180.0, "positiv muss auch bei CW außen sein");
    }

    #[test]
    fn fillet_rundet_quadratecken() {
        let sq = rect(0.0, 0.0, 10.0, 10.0);
        let out = fillet(&sq, true, 2.0);
        // Mehr Punkte als vorher (Bögen) …
        assert!(out.len() > 4);
        // … und die Fläche schrumpft um die Eckenabschnitte:
        // 100 − 4·(4 − π·4/4) ≈ 100 − 3,43 ≈ 96,57.
        let a = area(&out);
        assert!((a - 96.566).abs() < 0.2, "Fläche nach Fillet war {a}");
        // Kein Punkt liegt mehr auf der spitzen Ecke (0,0). Der nächste
        // Bogenpunkt hat Abstand r·(√2−1)·√2 ≈ 0,828 zur alten Ecke.
        assert!(out.iter().all(|&(x, y)| (x - 0.0).hypot(y - 0.0) > 0.8));
    }

    #[test]
    fn fillet_zu_grosser_radius_laesst_ecken_spitz() {
        let sq = rect(0.0, 0.0, 4.0, 4.0);
        let out = fillet(&sq, true, 10.0);
        assert_eq!(out.len(), 4, "Radius passt nicht → unverändert");
    }

    #[test]
    fn fillet_offene_kontur_behaelt_enden() {
        let l = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)];
        let out = fillet(&l, false, 2.0);
        assert_eq!(out[0], (0.0, 0.0));
        assert_eq!(*out.last().unwrap(), (10.0, 10.0));
        assert!(out.len() > 3, "die eine Ecke wurde verrundet");
    }

    // ── AppState-Verdrahtung ────────────────────────────────────────────────

    fn state_two_overlapping() -> AppState {
        let mut s = AppState::new();
        s.add_shape(Geo::Rect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
        });
        let c = s.layers[0].color;
        s.selected.clear();
        s.activate_color(c);
        s.add_shape(Geo::Rect {
            x: 5.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
        });
        s.selected = vec![0, 1];
        s
    }

    #[test]
    fn boolean_selected_ersetzt_shapes_und_undo_stellt_wieder_her() {
        let mut s = state_two_overlapping();
        assert!(s.can_boolean());
        s.boolean_selected(BoolOp::Union);
        assert_eq!(s.shapes.len(), 1, "zwei Rechtecke → eine Kontur");
        assert!(matches!(
            s.shapes[0].geo,
            Geo::Polyline { closed: true, .. }
        ));
        assert_eq!(s.selected, vec![0], "Ergebnis ist selektiert");
        s.undo();
        assert_eq!(s.shapes.len(), 2, "Undo stellt die Eingaben wieder her");
    }

    #[test]
    fn boolean_ohne_ueberlappung_intersect_aendert_nichts() {
        let mut s = AppState::new();
        s.add_shape(Geo::Rect {
            x: 0.0,
            y: 0.0,
            w: 5.0,
            h: 5.0,
        });
        let c = s.layers[0].color;
        s.selected.clear();
        s.activate_color(c);
        s.add_shape(Geo::Rect {
            x: 50.0,
            y: 0.0,
            w: 5.0,
            h: 5.0,
        });
        s.selected = vec![0, 1];
        s.boolean_selected(BoolOp::Intersect);
        assert_eq!(s.shapes.len(), 2, "leerer Schnitt zerstört nichts");
    }

    #[test]
    fn offset_selected_fuegt_kontur_hinzu_und_behaelt_original() {
        let mut s = AppState::new();
        s.add_shape(Geo::Rect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
        });
        s.selected = vec![0];
        s.offset_selected(2.0);
        assert_eq!(s.shapes.len(), 2, "Original + Offset-Kontur");
        assert!(matches!(s.shapes[1].geo, Geo::Polyline { .. }));
    }

    #[test]
    fn fillet_selected_ersetzt_rect_durch_polyline() {
        let mut s = AppState::new();
        s.add_shape(Geo::Rect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
        });
        s.selected = vec![0];
        s.fillet_selected(2.0);
        assert_eq!(s.shapes.len(), 1);
        let Geo::Polyline { ref pts, closed } = s.shapes[0].geo else {
            panic!("Polyline erwartet");
        };
        assert!(closed);
        assert!(pts.len() > 4, "Bögen eingefügt");
    }
}
