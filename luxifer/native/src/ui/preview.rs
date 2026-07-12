//! Legende des Preview-Reiters: erklärt die Farben der Jobvorschau und zeigt
//! die Kennzahlen (Arbeitsweg, Leerfahrt, Job-Fläche). Rein lesend — die Daten
//! kommen als [`PreviewLegend`] vom letzten Preview-Aufbau des Renderers.

use crate::canvas::scene::{PreviewLegend, PREVIEW_FILL, PREVIEW_RASTER, PREVIEW_TRAVEL};

/// [f32;4]-Szenenfarbe → egui-Farbe (die Legende zeigt exakt die Preview-Töne).
fn scene_color(c: [f32; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(
        (c[0] * 255.0) as u8,
        (c[1] * 255.0) as u8,
        (c[2] * 255.0) as u8,
        (c[3] * 255.0) as u8,
    )
}

/// Eine Legendenzeile: Farbfeld + Beschriftung.
fn row(ui: &mut egui::Ui, color: egui::Color32, label: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 3.0, color);
        ui.label(label);
    });
}

/// Längenangabe menschenlesbar: unter einem Meter mm, sonst m.
fn len_label(mm: f64) -> String {
    if mm >= 1000.0 {
        format!("{:.2} m", mm / 1000.0)
    } else {
        format!("{mm:.0} mm")
    }
}

/// Zeigt die Legende als schwebendes Fenster rechts oben im Preview-Reiter.
pub(super) fn preview_legend_window(ctx: &egui::Context, legend: &PreviewLegend) {
    egui::Window::new("Legende")
        .anchor(egui::Align2::RIGHT_TOP, [-12.0, 12.0])
        .collapsible(true)
        .resizable(false)
        .show(ctx, |ui| {
            let empty = legend.cut_layers.is_empty()
                && !legend.has_fill
                && !legend.has_raster
                && !legend.has_travel;
            if empty {
                ui.weak("Keine aktiven Job-Inhalte.");
                ui.weak("Layer aktivieren oder Formen anlegen.");
                return;
            }
            for (name, color) in &legend.cut_layers {
                row(
                    ui,
                    egui::Color32::from_rgb(color[0], color[1], color[2]),
                    &format!("Schnitt — {name}"),
                );
            }
            if legend.has_fill {
                row(ui, scene_color(PREVIEW_FILL), "Füllung (Scanlinien)");
            }
            if legend.has_raster {
                row(
                    ui,
                    scene_color(PREVIEW_RASTER),
                    "Bild-Gravur (verarbeitetes Raster)",
                );
            }
            if legend.has_travel {
                row(ui, scene_color(PREVIEW_TRAVEL), "Leerfahrt");
            }

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(2.0);
            ui.label(format!("Arbeitsweg: {}", len_label(legend.work_len_mm)));
            if legend.has_travel {
                ui.label(format!("Leerfahrt: {}", len_label(legend.travel_len_mm)));
            }
            if let Some((x0, y0, x1, y1)) = legend.bbox {
                ui.label(format!("Job-Fläche: {:.1} × {:.1} mm", x1 - x0, y1 - y0));
            }
        });
}
