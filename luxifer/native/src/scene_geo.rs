//! Wandelt den Core-`AppState` in Zeichendaten (Linien-Vertices) für den
//! wgpu-Canvas. Reines Zeichnen — keine Fachlogik. Farben kommen aus den Layern,
//! Rotation wird wie im Core (`rotate_point`) angewendet.

use luxifer_core::geometry::rotate_point;
use luxifer_core::state::AppState;

/// Ein Vertex im Welt-Raum (mm) mit Farbe. Die Projektion nach NDC macht der
/// Vertex-Shader anhand der Kamera-Uniforms.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 2],
    pub color: [f32; 4],
}

fn col(rgb: [u8; 3], a: f32) -> [f32; 4] {
    [
        rgb[0] as f32 / 255.0,
        rgb[1] as f32 / 255.0,
        rgb[2] as f32 / 255.0,
        a,
    ]
}

/// Weltpunkte einer Form inkl. Rotation (um den BBox-Mittelpunkt), wie der Core
/// sie für Hit-Test/BBox verwendet.
fn world_outline(shape: &luxifer_core::model::Shape) -> (Vec<(f64, f64)>, bool) {
    let (pts, closed) = shape.geo.outline_points();
    if shape.rotation.abs() <= f64::EPSILON {
        return (pts, closed);
    }
    let c = shape.geo.bbox().center();
    let rot = pts
        .into_iter()
        .map(|(x, y)| rotate_point(x, y, c.0, c.1, shape.rotation))
        .collect();
    (rot, closed)
}

/// Baut die Linien-Vertices (LineList) für alle sichtbaren Shapes. Selektierte
/// Shapes bekommen die Akzentfarbe, sonst die Layer-Farbe.
pub fn shape_lines(state: &AppState, accent: [u8; 3]) -> Vec<Vertex> {
    let mut v = Vec::new();
    for (i, shape) in state.shapes.iter().enumerate() {
        let layer = state.layers.get(shape.layer_id);
        let visible = layer.map(|l| l.visible).unwrap_or(true);
        if !visible {
            continue;
        }
        let selected = state.selected.contains(&i);
        let base = layer.map(|l| l.color).unwrap_or([200, 200, 200]);
        let color = if selected {
            col(accent, 1.0)
        } else {
            col(base, 1.0)
        };

        let (pts, closed) = world_outline(shape);
        push_polyline(&mut v, &pts, closed, color);
    }
    v
}

fn push_polyline(v: &mut Vec<Vertex>, pts: &[(f64, f64)], closed: bool, color: [f32; 4]) {
    if pts.len() < 2 {
        return;
    }
    for w in pts.windows(2) {
        v.push(Vertex {
            pos: [w[0].0 as f32, w[0].1 as f32],
            color,
        });
        v.push(Vertex {
            pos: [w[1].0 as f32, w[1].1 as f32],
            color,
        });
    }
    if closed {
        let (a, b) = (pts[pts.len() - 1], pts[0]);
        v.push(Vertex {
            pos: [a.0 as f32, a.1 as f32],
            color,
        });
        v.push(Vertex {
            pos: [b.0 as f32, b.1 as f32],
            color,
        });
    }
}

/// Rechteck-Umriss (Welt) als Linien — für Tisch-Rahmen und Auswahl-BBox.
pub fn rect_outline(x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) -> Vec<Vertex> {
    let p = [[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
    let mut v = Vec::new();
    for i in 0..4 {
        let a = p[i];
        let b = p[(i + 1) % 4];
        v.push(Vertex { pos: a, color });
        v.push(Vertex { pos: b, color });
    }
    v
}

/// Farbwert für den Tisch-Rahmen (dezentes Grau).
pub const BED_COLOR: [f32; 4] = [0.35, 0.38, 0.42, 1.0];
/// Auswahl-BBox-Rahmen (heller Akzentton).
pub const SEL_BOX_COLOR: [f32; 4] = [0.4, 0.7, 1.0, 0.9];
