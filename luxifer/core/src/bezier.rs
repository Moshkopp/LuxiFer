//! Editierbare Bézier-Pfade: Knoten (Anker + Tangenten) → geflattete Polylinie.
//!
//! Reine Geometrie (UI-frei, testbar). Nach v3-Analyse neu gebaut
//! (`shapes.rs`: `BezierNode`/`bezier_path`/`split_bezier_segment`), nicht
//! kopiert. Ein Bézier-Pfad kettet kubische Segmente: das Kontrollpolygon
//! zwischen Knoten `i` und `i+1` ist `[p_i, h_out_i, h_in_{i+1}, p_{i+1}]`.
//! Fehlt ein Handle, wird der Anker genommen → gerade Strecke.
//!
//! **Einbettung in v5** (bewusst, ADR-Muster wie `TextMeta`): ein Bézier lebt
//! NICHT als eigener `Geo`-Typ, sondern als **Metadatum** (`BezierPath`) an
//! einer normalen `Geo::Polyline`. Die Polyline trägt die geflatteten Punkte —
//! Job, Preview, Treiber und Canvas-Zeichnen bleiben unverändert. Nur der
//! Node-Editor kennt die Knoten und schreibt bei jeder Bearbeitung die
//! Polyline neu. So bleibt das Kern-Enum schlank.

use crate::geometry::Pt;
use serde::{Deserialize, Serialize};

/// Ein Bézier-Knoten: Ankerpunkt + optionale Tangenten-Endpunkte (absolute
/// mm-Koordinaten). `None` = harte Ecke ohne Tangente an dieser Seite.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BezierNode {
    pub p: Pt,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h_in: Option<Pt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h_out: Option<Pt>,
}

impl BezierNode {
    /// Knoten ohne Tangenten (Ecke).
    pub fn corner(p: Pt) -> Self {
        Self {
            p,
            h_in: None,
            h_out: None,
        }
    }
}

/// Editierbarer Bézier-Pfad (Metadatum an einer Polyline).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BezierPath {
    pub nodes: Vec<BezierNode>,
    pub closed: bool,
}

/// Standard-Flatten-Toleranz in mm (feiner als sichtbar, gröber als teuer).
const TOL: f64 = 0.05;

impl BezierPath {
    /// Wandelt die Knoten in die geflattete Polylinie (mm) — das ist, was
    /// gezeichnet und gelasert wird.
    pub fn flatten(&self) -> Vec<Pt> {
        bezier_path(&self.nodes, self.closed, TOL)
    }
}

/// Kettet die Knoten zu kubischen Segmenten und flacht adaptiv ab.
pub fn bezier_path(nodes: &[BezierNode], closed: bool, tol: f64) -> Vec<Pt> {
    if nodes.len() < 2 {
        return nodes.iter().map(|n| n.p).collect();
    }
    let mut out: Vec<Pt> = vec![nodes[0].p];
    let seg = |a: &BezierNode, b: &BezierNode, out: &mut Vec<Pt>| {
        let p0 = a.p;
        let p1 = a.h_out.unwrap_or(a.p);
        let p2 = b.h_in.unwrap_or(b.p);
        let p3 = b.p;
        out.extend(flatten_cubic(p0, p1, p2, p3, tol));
    };
    for i in 0..nodes.len() - 1 {
        seg(&nodes[i], &nodes[i + 1], &mut out);
    }
    if closed {
        seg(&nodes[nodes.len() - 1], &nodes[0], &mut out);
    }
    out
}

/// Ergebnis einer formerhaltenden Segmentteilung (De-Casteljau bei `t`).
#[derive(Debug, Clone, Copy)]
pub struct BezierSplit {
    pub left_h_out: Pt,
    pub mid: BezierNode,
    pub right_h_in: Pt,
}

/// Teilt das Segment zwischen `a` und `b` bei `t ∈ (0,1)` formerhaltend.
pub fn split_bezier_segment(a: &BezierNode, b: &BezierNode, t: f64) -> BezierSplit {
    let p0 = a.p;
    let p1 = a.h_out.unwrap_or(a.p);
    let p2 = b.h_in.unwrap_or(b.p);
    let p3 = b.p;
    let lerp = |u: Pt, v: Pt| (u.0 + (v.0 - u.0) * t, u.1 + (v.1 - u.1) * t);
    let q0 = lerp(p0, p1);
    let q1 = lerp(p1, p2);
    let q2 = lerp(p2, p3);
    let r0 = lerp(q0, q1);
    let r1 = lerp(q1, q2);
    let s = lerp(r0, r1);
    BezierSplit {
        left_h_out: q0,
        mid: BezierNode {
            p: s,
            h_in: Some(r0),
            h_out: Some(r1),
        },
        right_h_in: q2,
    }
}

/// Adaptive Flachschlagung einer kubischen Bézier (De-Casteljau-Rekursion).
/// `tol` = maximale Abweichung in mm.
pub fn flatten_cubic(p0: Pt, p1: Pt, p2: Pt, p3: Pt, tol: f64) -> Vec<Pt> {
    let mut out = Vec::new();
    subdiv(p0, p1, p2, p3, tol, 0, &mut out);
    out.push(p3);
    out
}

fn subdiv(p0: Pt, p1: Pt, p2: Pt, p3: Pt, tol: f64, depth: u32, out: &mut Vec<Pt>) {
    // Ebenheit über den Abstand der Kontrollpunkte p1/p2 zur Sehne p0–p3.
    let flat = seg_dist(p1, p0, p3).max(seg_dist(p2, p0, p3)) <= tol;
    if flat || depth >= 18 {
        return;
    }
    let lerp = |u: Pt, v: Pt| ((u.0 + v.0) * 0.5, (u.1 + v.1) * 0.5);
    let p01 = lerp(p0, p1);
    let p12 = lerp(p1, p2);
    let p23 = lerp(p2, p3);
    let p012 = lerp(p01, p12);
    let p123 = lerp(p12, p23);
    let m = lerp(p012, p123);
    subdiv(p0, p01, p012, m, tol, depth + 1, out);
    out.push(m);
    subdiv(m, p123, p23, p3, tol, depth + 1, out);
}

/// Abstand von `q` zur Strecke a–b.
fn seg_dist(q: Pt, a: Pt, b: Pt) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    if len2 < 1e-18 {
        return (q.0 - a.0).hypot(q.1 - a.1);
    }
    ((q.0 - a.0) * dy - (q.1 - a.1) * dx).abs() / len2.sqrt()
}

/// Baut aus einer Klick-Polylinie (die Zeichen-Punkte) einen glatten
/// Bézier-Pfad: an jedem Innenknoten werden Tangenten in Richtung der
/// Nachbarn gesetzt (Catmull-Rom-artig), sodass die Kurve weich durch alle
/// Punkte läuft. Das ist der „Bézier-Feder"-Zeichenmodus.
pub fn smooth_from_points(pts: &[Pt], closed: bool) -> BezierPath {
    let n = pts.len();
    let mut nodes: Vec<BezierNode> = pts.iter().map(|&p| BezierNode::corner(p)).collect();
    if n < 3 {
        return BezierPath { nodes, closed };
    }
    // Tangentenlänge = 1/6 des Abstands zu den Nachbarn (Catmull-Rom → Bézier).
    let at = |i: i64| -> Pt {
        if closed {
            pts[i.rem_euclid(n as i64) as usize]
        } else {
            pts[i.clamp(0, n as i64 - 1) as usize]
        }
    };
    for i in 0..n {
        let prev = at(i as i64 - 1);
        let next = at(i as i64 + 1);
        let tx = (next.0 - prev.0) / 6.0;
        let ty = (next.1 - prev.1) / 6.0;
        let p = pts[i];
        let is_end = !closed && (i == 0 || i == n - 1);
        if !is_end {
            nodes[i].h_in = Some((p.0 - tx, p.1 - ty));
            nodes[i].h_out = Some((p.0 + tx, p.1 + ty));
        }
    }
    BezierPath { nodes, closed }
}

// ── AppState-Anbindung ───────────────────────────────────────────────────────

use crate::geometry::Geo;
use crate::model::Shape;
use crate::state::AppState;

/// Welcher Teil eines Knotens angefasst wird (Node-Editor).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodePart {
    /// Der Ankerpunkt (verschiebt Anker + beide Handles mit).
    Anchor,
    /// Der eingehende Tangenten-Endpunkt.
    HandleIn,
    /// Der ausgehende Tangenten-Endpunkt.
    HandleOut,
}

impl AppState {
    /// Fügt eine Bézier-Feder aus den geklickten Punkten ein: glatte Kurve
    /// durch alle Punkte, editierbare Knoten als Metadatum. Ein Undo-Punkt.
    pub fn add_bezier(&mut self, pts: Vec<Pt>, closed: bool) -> usize {
        let bp = smooth_from_points(&pts, closed);
        let flat = bp.flatten();
        self.push_undo();
        let layer_id = self.layer_for_new_shape();
        let mut sh = Shape::new(layer_id, Geo::Polyline { pts: flat, closed });
        sh.bezier = Some(bp);
        self.shapes.push(sh);
        let idx = self.shapes.len() - 1;
        self.selected = vec![idx];
        self.pending_color = None;
        idx
    }

    /// Fügt eine Bézier-Feder aus **fertigen Knoten** (Inkscape-Feder-Stil) ein.
    /// Ein Undo-Punkt.
    pub fn add_bezier_nodes(&mut self, nodes: Vec<BezierNode>, closed: bool) -> usize {
        let bp = BezierPath { nodes, closed };
        let flat = bp.flatten();
        self.push_undo();
        let layer_id = self.layer_for_new_shape();
        let mut sh = Shape::new(layer_id, Geo::Polyline { pts: flat, closed });
        sh.bezier = Some(bp);
        self.shapes.push(sh);
        let idx = self.shapes.len() - 1;
        self.selected = vec![idx];
        self.pending_color = None;
        idx
    }

    /// Node-Editor: verschiebt Anker/Handle des Knotens `node` von Shape `idx`
    /// an die neue Weltposition und schreibt die Polyline neu. Ein Undo-Punkt
    /// pro Geste — der Aufrufer ruft `push_undo` einmal beim Drag-Beginn.
    ///
    /// Für Shapes OHNE Bézier-Meta (normale Polyline/Rect) wird beim ersten
    /// Anfassen automatisch ein Eckknoten-Pfad erzeugt, sodass jede Kontur
    /// per Knoten editierbar ist (v3-Verhalten: Polylinien-Punkte ziehen).
    pub fn drag_node(&mut self, idx: usize, node: usize, part: NodePart, to: Pt) {
        self.ensure_bezier(idx);
        let Some(sh) = self.shapes.get_mut(idx) else {
            return;
        };
        let Some(bp) = sh.bezier.as_mut() else { return };
        let Some(n) = bp.nodes.get_mut(node) else {
            return;
        };
        match part {
            NodePart::Anchor => {
                let (dx, dy) = (to.0 - n.p.0, to.1 - n.p.1);
                n.p = to;
                if let Some(h) = n.h_in.as_mut() {
                    *h = (h.0 + dx, h.1 + dy);
                }
                if let Some(h) = n.h_out.as_mut() {
                    *h = (h.0 + dx, h.1 + dy);
                }
            }
            NodePart::HandleIn => {
                n.h_in = Some(to);
                if n.h_out.is_some() {
                    n.h_out = Some((2.0 * n.p.0 - to.0, 2.0 * n.p.1 - to.1));
                }
            }
            NodePart::HandleOut => {
                n.h_out = Some(to);
                if n.h_in.is_some() {
                    n.h_in = Some((2.0 * n.p.0 - to.0, 2.0 * n.p.1 - to.1));
                }
            }
        }
        let (flat, closed) = (bp.flatten(), bp.closed);
        sh.geo = Geo::Polyline { pts: flat, closed };
        self.dirty = true;
    }

    /// Teilt das Segment vor Knoten `node` (fügt einen Mittelknoten ein).
    /// Ein Undo-Punkt.
    pub fn split_node_segment(&mut self, idx: usize, seg_start: usize, t: f64) {
        self.ensure_bezier(idx);
        let Some(sh) = self.shapes.get_mut(idx) else {
            return;
        };
        let Some(bp) = sh.bezier.as_mut() else { return };
        let n = bp.nodes.len();
        if n < 2 {
            return;
        }
        let (i, j) = (seg_start, (seg_start + 1) % n);
        if !bp.closed && j <= i {
            return;
        }
        self.dirty = true; // Undo hat der Aufrufer gesetzt
        let split = split_bezier_segment(&bp.nodes[i], &bp.nodes[j], t.clamp(0.001, 0.999));
        bp.nodes[i].h_out = Some(split.left_h_out);
        bp.nodes[j].h_in = Some(split.right_h_in);
        bp.nodes.insert(i + 1, split.mid);
        let (flat, closed) = (bp.flatten(), bp.closed);
        sh.geo = Geo::Polyline { pts: flat, closed };
    }

    /// Löscht einen Knoten. Ein Undo-Punkt.
    pub fn delete_node(&mut self, idx: usize, node: usize) {
        let Some(sh) = self.shapes.get_mut(idx) else {
            return;
        };
        let Some(bp) = sh.bezier.as_mut() else { return };
        if bp.nodes.len() <= 2 || node >= bp.nodes.len() {
            return;
        }
        bp.nodes.remove(node);
        let (flat, closed) = (bp.flatten(), bp.closed);
        sh.geo = Geo::Polyline { pts: flat, closed };
        self.dirty = true;
    }

    /// Schaltet einen Knoten zwischen Ecke und glattem, symmetrischem Knoten um.
    pub fn toggle_node_smooth(&mut self, idx: usize, node: usize) {
        self.ensure_bezier(idx);
        let Some(sh) = self.shapes.get_mut(idx) else {
            return;
        };
        let Some(bp) = sh.bezier.as_mut() else { return };
        if node >= bp.nodes.len() {
            return;
        }
        if bp.nodes[node].h_in.is_some() || bp.nodes[node].h_out.is_some() {
            bp.nodes[node].h_in = None;
            bp.nodes[node].h_out = None;
        } else {
            let count = bp.nodes.len();
            if count < 2 {
                return;
            }
            let p = bp.nodes[node].p;
            let prev = if node > 0 {
                Some(bp.nodes[node - 1].p)
            } else if bp.closed {
                Some(bp.nodes[count - 1].p)
            } else {
                None
            };
            let next = if node + 1 < count {
                Some(bp.nodes[node + 1].p)
            } else if bp.closed {
                Some(bp.nodes[0].p)
            } else {
                None
            };
            let (dx, dy, length) = match (prev, next) {
                (Some(a), Some(b)) => (
                    b.0 - a.0,
                    b.1 - a.1,
                    ((p.0 - a.0).hypot(p.1 - a.1) + (b.0 - p.0).hypot(b.1 - p.1)) / 6.0,
                ),
                (None, Some(b)) => (b.0 - p.0, b.1 - p.1, (b.0 - p.0).hypot(b.1 - p.1) / 3.0),
                (Some(a), None) => (p.0 - a.0, p.1 - a.1, (p.0 - a.0).hypot(p.1 - a.1) / 3.0),
                _ => return,
            };
            let norm = dx.hypot(dy);
            if norm <= f64::EPSILON {
                return;
            }
            let (vx, vy) = (dx / norm * length, dy / norm * length);
            bp.nodes[node].h_in = Some((p.0 - vx, p.1 - vy));
            bp.nodes[node].h_out = Some((p.0 + vx, p.1 + vy));
        }
        let (flat, closed) = (bp.flatten(), bp.closed);
        sh.geo = Geo::Polyline { pts: flat, closed };
        self.dirty = true;
    }

    /// Stellt sicher, dass Shape `idx` ein Bézier-Metadatum hat: fehlt es (bei
    /// einer normalen Polyline oder einem Rechteck), werden die Konturpunkte
    /// als Eckknoten übernommen — so ist jede Vektor-Kontur node-editierbar.
    fn ensure_bezier(&mut self, idx: usize) {
        let Some(sh) = self.shapes.get_mut(idx) else {
            return;
        };
        if sh.bezier.is_some() || matches!(sh.geo, Geo::Image { .. } | Geo::Ellipse { .. }) {
            return;
        }
        let (pts, closed) = sh.geo.outline_points();
        if pts.len() < 2 {
            return;
        }
        sh.bezier = Some(BezierPath {
            nodes: pts.into_iter().map(BezierNode::corner).collect(),
            closed,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gerade_ohne_handles_bleibt_gerade() {
        let nodes = vec![
            BezierNode::corner((0.0, 0.0)),
            BezierNode::corner((10.0, 0.0)),
        ];
        let path = bezier_path(&nodes, false, TOL);
        assert_eq!(path.first(), Some(&(0.0, 0.0)));
        assert_eq!(path.last(), Some(&(10.0, 0.0)));
        assert!(path.iter().all(|&(_, y)| y.abs() < 1e-9), "gerade Linie");
    }

    #[test]
    fn kurve_mit_handles_beult_aus() {
        // Symmetrische Handles nach oben → Kurvenmitte liegt über der Sehne.
        let a = BezierNode {
            p: (0.0, 0.0),
            h_in: None,
            h_out: Some((3.0, 5.0)),
        };
        let b = BezierNode {
            p: (10.0, 0.0),
            h_in: Some((7.0, 5.0)),
            h_out: None,
        };
        let path = bezier_path(&[a, b], false, TOL);
        let max_y = path.iter().map(|p| p.1).fold(f64::MIN, f64::max);
        assert!(max_y > 2.0, "Kurve beult aus, max_y war {max_y}");
    }

    #[test]
    fn split_erhaelt_die_kurvenform() {
        let a = BezierNode {
            p: (0.0, 0.0),
            h_in: None,
            h_out: Some((3.0, 6.0)),
        };
        let b = BezierNode {
            p: (10.0, 0.0),
            h_in: Some((7.0, 6.0)),
            h_out: None,
        };
        let before = bezier_path(&[a, b], false, 0.01);
        let sp = split_bezier_segment(&a, &b, 0.5);
        let a2 = BezierNode {
            h_out: Some(sp.left_h_out),
            ..a
        };
        let b2 = BezierNode {
            h_in: Some(sp.right_h_in),
            ..b
        };
        let after = bezier_path(&[a2, sp.mid, b2], false, 0.01);
        // Der geteilte Pfad muss die alte Kurve treffen (Stichprobe: Mittelpunkt).
        let mid_before = before[before.len() / 2];
        let closest = after
            .iter()
            .map(|&q| (q.0 - mid_before.0).hypot(q.1 - mid_before.1))
            .fold(f64::MAX, f64::min);
        assert!(closest < 0.1, "geteilte Kurve weicht ab ({closest})");
    }

    #[test]
    fn smooth_laeuft_durch_alle_punkte() {
        let pts = vec![(0.0, 0.0), (10.0, 10.0), (20.0, 0.0), (30.0, 10.0)];
        let bp = smooth_from_points(&pts, false);
        let path = bp.flatten();
        // Jeder Original-Punkt liegt (fast) auf der geflatteten Kurve.
        for &q in &pts {
            let d = path
                .iter()
                .map(|&r| (r.0 - q.0).hypot(r.1 - q.1))
                .fold(f64::MAX, f64::min);
            assert!(d < 0.2, "Punkt {q:?} nicht auf der Kurve ({d})");
        }
        // Endpunkte sind Ecken (keine Tangenten) bei offener Kurve.
        assert!(bp.nodes[0].h_in.is_none() && bp.nodes[0].h_out.is_none());
    }

    #[test]
    fn bezier_zeichnen_und_node_ziehen() {
        use crate::state::AppState;
        let mut app = AppState::new();
        let idx = app.add_bezier(vec![(0.0, 0.0), (10.0, 10.0), (20.0, 0.0)], false);
        // Shape ist eine Polyline MIT Bézier-Meta.
        assert!(app.shapes[idx].bezier.is_some());
        assert!(matches!(app.shapes[idx].geo, Geo::Polyline { .. }));
        // Ankerpunkt des mittleren Knotens ziehen → Kurve ändert sich.
        let before = match &app.shapes[idx].geo {
            Geo::Polyline { pts, .. } => pts.clone(),
            _ => panic!(),
        };
        app.push_undo();
        app.drag_node(idx, 1, NodePart::Anchor, (10.0, 30.0));
        let after = match &app.shapes[idx].geo {
            Geo::Polyline { pts, .. } => pts.clone(),
            _ => panic!(),
        };
        assert_ne!(before, after, "Node-Zug verändert die Kurve");
        // Der mittlere Knoten sitzt jetzt bei y=30.
        assert_eq!(
            app.shapes[idx].bezier.as_ref().unwrap().nodes[1].p,
            (10.0, 30.0)
        );
        // Undo stellt die alte Kurve wieder her.
        app.undo();
        assert_eq!(
            match &app.shapes[idx].geo {
                Geo::Polyline { pts, .. } => pts.clone(),
                _ => panic!(),
            },
            before
        );
    }

    #[test]
    fn normale_polyline_wird_node_editierbar() {
        use crate::state::AppState;
        let mut app = AppState::new();
        let i = app.add_shape(Geo::Polyline {
            pts: vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)],
            closed: false,
        });
        assert!(app.shapes[i].bezier.is_none());
        app.push_undo();
        app.drag_node(i, 1, NodePart::Anchor, (5.0, 5.0));
        // Jetzt hat sie ein Bézier-Meta (Eckknoten) und der Punkt ist verschoben.
        assert!(app.shapes[i].bezier.is_some());
        assert_eq!(
            app.shapes[i].bezier.as_ref().unwrap().nodes[1].p,
            (5.0, 5.0)
        );
    }

    #[test]
    fn segment_teilen_fuegt_knoten_ein() {
        use crate::state::AppState;
        let mut app = AppState::new();
        let i = app.add_bezier(vec![(0.0, 0.0), (10.0, 10.0)], false);
        let n0 = app.shapes[i].bezier.as_ref().unwrap().nodes.len();
        app.push_undo();
        app.split_node_segment(i, 0, 0.5);
        assert_eq!(app.shapes[i].bezier.as_ref().unwrap().nodes.len(), n0 + 1);
    }

    #[test]
    fn shape_transformationen_halten_bezier_meta_synchron() {
        let mut app = AppState::new();
        let i = app.add_bezier_nodes(
            vec![
                BezierNode {
                    p: (0.0, 0.0),
                    h_in: None,
                    h_out: Some((2.0, 3.0)),
                },
                BezierNode {
                    p: (10.0, 10.0),
                    h_in: Some((8.0, 7.0)),
                    h_out: None,
                },
            ],
            false,
        );

        app.shapes[i].translate(5.0, 4.0);
        let bp = app.shapes[i].bezier.as_ref().unwrap();
        assert_eq!(bp.nodes[0].p, (5.0, 4.0));
        assert_eq!(bp.nodes[0].h_out, Some((7.0, 7.0)));

        app.shapes[i].set_bbox(0.0, 0.0, 20.0, 20.0);
        let bp = app.shapes[i].bezier.as_ref().unwrap();
        assert_eq!(bp.nodes[0].p, (0.0, 0.0));
        assert_eq!(bp.nodes[1].p, (20.0, 20.0));
        assert_eq!(bp.nodes[0].h_out, Some((4.0, 6.0)));

        app.shapes[i].mirror(crate::geometry::Axis::Vertical, 10.0);
        let bp = app.shapes[i].bezier.as_ref().unwrap();
        assert_eq!(bp.nodes[0].p, (20.0, 0.0));
        assert_eq!(bp.nodes[1].p, (0.0, 20.0));
        assert_eq!(bp.nodes[0].h_out, Some((16.0, 6.0)));
    }

    #[test]
    fn flatten_toleranz_wirkt() {
        let a = BezierNode {
            p: (0.0, 0.0),
            h_in: None,
            h_out: Some((0.0, 10.0)),
        };
        let b = BezierNode {
            p: (10.0, 0.0),
            h_in: Some((10.0, 10.0)),
            h_out: None,
        };
        let grob = bezier_path(&[a, b], false, 1.0);
        let fein = bezier_path(&[a, b], false, 0.01);
        assert!(fein.len() > grob.len(), "feinere Toleranz = mehr Punkte");
    }
}
