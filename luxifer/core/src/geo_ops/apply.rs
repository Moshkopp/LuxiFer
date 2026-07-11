//! Anbindung der Geometrie-Werkzeuge an den Editor-Zustand: wendet
//! Boolean/Offset/Fillet/Haltesteg auf die aktuelle Auswahl an (Muster wie
//! arrange.rs). Die reine mm-Geometrie liegt im Elternmodul (super::).

use crate::geometry::{rotate_point, Geo, Pt};
use crate::state::AppState;

use super::{boolean, bridge_line, fillet, fillet_corners, offset, unit, BoolOp};

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

    /// Haltesteg: der Nutzer zieht eine Steg-Linie (`p0`→`p1`) der Breite
    /// `width` über die Konturen (v3-Modell). Jede Kontur, die die Linie
    /// kreuzt, wird dort **aufgeschnitten** (Lücke = Materialbrücke); die
    /// verbleibenden Teilstücke ersetzen sie. Ein Undo-Punkt. `true`, wenn
    /// mindestens eine Kontur getroffen wurde.
    pub fn bridge_stroke(&mut self, p0: Pt, p1: Pt, width: f64) -> bool {
        if width <= 0.0 {
            return false;
        }
        // Referenz-UX: Ein Klick (praktisch eine Null-Längen-Linie) sucht die
        // nächste Konturkante und legt automatisch eine senkrechte Schnittlinie
        // darüber. So funktioniert der Haltesteg auch ohne exakten Drag.
        let (p0, p1) = if (p1.0 - p0.0).hypot(p1.1 - p0.1) < 0.1 {
            let mut nearest: Option<(f64, Pt, Pt)> = None;
            for s in &self.shapes {
                if matches!(s.geo, Geo::Image { .. }) {
                    continue;
                }
                let (mut pts, closed) = s.geo.outline_points();
                if s.rotation != 0.0 {
                    let (cx, cy) = s.bbox().center();
                    for pt in &mut pts {
                        *pt = rotate_point(pt.0, pt.1, cx, cy, s.rotation);
                    }
                }
                let edges = if closed {
                    pts.len()
                } else {
                    pts.len().saturating_sub(1)
                };
                for i in 0..edges {
                    let a = pts[i];
                    let b = pts[(i + 1) % pts.len()];
                    let Some((dir, len)) = unit(a, b) else {
                        continue;
                    };
                    let t = (((p0.0 - a.0) * dir.0 + (p0.1 - a.1) * dir.1) / len).clamp(0.0, 1.0);
                    let projection = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
                    let distance = (p0.0 - projection.0).hypot(p0.1 - projection.1);
                    if nearest.is_none_or(|(best, _, _)| distance < best) {
                        nearest = Some((distance, projection, dir));
                    }
                }
            }
            let Some((distance, projection, edge_dir)) = nearest else {
                return false;
            };
            if distance > 10.0 {
                return false;
            }
            let normal = (-edge_dir.1, edge_dir.0);
            (
                (
                    projection.0 - normal.0 * width,
                    projection.1 - normal.1 * width,
                ),
                (
                    projection.0 + normal.0 * width,
                    projection.1 + normal.1 * width,
                ),
            )
        } else {
            (p0, p1)
        };
        // Betroffene Shapes vorab bestimmen (Index + Teilstücke), dann anwenden.
        type Cut = (usize, Vec<(Vec<Pt>, bool)>);
        let mut cuts: Vec<Cut> = Vec::new();
        for (i, s) in self.shapes.iter().enumerate() {
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
            if let Some(pieces) = bridge_line(&pts, closed, p0, p1, width) {
                cuts.push((i, pieces));
            }
        }
        if cuts.is_empty() {
            return false;
        }
        self.push_undo();
        // Von hinten anwenden, damit die Indizes gültig bleiben.
        cuts.sort_by_key(|c| std::cmp::Reverse(c.0));
        self.selected.clear();
        for (idx, pieces) in cuts {
            let layer_id = self.shapes[idx].layer_id;
            let group_id = self.shapes[idx].group_id;
            self.shapes.remove(idx);
            for (piece, closed) in pieces {
                let i = self.shapes.len();
                let mut sh =
                    crate::model::Shape::new(layer_id, Geo::Polyline { pts: piece, closed });
                sh.group_id = group_id;
                self.shapes.push(sh);
                self.selected.push(i);
            }
        }
        self.dirty = true;
        true
    }

    /// Verrundet NUR die angegebenen Ecken einer Shape (Punkt-Indizes der
    /// Kontur; Referenz-UX: Ecken anklicken). Ein Undo-Punkt.
    pub fn fillet_shape_corners(&mut self, idx: usize, corners: &[usize], radius: f64) {
        if radius <= 0.0 || corners.is_empty() {
            return;
        }
        let Some(s) = self.shapes.get(idx) else {
            return;
        };
        if matches!(s.geo, Geo::Image { .. } | Geo::Ellipse { .. }) {
            return;
        }
        let (mut pts, closed) = s.geo.outline_points();
        if pts.len() < 3 {
            return;
        }
        let rotation = s.rotation;
        let center = s.bbox().center();
        if rotation != 0.0 {
            for p in pts.iter_mut() {
                *p = rotate_point(p.0, p.1, center.0, center.1, rotation);
            }
        }
        self.push_undo();
        let rounded = fillet_corners(&pts, closed, radius, Some(corners));
        if let Some(s) = self.shapes.get_mut(idx) {
            s.rotation = 0.0;
            s.geo = Geo::Polyline {
                pts: rounded,
                closed,
            };
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
