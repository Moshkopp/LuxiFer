//! Ebenenliste (rechtes Inspector-Panel im Design-Reiter). Farbe = Layer;
//! Doppelklick auf den Namen öffnet den Parameter-Dialog.
//!
//! Über die `UiAction`-Grenze (ADR 0011): Das Panel bekommt eine reine Sicht
//! (`LayerRow`) statt `&mut App` und liefert Absichten zurück.

use egui::RichText;
use luxifer_application::LayerToggle;

use super::action::UiAction;
use super::c32;

/// Reine Darstellungssicht einer Ebene für die Liste. Vom Root aus der Session
/// abgeleitet, damit das Panel nicht selbst auf den Zustand zugreift.
pub(super) struct LayerRow {
    pub color: [u8; 3],
    pub name: String,
    pub visible: bool,
    pub enabled: bool,
    pub locked: bool,
    pub air_assist: bool,
    /// Anzahl Shapes auf dieser Ebene.
    pub count: usize,
}

/// `rows` sind in Layer-Reihenfolge (Index 0 = unterste). Angezeigt wird von
/// oben (letzte Ebene) nach unten. Gibt die ausgelösten Absichten zurück.
pub(super) fn layers_panel(ui: &mut egui::Ui, rows: &[LayerRow]) -> Vec<UiAction> {
    let mut actions = Vec::new();
    ui.label(RichText::new("EBENEN").small().weak());
    ui.add_space(4.0);
    if rows.is_empty() {
        ui.weak("Keine Ebenen — zeichne etwas.");
        return actions;
    }
    let n = rows.len();
    // Von oben (letzter Layer) nach unten anzeigen.
    for i in (0..n).rev() {
        let row = &rows[i];
        ui.horizontal(|ui| {
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::click());
            ui.painter().rect_filled(rect, 4.0, c32(row.color));
            if resp.clicked() {
                actions.push(UiAction::PickColor(row.color));
            }
            if ui
                .selectable_label(row.visible, "S")
                .on_hover_text("Im Canvas sichtbar")
                .clicked()
            {
                actions.push(UiAction::ToggleLayer(i, LayerToggle::Visible));
            }
            if ui
                .selectable_label(row.enabled, "J")
                .on_hover_text("Im Laserjob aktiviert")
                .clicked()
            {
                actions.push(UiAction::ToggleLayer(i, LayerToggle::Enabled));
            }
            if ui
                .selectable_label(row.locked, "L")
                .on_hover_text("Bearbeitung sperren")
                .clicked()
            {
                actions.push(UiAction::ToggleLayer(i, LayerToggle::Locked));
            }
            if ui
                .selectable_label(row.air_assist, "A")
                .on_hover_text("Luftunterstützung")
                .clicked()
            {
                actions.push(UiAction::ToggleLayer(i, LayerToggle::AirAssist));
            }
            if ui
                .add(
                    egui::Label::new(format!("{}  ·  {}", row.name, row.count))
                        .sense(egui::Sense::click()),
                )
                .on_hover_text("Doppelklick: Parameter bearbeiten")
                .double_clicked()
            {
                actions.push(UiAction::OpenLayerDialog(i));
            }
            if ui
                .small_button("↑")
                .on_hover_text("Ebene nach oben")
                .clicked()
                && i + 1 < n
            {
                actions.push(UiAction::MoveLayer { from: i, to: i + 1 });
            }
            if ui
                .small_button("↓")
                .on_hover_text("Ebene nach unten")
                .clicked()
                && i > 0
            {
                actions.push(UiAction::MoveLayer { from: i, to: i - 1 });
            }
        });
    }
    actions
}
