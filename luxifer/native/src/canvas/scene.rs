//! Gecachter Basis-Vertexpuffer: Tisch-Gitter, Füllung und Konturen. Hängt nur
//! an der Geometrie (über die Render-Revision invalidiert), nicht an der Auswahl
//! — die Auswahl-Akzentuierung liegt bewusst im Overlay.

use luxifer_application::EditorSession;

use crate::scene_geo::{self, Vertex};

pub struct BaseGeometry {
    pub vertices: Vec<Vertex>,
    /// Ende des Bett-/Gitter-Bereichs im gemeinsamen Vertexpuffer.
    pub background_end: u32,
}

/// Baut die gecachten Zeichendaten (Tisch-Gitter, Shapes-Füllung/Kontur).
pub fn base_vertices(session: &EditorSession) -> BaseGeometry {
    let mut v = scene_geo::bed_grid(session.bed_w_mm as f32, session.bed_h_mm as f32);
    let background_end = v.len() as u32;
    // Füllung zuerst (liegt unter den Konturen), dann die Umrisse.
    v.extend(scene_geo::fill_lines(session));
    v.extend(scene_geo::shape_lines(session));
    // Der laufende Punkt-Zug (Polyline/Spline/Bézier/Polygon) wird im Overlay
    // gezeichnet (jeden Frame, damit das Gummiband der Maus folgt).
    BaseGeometry {
        vertices: v,
        background_end,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bett_und_szenengeometrie_haben_getrennte_renderbereiche() {
        let mut session = EditorSession::default();
        session
            .state_mut_for_migration()
            .add_image("asset".into(), 0.0, 0.0, 20.0, 10.0);

        let geometry = base_vertices(&session);

        assert!(geometry.background_end > 0);
        assert!((geometry.background_end as usize) < geometry.vertices.len());
    }
}
