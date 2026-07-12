//! Ebenenliste (rechtes Inspector-Panel im Design-Reiter). Farbe = Layer;
//! Doppelklick auf den Namen öffnet den Parameter-Dialog.

use egui::RichText;
use luxifer_application::LayerToggle;

use super::c32;
use crate::app::App;

pub(super) fn layers_panel(ui: &mut egui::Ui, app: &mut App) {
    ui.label(RichText::new("EBENEN").small().weak());
    ui.add_space(4.0);
    if app.session.layers.is_empty() {
        ui.weak("Keine Ebenen — zeichne etwas.");
        return;
    }
    // Von oben (letzter Layer) nach unten anzeigen.
    let n = app.session.layers.len();
    for i in (0..n).rev() {
        let (color, name, visible, enabled, locked, air_assist, count) = {
            let l = &app.session.layers[i];
            let cnt = app
                .session
                .shapes
                .iter()
                .filter(|s| s.layer_id == i)
                .count();
            (
                l.color,
                l.name.clone(),
                l.visible,
                l.enabled,
                l.locked,
                l.air_assist,
                cnt,
            )
        };
        ui.horizontal(|ui| {
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::click());
            ui.painter().rect_filled(rect, 4.0, c32(color));
            if resp.clicked() {
                app.pick_color(color);
            }
            if ui
                .selectable_label(visible, "S")
                .on_hover_text("Im Canvas sichtbar")
                .clicked()
            {
                app.toggle_layer(i, LayerToggle::Visible);
            }
            if ui
                .selectable_label(enabled, "J")
                .on_hover_text("Im Laserjob aktiviert")
                .clicked()
            {
                app.toggle_layer(i, LayerToggle::Enabled);
            }
            if ui
                .selectable_label(locked, "L")
                .on_hover_text("Bearbeitung sperren")
                .clicked()
            {
                app.toggle_layer(i, LayerToggle::Locked);
            }
            if ui
                .selectable_label(air_assist, "A")
                .on_hover_text("Luftunterstützung")
                .clicked()
            {
                app.toggle_layer(i, LayerToggle::AirAssist);
            }
            if ui
                .add(egui::Label::new(format!("{name}  ·  {count}")).sense(egui::Sense::click()))
                .on_hover_text("Doppelklick: Parameter bearbeiten")
                .double_clicked()
            {
                app.open_layer_dialog(i);
            }
            if ui
                .small_button("↑")
                .on_hover_text("Ebene nach oben")
                .clicked()
                && i + 1 < n
            {
                app.move_layer(i, i + 1);
            }
            if ui
                .small_button("↓")
                .on_hover_text("Ebene nach unten")
                .clicked()
                && i > 0
            {
                app.move_layer(i, i - 1);
            }
        });
    }
}
