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

/// Preview-Farben — geteilt zwischen Szene und Legende, damit beide dieselbe
/// Sprache sprechen.
pub const PREVIEW_TRAVEL: [f32; 4] = [0.55, 0.6, 0.68, 0.45];
pub const PREVIEW_FILL: [f32; 4] = [1.0, 0.58, 0.2, 0.9];
pub const PREVIEW_RASTER: [f32; 4] = [0.75, 0.75, 0.75, 0.9];

/// Kennzahlen und vorkommende Segmentarten der Jobvorschau für die Legende.
/// Wird beim Vertex-Aufbau nebenbei gefüllt (kein zweiter Preview-Lauf).
#[derive(Default)]
pub struct PreviewLegend {
    /// Cut-Layer, die im Job tatsächlich vorkommen (Name, Farbe), in
    /// Brenn-Reihenfolge und ohne Doppelte.
    pub cut_layers: Vec<(String, [u8; 3])>,
    pub has_fill: bool,
    pub has_raster: bool,
    pub has_travel: bool,
    /// Arbeitsweg (Laser an) in mm.
    pub work_len_mm: f64,
    /// Leerfahrten in mm.
    pub travel_len_mm: f64,
    /// Bounding-Box der Job-Geometrie (mm).
    pub bbox: Option<(f64, f64, f64, f64)>,
}

/// Vollständiger Preview-Aufbau: Vertices für die Bewegungen plus die
/// verarbeiteten Rastertexturen der Bild-Layer und die Legende.
pub struct PreviewGeometry {
    pub vertices: Vec<Vertex>,
    pub background_end: u32,
    /// Verarbeitete Bild-Rasterungen (Pixel 255 = gebrannt) an ihrer mm-Box.
    pub rasters: Vec<luxifer_core::RasterTexture>,
    pub legend: PreviewLegend,
}

/// Read-only Jobpfad: Arbeitsbewegungen nach Layerfarbe, Leerfahrten dezent,
/// Bild-Layer als verarbeitete Rastertextur. Grundlage ist ausschließlich die
/// Application-Preview (derselbe JobPlan wie Export/Treiber).
pub fn preview_vertices(session: &EditorSession, selection_only: bool) -> PreviewGeometry {
    let mut v = scene_geo::bed_grid(session.bed_w_mm as f32, session.bed_h_mm as f32);
    let background_end = v.len() as u32;
    let preview = session.job_preview(selection_only);
    let mut legend = PreviewLegend {
        bbox: preview.bbox,
        has_raster: !preview.rasters.is_empty(),
        ..Default::default()
    };
    for movement in &preview.moves {
        let color = match movement.kind {
            luxifer_core::preview::MoveKind::Travel => PREVIEW_TRAVEL,
            luxifer_core::preview::MoveKind::Fill => PREVIEW_FILL,
            luxifer_core::preview::MoveKind::Raster => PREVIEW_RASTER,
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
        match movement.kind {
            luxifer_core::preview::MoveKind::Travel => {
                legend.has_travel = true;
                legend.travel_len_mm += movement.len_mm();
            }
            luxifer_core::preview::MoveKind::Fill => {
                legend.has_fill = true;
                legend.work_len_mm += movement.len_mm();
            }
            luxifer_core::preview::MoveKind::Raster => {
                legend.work_len_mm += movement.len_mm();
            }
            luxifer_core::preview::MoveKind::Cut => {
                legend.work_len_mm += movement.len_mm();
                if let Some(layer) = session.layers.get(movement.layer_id) {
                    if !legend.cut_layers.iter().any(|(n, _)| n == &layer.name) {
                        legend.cut_layers.push((layer.name.clone(), layer.color));
                    }
                }
            }
        }
        scene_geo::push_seg(
            &mut v,
            [movement.from.0 as f32, movement.from.1 as f32],
            [movement.to.0 as f32, movement.to.1 as f32],
            color,
        );
    }
    PreviewGeometry {
        vertices: v,
        background_end,
        rasters: preview.rasters,
        legend,
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
