//! Anordnen: Ausrichten und Verteilen der Auswahl. Reine Core-Logik.
//!
//! Ausrichten wirkt bei einer Form relativ zum Bett und bei mehreren Formen
//! relativ zur gemeinsamen Auswahl. Verteilen braucht mindestens drei Formen.

use crate::geometry::{Axis, BBox};
use crate::state::AppState;

/// Ausricht-Art.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    HCenter,
    Right,
    Top,
    VCenter,
    Bottom,
    Center,
}

/// Verteil-Art (gleiche Abstände der Startkanten).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Distribute {
    Horizontal,
    Vertical,
    SpaceHorizontal,
    SpaceVertical,
}

impl AppState {
    pub fn can_align(&self) -> bool {
        !self.selected.is_empty()
    }
    pub fn can_distribute(&self) -> bool {
        self.arrange_units().len() >= 3
    }

    /// Gruppierte Shapes zählen beim Anordnen als eine unteilbare Einheit.
    fn arrange_units(&self) -> Vec<Vec<usize>> {
        let mut units: Vec<Vec<usize>> = Vec::new();
        for &idx in &self.selected {
            let Some(shape) = self.shapes.get(idx) else {
                continue;
            };
            if let Some(gid) = shape.group_id {
                if let Some(unit) = units.iter_mut().find(|unit| {
                    unit.first().is_some_and(|&i| {
                        self.shapes.get(i).is_some_and(|s| s.group_id == Some(gid))
                    })
                }) {
                    unit.push(idx);
                } else {
                    units.push(vec![idx]);
                }
            } else {
                units.push(vec![idx]);
            }
        }
        units
    }

    fn unit_bbox(&self, unit: &[usize]) -> Option<BBox> {
        BBox::union_all(
            unit.iter()
                .filter_map(|&i| self.shapes.get(i))
                .map(|s| s.bbox()),
        )
    }

    /// Richtet die Auswahl an der gemeinsamen Kante/Mitte aus (ein Undo-Punkt).
    pub fn align_selection(&mut self, kind: Align) {
        if !self.can_align() {
            return;
        }
        let units = self.arrange_units();
        let Some(selection) = self.selection_bbox() else {
            return;
        };
        // Eine einzelne Einheit wird wie in der Referenz am Bett ausgerichtet;
        // mehrere Formen richten sich an ihrer gemeinsamen Auswahlbox aus.
        let g = if units.len() == 1 {
            BBox::new(0.0, 0.0, self.bed_w_mm, self.bed_h_mm)
        } else {
            selection
        };
        self.push_undo();
        for unit in units {
            let Some(b) = self.unit_bbox(&unit) else {
                continue;
            };
            let (dx, dy) = align_delta(kind, &g, &b);
            for idx in unit {
                if let Some(s) = self.shapes.get_mut(idx) {
                    s.translate(dx, dy);
                }
            }
        }
        self.dirty = true;
    }

    /// Spiegelbar, sobald mindestens eine Form selektiert ist.
    pub fn can_mirror(&self) -> bool {
        !self.selected.is_empty()
    }

    /// Spiegelt die Auswahl an der Mittelachse ihrer gemeinsamen Bounding-Box
    /// (ein Undo-Punkt). `Axis::Vertical` klappt links↔rechts, `Axis::Horizontal`
    /// oben↔unten. Bei mehreren Formen spiegeln auch die Lagen zueinander.
    pub fn mirror_selection(&mut self, axis: Axis) {
        if !self.can_mirror() {
            return;
        }
        let Some(g) = self.selection_bbox() else {
            return;
        };
        let coord = match axis {
            Axis::Vertical => g.x + g.w / 2.0,
            Axis::Horizontal => g.y + g.h / 2.0,
        };
        self.push_undo();
        let sel = self.selected.clone();
        for idx in sel {
            if let Some(s) = self.shapes.get_mut(idx) {
                s.mirror(axis, coord);
            }
        }
        self.dirty = true;
    }

    /// Verteilt die Auswahl mit gleichen Startkanten-Abständen (ein Undo-Punkt).
    pub fn distribute_selection(&mut self, kind: Distribute) {
        if !self.can_distribute() {
            return;
        }
        self.push_undo();

        let mut items: Vec<(Vec<usize>, BBox)> = self
            .arrange_units()
            .into_iter()
            .filter_map(|unit| self.unit_bbox(&unit).map(|b| (unit, b)))
            .collect();
        items.sort_by(|a, b| {
            distribute_pos(kind, &a.1)
                .partial_cmp(&distribute_pos(kind, &b.1))
                .unwrap()
        });

        let n = items.len();
        let first = distribute_pos(kind, &items[0].1);
        let last = distribute_pos(kind, &items[n - 1].1);
        let total_size: f64 = items.iter().map(|(_, b)| distribute_size(kind, b)).sum();
        let gap = match kind {
            Distribute::SpaceHorizontal | Distribute::SpaceVertical => {
                let outer =
                    distribute_end(kind, &items[n - 1].1) - distribute_start(kind, &items[0].1);
                (outer - total_size) / (n as f64 - 1.0)
            }
            _ => 0.0,
        };
        let step = (last - first) / (n as f64 - 1.0);
        let mut cursor = distribute_start(kind, &items[0].1);

        for (k, (unit, b)) in items.iter().enumerate() {
            if k == 0 || k == n - 1 {
                cursor += distribute_size(kind, b) + gap;
                continue; // Ränder bleiben stehen
            }
            let target = match kind {
                Distribute::SpaceHorizontal | Distribute::SpaceVertical => cursor,
                _ => first + step * k as f64,
            };
            let current = match kind {
                Distribute::SpaceHorizontal | Distribute::SpaceVertical => {
                    distribute_start(kind, b)
                }
                _ => distribute_pos(kind, b),
            };
            let delta = target - current;
            for &idx in unit {
                if let Some(s) = self.shapes.get_mut(idx) {
                    match kind {
                        Distribute::Horizontal | Distribute::SpaceHorizontal => {
                            s.translate(delta, 0.0)
                        }
                        Distribute::Vertical | Distribute::SpaceVertical => s.translate(0.0, delta),
                    }
                }
            }
            cursor += distribute_size(kind, b) + gap;
        }
        self.dirty = true;
    }
}

fn distribute_pos(kind: Distribute, b: &BBox) -> f64 {
    match kind {
        Distribute::Horizontal => b.x + b.w / 2.0,
        Distribute::Vertical => b.y + b.h / 2.0,
        Distribute::SpaceHorizontal => b.x,
        Distribute::SpaceVertical => b.y,
    }
}
fn distribute_start(kind: Distribute, b: &BBox) -> f64 {
    match kind {
        Distribute::Horizontal | Distribute::SpaceHorizontal => b.x,
        _ => b.y,
    }
}
fn distribute_end(kind: Distribute, b: &BBox) -> f64 {
    distribute_start(kind, b) + distribute_size(kind, b)
}
fn distribute_size(kind: Distribute, b: &BBox) -> f64 {
    match kind {
        Distribute::Horizontal | Distribute::SpaceHorizontal => b.w,
        _ => b.h,
    }
}

fn align_delta(kind: Align, g: &BBox, b: &BBox) -> (f64, f64) {
    match kind {
        Align::Left => (g.x - b.x, 0.0),
        Align::Right => (g.x + g.w - (b.x + b.w), 0.0),
        Align::HCenter => (g.x + g.w / 2.0 - (b.x + b.w / 2.0), 0.0),
        Align::Top => (0.0, g.y - b.y),
        Align::Bottom => (0.0, g.y + g.h - (b.y + b.h)),
        Align::VCenter => (0.0, g.y + g.h / 2.0 - (b.y + b.h / 2.0)),
        Align::Center => (
            g.x + g.w / 2.0 - (b.x + b.w / 2.0),
            g.y + g.h / 2.0 - (b.y + b.h / 2.0),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Geo;

    fn rect(x: f64, y: f64, w: f64, h: f64) -> Geo {
        Geo::Rect { x, y, w, h }
    }

    #[test]
    fn align_left_richtet_an_gemeinsamer_kante() {
        let mut s = AppState::new();
        s.add_shape(rect(10.0, 0.0, 20.0, 10.0));
        s.add_shape(rect(50.0, 30.0, 20.0, 10.0));
        s.selected = vec![0, 1];
        s.align_selection(Align::Left);
        assert_eq!(s.shapes[0].bbox().x, 10.0);
        assert_eq!(s.shapes[1].bbox().x, 10.0);
    }

    #[test]
    fn align_hcenter_zentriert() {
        let mut s = AppState::new();
        s.add_shape(rect(0.0, 0.0, 20.0, 10.0)); // Mitte 10
        s.add_shape(rect(80.0, 0.0, 20.0, 10.0)); // Mitte 90
        s.selected = vec![0, 1];
        s.align_selection(Align::HCenter);
        // Gruppenmitte 50 → beide Mitten auf 50.
        assert!((s.shapes[0].bbox().center().0 - 50.0).abs() < 1e-9);
        assert!((s.shapes[1].bbox().center().0 - 50.0).abs() < 1e-9);
    }

    #[test]
    fn distribute_horizontal_verteilt_mitte() {
        let mut s = AppState::new();
        s.add_shape(rect(0.0, 0.0, 5.0, 5.0)); // Start 0
        s.add_shape(rect(10.0, 0.0, 5.0, 5.0)); // Start 10 → soll 45
        s.add_shape(rect(90.0, 0.0, 5.0, 5.0)); // Start 90
        s.selected = vec![0, 1, 2];
        s.distribute_selection(Distribute::Horizontal);
        assert!((s.shapes[1].bbox().x - 45.0).abs() < 1e-9);
        assert_eq!(s.shapes[0].bbox().x, 0.0);
        assert_eq!(s.shapes[2].bbox().x, 90.0);
    }

    #[test]
    fn mirror_vertikal_klappt_gruppe_um_ihre_mitte() {
        let mut s = AppState::new();
        s.add_shape(rect(0.0, 0.0, 10.0, 10.0)); // links
        s.add_shape(rect(90.0, 0.0, 10.0, 10.0)); // rechts
        s.selected = vec![0, 1];
        // Gruppen-BBox 0..100, Achse x=50. Formen tauschen die Seite.
        s.mirror_selection(Axis::Vertical);
        assert_eq!(s.shapes[0].bbox().x, 90.0);
        assert_eq!(s.shapes[1].bbox().x, 0.0);
    }

    #[test]
    fn mirror_einzeln_asymmetrisch_spiegelt_form() {
        let mut s = AppState::new();
        s.add_shape(Geo::Polyline {
            pts: vec![(0.0, 0.0), (10.0, 0.0), (10.0, 5.0)],
            closed: true,
        });
        s.selected = vec![0];
        // BBox 0..10 in x → Achse x=5.
        s.mirror_selection(Axis::Vertical);
        if let Geo::Polyline { pts, .. } = &s.shapes[0].geo {
            assert_eq!(pts[0], (10.0, 0.0));
            assert_eq!(pts[1], (0.0, 0.0));
            assert_eq!(pts[2], (0.0, 5.0));
        } else {
            panic!("kein Polyline");
        }
    }

    #[test]
    fn mirror_ohne_auswahl_ist_noop() {
        let mut s = AppState::new();
        s.add_shape(rect(0.0, 0.0, 10.0, 10.0));
        s.selected.clear();
        assert!(!s.can_mirror());
        s.mirror_selection(Axis::Horizontal); // no-op
        assert_eq!(s.shapes[0].bbox().y, 0.0);
    }

    #[test]
    fn einzelne_auswahl_richtet_sich_am_bett_aus() {
        let mut s = AppState::new();
        s.add_shape(rect(0.0, 0.0, 5.0, 5.0));
        s.selected = vec![0];
        assert!(s.can_align());
        s.align_selection(Align::Right);
        assert_eq!(s.shapes[0].bbox().x, s.bed_w_mm - 5.0);
    }

    #[test]
    fn verteilen_nach_mitten_beruecksichtigt_unterschiedliche_breiten() {
        let mut s = AppState::new();
        s.add_shape(rect(0.0, 0.0, 10.0, 5.0)); // Mitte 5
        s.add_shape(rect(20.0, 0.0, 30.0, 5.0)); // Mitte 35
        s.add_shape(rect(90.0, 0.0, 10.0, 5.0)); // Mitte 95
        s.selected = vec![0, 1, 2];
        s.distribute_selection(Distribute::Horizontal);
        assert!((s.shapes[1].bbox().center().0 - 50.0).abs() < 1e-9);
    }

    #[test]
    fn verteilen_gleicher_zwischenraeume() {
        let mut s = AppState::new();
        s.add_shape(rect(0.0, 0.0, 10.0, 5.0));
        s.add_shape(rect(20.0, 0.0, 30.0, 5.0));
        s.add_shape(rect(90.0, 0.0, 10.0, 5.0));
        s.selected = vec![0, 1, 2];
        s.distribute_selection(Distribute::SpaceHorizontal);
        assert!((s.shapes[1].bbox().x - 35.0).abs() < 1e-9);
    }

    #[test]
    fn ausrichten_bewegt_gruppe_als_unteilbare_einheit() {
        let mut s = AppState::new();
        s.add_shape(rect(10.0, 10.0, 5.0, 5.0));
        s.add_shape(rect(20.0, 10.0, 5.0, 5.0));
        s.add_shape(rect(50.0, 30.0, 5.0, 5.0));
        s.shapes[0].group_id = Some(1);
        s.shapes[1].group_id = Some(1);
        s.selected = vec![0, 1, 2];
        let internal_gap = s.shapes[1].bbox().x - s.shapes[0].bbox().x;
        s.align_selection(Align::Left);
        assert_eq!(s.shapes[1].bbox().x - s.shapes[0].bbox().x, internal_gap);
        assert_eq!(s.shapes[0].bbox().x, s.shapes[2].bbox().x);
    }

    #[test]
    fn eine_gruppe_aus_drei_shapes_ist_nicht_verteilbar() {
        let mut s = AppState::new();
        for x in [0.0, 10.0, 20.0] {
            s.add_shape(rect(x, 0.0, 5.0, 5.0));
        }
        for shape in &mut s.shapes {
            shape.group_id = Some(1);
        }
        s.selected = vec![0, 1, 2];
        assert!(!s.can_distribute());
    }
}
