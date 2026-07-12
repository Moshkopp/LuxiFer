//! Anordnen-Leiste (zweite Kopfzeile im Design-Reiter): Ausrichten, Verteilen,
//! Gruppieren/Lösen und Nesting.

use egui::Color32;

use crate::app::App;

/// Kleiner horizontaler Icon-Knopf (Anordnen-Leiste). `dim` = deaktiviert.
fn bar_icon(ui: &mut egui::Ui, icon: &str, tip: &str, enabled: bool) -> bool {
    let side = 28.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::click());
    let hov = resp.hovered() && enabled;
    let bg = if hov {
        Color32::from_rgb(0x25, 0x2a, 0x33)
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 6.0, bg);
    let fg = if enabled {
        Color32::from_rgb(0xd4, 0xd8, 0xdd)
    } else {
        Color32::from_rgb(0x55, 0x5a, 0x62)
    };
    let pad = side * 0.2;
    let ic = egui::Rect::from_min_max(
        rect.min + egui::vec2(pad, pad),
        rect.max - egui::vec2(pad, pad),
    );
    crate::icons::draw(ui.painter(), ic, icon, fg);
    enabled && resp.on_hover_text(tip).clicked()
}

/// Anordnen-Leiste: Ausrichten (7), Verteilen (4), Gruppieren/Lösen, Nesting.
pub(super) fn arrange_bar(ui: &mut egui::Ui, app: &mut App) {
    use luxifer_core::{Align, Distribute};
    let n = app.selection_count();
    ui.horizontal(|ui| {
        // Ausrichten (ab 1 Objekt).
        let a1 = n >= 1;
        if bar_icon(ui, "align-left", "Links ausrichten", a1) {
            app.align(Align::Left);
        }
        if bar_icon(ui, "align-hcenter", "Horizontal zentrieren", a1) {
            app.align(Align::HCenter);
        }
        if bar_icon(ui, "align-right", "Rechts ausrichten", a1) {
            app.align(Align::Right);
        }
        ui.add_space(2.0);
        if bar_icon(ui, "align-top", "Oben ausrichten", a1) {
            app.align(Align::Top);
        }
        if bar_icon(ui, "align-vcenter", "Vertikal zentrieren", a1) {
            app.align(Align::VCenter);
        }
        if bar_icon(ui, "align-bottom", "Unten ausrichten", a1) {
            app.align(Align::Bottom);
        }
        if bar_icon(ui, "align-center", "Auf beiden Achsen zentrieren", a1) {
            app.align(Align::Center);
        }
        ui.separator();
        // Verteilen (ab 3 Objekten).
        let a3 = n >= 3;
        if bar_icon(ui, "dist-h", "Horizontal verteilen", a3) {
            app.distribute(Distribute::Horizontal);
        }
        if bar_icon(ui, "space-h", "Horizontale Abstände angleichen", a3) {
            app.distribute(Distribute::SpaceHorizontal);
        }
        if bar_icon(ui, "dist-v", "Vertikal verteilen", a3) {
            app.distribute(Distribute::Vertical);
        }
        if bar_icon(ui, "space-v", "Vertikale Abstände angleichen", a3) {
            app.distribute(Distribute::SpaceVertical);
        }
        ui.separator();
        // Gruppieren.
        if bar_icon(ui, "group", "Gruppieren", n >= 2) {
            app.group();
        }
        if bar_icon(ui, "ungroup", "Gruppierung lösen", n >= 1) {
            app.ungroup();
        }
        ui.separator();
        // Nesting: Packen (≥2) / Bett füllen (≥1), fester Abstand 2 mm.
        if bar_icon(ui, "nest", "Auswahl packen (2 mm)", n >= 2) {
            app.nest(2.0);
        }
        if ui
            .add_enabled(n >= 1, egui::Button::new("Bett füllen"))
            .clicked()
        {
            app.nest_fill(2.0);
        }
    });
}
