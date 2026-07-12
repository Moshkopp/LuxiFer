//! Farbpalette und Form-Wähler (unteres Dock im Design-Reiter).
//!
//! Über die `UiAction`-Grenze (ADR 0011): beide Panels lesen nur ihren
//! aktiven Wert und liefern Absichten zurück, statt `App` zu mutieren.

use egui::{Color32, RichText};
use luxifer_core::{model::SWATCH_COLORS, PolyShape};

use super::action::UiAction;
use super::c32;
use super::tools::icon_button;

/// Form-Wähler für das Polygon-Werkzeug (Dreieck/Stern/… wie Tauri-ShapesPanel).
/// `active` = aktuell gewählte Form (nur für die Markierung).
pub(super) fn shape_picker(ui: &mut egui::Ui, active: PolyShape) -> Vec<UiAction> {
    use luxifer_core::PolyShape as P;
    let shapes = [
        (P::Tri, "tri"),
        (P::Quad, "quad"),
        (P::Penta, "penta"),
        (P::Hex, "hex"),
        (P::Octa, "octa"),
        (P::Star, "star"),
        (P::Sun, "sun"),
        (P::Gear, "gear"),
        (P::Heart, "heart"),
    ];
    let mut actions = Vec::new();
    ui.horizontal(|ui| {
        for (shape, icon) in shapes {
            let on = active == shape;
            if icon_button(ui, 30.0, icon, "", on, false) {
                actions.push(UiAction::SelectShape(shape));
            }
        }
    });
    actions
}

/// `active` = aktive Zeichenfarbe (nur für die Markierung des Swatches).
pub(super) fn palette_panel(ui: &mut egui::Ui, active: [u8; 3]) -> Vec<UiAction> {
    ui.label(RichText::new("FARBE").small().weak());
    ui.add_space(6.0);
    let mut actions = Vec::new();
    ui.horizontal_wrapped(|ui| {
        for &sw in SWATCH_COLORS {
            let is_active = sw == active;
            let size = if is_active { 26.0 } else { 22.0 };
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
            let r = size * 0.5;
            ui.painter().circle_filled(rect.center(), r, c32(sw));
            if is_active {
                // Heller Ring mit dunklem Absatz — wie in der Svelte-Palette.
                ui.painter()
                    .circle_stroke(rect.center(), r + 1.5, (2.0, Color32::from_gray(20)));
                ui.painter()
                    .circle_stroke(rect.center(), r + 3.0, (2.0, Color32::from_gray(235)));
            }
            if resp.hovered() {
                ui.painter()
                    .circle_stroke(rect.center(), r, (1.5, Color32::WHITE));
            }
            if resp.clicked() {
                actions.push(UiAction::PickColor(sw));
            }
        }
    });
    actions
}
