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

/// Read-only Jobpfad: Arbeitsbewegungen nach Layerfarbe, Leerfahrten dezent
/// gestrichelt. Grundlage ist ausschließlich die Application-Preview.
pub fn preview_vertices(session: &EditorSession, selection_only: bool) -> BaseGeometry {
    let mut v = scene_geo::bed_grid(session.bed_w_mm as f32, session.bed_h_mm as f32);
    let background_end = v.len() as u32;
    let preview = session.job_preview(selection_only);
    for movement in preview.moves {
        let color = match movement.kind {
            luxifer_core::preview::MoveKind::Travel => [0.55, 0.6, 0.68, 0.45],
            luxifer_core::preview::MoveKind::Fill => [1.0, 0.58, 0.2, 0.9],
            luxifer_core::preview::MoveKind::Raster => [0.75, 0.75, 0.75, 0.9],
            luxifer_core::preview::MoveKind::Cut => session
                .layers
                .get(movement.layer_id)
                .map(|l| {
                    [
                        l.color[0] as f32 / 255.0,
                        l.color[1] as f32 / 255.0,
                        l.color[2] as f32 / 255.0,
                        1.0,
                    ]
                })
                .unwrap_or([0.9, 0.2, 0.2, 1.0]),
        };
        scene_geo::push_seg(
            &mut v,
            [movement.from.0 as f32, movement.from.1 as f32],
            [movement.to.0 as f32, movement.to.1 as f32],
            color,
        );
    }
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
