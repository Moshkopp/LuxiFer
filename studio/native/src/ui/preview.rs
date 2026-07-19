//! Rechtes Panel des Preview-Reiters: Material-Vorlage wählen (Untergrund und
//! Brennfarbe der Vorschau) und die Kennzahlen-Legende. Rein lesend bis auf
//! die Materialwahl, die als `UiAction` zum Root läuft.

use egui::RichText;

use crate::canvas::scene::{PreviewLegend, PreviewMaterial};

use super::action::UiAction;

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

/// Eine Material-Karte: Untergrund-Muster mit Brennlinie + Label. Gibt true
/// bei Klick zurück.
fn material_card(ui: &mut egui::Ui, material: PreviewMaterial, active: bool) -> bool {
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 44.0), egui::Sense::click());
    let bg = if active {
        ui.visuals().selection.bg_fill.gamma_multiply(0.35)
    } else if resp.hovered() {
        ui.visuals().widgets.hovered.bg_fill
    } else {
        ui.visuals().widgets.inactive.bg_fill
    };
    let p = ui.painter();
    p.rect_filled(rect, 6.0, bg);
    if active {
        p.rect_stroke(
            rect,
            6.0,
            egui::Stroke::new(1.5, ui.visuals().selection.stroke.color),
            egui::StrokeKind::Inside,
        );
    }
    // Muster links: Untergrund mit einer Brennlinie darüber.
    let swatch = egui::Rect::from_min_size(
        rect.min + egui::vec2(8.0, 8.0),
        egui::vec2(42.0, rect.height() - 16.0),
    );
    p.rect_filled(swatch, 4.0, scene_color(material.bed()));
    p.line_segment(
        [
            swatch.left_center() + egui::vec2(6.0, 4.0),
            swatch.right_center() + egui::vec2(-6.0, -4.0),
        ],
        egui::Stroke::new(2.0, scene_color(material.burn())),
    );
    p.text(
        egui::pos2(swatch.right() + 10.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        material.label(),
        egui::FontId::proportional(13.0),
        ui.visuals().text_color(),
    );
    resp.clicked()
}

/// Baut das Preview-Panel: Material-Auswahl oben, Legende darunter.
pub(super) fn preview_panel(
    ui: &mut egui::Ui,
    current: PreviewMaterial,
    show_travel: bool,
    show_laser_path: bool,
    show_scan_offset: bool,
    legend: Option<&PreviewLegend>,
) -> Vec<UiAction> {
    let mut actions = Vec::new();
    ui.add_space(8.0);
    ui.label(RichText::new("MATERIAL").small().weak());
    ui.add_space(4.0);
    for material in PreviewMaterial::ALL {
        if material_card(ui, material, material == current) {
            actions.push(UiAction::SelectPreviewMaterial(material));
        }
        ui.add_space(6.0);
    }

    let mut travel = show_travel;
    if ui
        .checkbox(&mut travel, "Leerfahrten anzeigen")
        .on_hover_text("Bei vielen Objekten übertünchen Leerfahrten das Motiv")
        .changed()
    {
        actions.push(UiAction::SetPreviewTravel(travel));
    }
    let mut laser_path = show_laser_path;
    if ui
        .checkbox(&mut laser_path, "Laserpfad grün hervorheben")
        .changed()
    {
        actions.push(UiAction::SetPreviewLaserPath(laser_path));
    }
    let mut scan_offset = show_scan_offset;
    if ui
        .checkbox(&mut scan_offset, "Scan-Offset anzeigen")
        .changed()
    {
        actions.push(UiAction::SetPreviewScanOffset(scan_offset));
    }

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(6.0);
    ui.label(RichText::new("LEGENDE").small().weak());
    ui.add_space(4.0);
    let Some(legend) = legend else {
        ui.weak("Wird berechnet …");
        return actions;
    };
    if legend.scan_offset_active && !show_scan_offset {
        ui.weak("Scan-Offset aktiv, in der Darstellung ausgeblendet.");
    }
    if !legend.has_content {
        ui.weak("Keine aktiven Job-Inhalte.");
        ui.weak("Layer aktivieren oder Formen anlegen.");
        return actions;
    }
    row(
        ui,
        scene_color(legend.material.burn()),
        "Brennweg (Schnitt/Gravur)",
    );
    if legend.has_travel {
        row(ui, scene_color(legend.material.travel()), "Leerfahrt");
    }
    ui.add_space(6.0);
    ui.label(format!("Laserweg: {}", len_label(legend.work_len_mm)));
    if legend.has_travel {
        ui.label(format!("Leerfahrt: {}", len_label(legend.travel_len_mm)));
    }
    ui.label(format!(
        "Gesamtfahrweg: {}",
        len_label(legend.work_len_mm + legend.travel_len_mm)
    ));
    if let Some((x0, y0, x1, y1)) = legend.bbox {
        ui.label(format!("Job-Fläche: {:.1} × {:.1} mm", x1 - x0, y1 - y0));
    }
    actions
}
