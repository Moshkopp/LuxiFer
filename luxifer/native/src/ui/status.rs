//! Fehler-Banner (oben) und Statuszeile (unten). Beide rein darstellend; der
//! Banner meldet nur „schließen" als Absicht zurück.

use egui::{Color32, RichText};

use super::action::UiAction;

/// Rotes Banner mit `message` und stabilem Fehlercode. Gibt `DismissError`
/// zurück, wenn der Nutzer schließt.
pub(super) fn error_banner(ui: &mut egui::Ui, message: &str, code: &str) -> Vec<UiAction> {
    let mut actions = Vec::new();
    ui.horizontal(|ui| {
        ui.colored_label(
            Color32::from_rgb(0xf8, 0x71, 0x71),
            format!("{message}  [{code}]"),
        );
        if ui.small_button("Schließen").clicked() {
            actions.push(UiAction::DismissError);
        }
    });
    actions
}

/// Statuszeile: FPS, aktives Werkzeug, Objektzahl und optionale Projektmeldung.
/// Rein lesend.
pub(super) fn status_bar(ui: &mut egui::Ui, fps: f32, tool: &str, shapes: usize, msg: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{fps:.0} fps")).monospace());
        ui.separator();
        ui.label(format!("Werkzeug: {tool}"));
        ui.separator();
        ui.label(format!("{shapes} Objekte"));
        if !msg.is_empty() {
            ui.separator();
            ui.label(RichText::new(msg).weak());
        }
    });
}
