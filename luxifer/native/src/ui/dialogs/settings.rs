//! Einstellungen-Dialog (ADR 0002): Arbeitsplatz, Rasterweite, Theme-Farben
//! plus About-Zeile (Version aus build.rs, wächst mit jedem Commit). Native
//! hält nur den Entwurf; Klemmen und Speichern macht der Core beim Übernehmen.

use egui::RichText;
use luxifer_core::ui_settings::{GRID_SIZE_MAX, GRID_SIZE_MIN, INTENSITY_MAX, INTENSITY_MIN};

use crate::ui::state::SettingsDialogState;

use super::DialogOutcome;

/// Farbe + Intensitäts-Regler einer Theme-Farbe (ADR 0002 §3: Korridor).
fn theme_color_row(ui: &mut egui::Ui, color: &mut luxifer_core::ThemeColor) {
    ui.horizontal(|ui| {
        ui.color_edit_button_srgb(&mut color.hue);
        ui.add(
            egui::Slider::new(&mut color.intensity, INTENSITY_MIN..=INTENSITY_MAX)
                .show_value(false)
                .text("Intensität"),
        );
    });
}

pub(in crate::ui) fn settings_dialog_window(
    ctx: &egui::Context,
    st: &mut SettingsDialogState,
) -> DialogOutcome {
    let mut outcome = DialogOutcome::None;
    egui::Window::new("Einstellungen")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_min_width(360.0);
            let s = &mut st.draft;

            egui::Grid::new("settings_cfg")
                .num_columns(2)
                .spacing([8.0, 10.0])
                .show(ui, |ui| {
                    ui.label("Arbeitsplatz");
                    ui.add(egui::TextEdit::singleline(&mut s.workplace).desired_width(220.0));
                    ui.end_row();

                    ui.label("Raster (mm)");
                    ui.add(
                        egui::DragValue::new(&mut s.grid_size_mm)
                            .range(GRID_SIZE_MIN..=GRID_SIZE_MAX)
                            .speed(1.0),
                    );
                    ui.end_row();

                    ui.label("Akzentfarbe");
                    theme_color_row(ui, &mut s.theme.accent);
                    ui.end_row();

                    ui.label("Buttonfarbe");
                    theme_color_row(ui, &mut s.theme.button);
                    ui.end_row();
                });

            ui.add_space(6.0);
            if ui.button("Theme zurücksetzen").clicked() {
                s.theme = Default::default();
            }

            ui.add_space(10.0);
            ui.separator();
            // Version wächst mit jedem Commit (git describe, siehe build.rs).
            ui.label(
                RichText::new(format!(
                    "LuxiFer {}  ·  Commit {}",
                    env!("LUXIFER_VERSION"),
                    env!("LUXIFER_COMMIT")
                ))
                .small()
                .weak(),
            );

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Speichern").clicked() {
                    outcome = DialogOutcome::Commit;
                }
                if ui.button("Abbrechen").clicked() {
                    outcome = DialogOutcome::Cancel;
                }
            });
        });
    outcome
}
