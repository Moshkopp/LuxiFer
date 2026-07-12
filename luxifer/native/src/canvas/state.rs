//! Kurzlebiger Interaktions- und Kamerazustand des Canvas. Reines UI-Anliegen:
//! welches Werkzeug aktiv ist, welche Geste läuft, wo der Cursor steht, wie die
//! Kamera steht. Die Fach-Wahrheit bleibt im Core (`EditorSession`); dieser
//! Zustand steuert nur Darstellung und Eingabe.

use crate::camera::Camera;
use crate::tools::{Drag, Tool};

pub struct CanvasState {
    pub cam: Camera,
    pub tool: Tool,
    /// Aktive Polygon-Form (beim Polygon-Werkzeug aufgezogen).
    pub active_shape: luxifer_core::PolyShape,
    /// Laufende Maus-Geste (zwischen Press und Release).
    pub drag: Drag,
    /// Cursor in Fensterpixeln (für Welt-Umrechnung).
    pub cursor: [f32; 2],
    pub space_down: bool,
    pub ctrl_down: bool,
    pub shift_down: bool,
    /// Punkt-Zug (Welt-Punkte), bis Doppelklick/Enter schließt.
    pub poly_pts: Vec<(f64, f64)>,
}

impl CanvasState {
    pub fn new(cam: Camera) -> Self {
        Self {
            cam,
            tool: Tool::Select,
            active_shape: luxifer_core::PolyShape::Penta,
            drag: Drag::None,
            cursor: [0.0, 0.0],
            space_down: false,
            ctrl_down: false,
            shift_down: false,
            poly_pts: Vec::new(),
        }
    }

    /// Cursor-Weltkoordinaten (mm).
    pub fn world(&self) -> [f64; 2] {
        self.cam.screen_to_world(self.cursor)
    }
}
