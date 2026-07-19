//! Kleine Verwaltung lokaler Materialstandards (ADR 0019, Prototyp).

use crate::ui::MaterialManagerState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(in crate::ui) enum MaterialManagerOutcome {
    #[default]
    None,
    Save,
    Delete,
    Cancel,
}

pub(in crate::ui) fn material_manager_window(
    root_ui: &egui::Ui,
    state: &mut MaterialManagerState,
) -> MaterialManagerOutcome {
    let mut outcome = MaterialManagerOutcome::None;
    egui::Window::new(if state.is_new {
        "Material anlegen"
    } else {
        "Material bearbeiten"
    })
    .order(egui::Order::Foreground)
    .collapsible(false)
    .resizable(false)
    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
    .show(root_ui, |ui| {
        ui.set_min_width(520.0);
        egui::Grid::new("material_identity")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label("Material");
                ui.add(egui::TextEdit::singleline(&mut state.draft.name).desired_width(260.0));
                ui.end_row();
                ui.label("Stärke");
                ui.horizontal(|ui| {
                    let mut enabled = state.draft.thickness_mm.is_some();
                    if ui.checkbox(&mut enabled, "").changed() {
                        state.draft.thickness_mm = enabled.then_some(3.0);
                    }
                    if let Some(thickness) = state.draft.thickness_mm.as_mut() {
                        ui.add(egui::DragValue::new(thickness).range(0.01..=1000.0));
                        ui.label("mm");
                    } else {
                        ui.weak("nicht relevant");
                    }
                });
                ui.end_row();
            });

        ui.add_space(12.0);
        ui.weak("Nur benötigte Prozessstandards aktivieren. Werte können später aus einem Layer übernommen werden.");
        for process in studio_core::MaterialProcess::ALL {
            process_editor(ui, &mut state.draft, process);
        }

        ui.add_space(12.0);
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Speichern").clicked() {
                outcome = MaterialManagerOutcome::Save;
            }
            if ui.button("Abbrechen").clicked() {
                outcome = MaterialManagerOutcome::Cancel;
            }
            if !state.is_new
                && ui
                    .button(egui::RichText::new("Löschen").color(ui.visuals().error_fg_color))
                    .clicked()
            {
                outcome = MaterialManagerOutcome::Delete;
            }
        });
    });
    outcome
}

fn process_editor(
    ui: &mut egui::Ui,
    profile: &mut studio_core::MaterialProfile,
    process: studio_core::MaterialProcess,
) {
    let slot = profile.defaults_mut(process);
    let mut enabled = slot.is_some();
    egui::CollapsingHeader::new(process.label())
        .default_open(enabled)
        .show(ui, |ui| {
            if ui.checkbox(&mut enabled, "Standard verwenden").changed() {
                *slot = enabled.then(default_process_values);
            }
            let Some(values) = slot.as_mut() else {
                return;
            };
            egui::Grid::new(("material_process", process))
                .num_columns(4)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Speed");
                    ui.add(egui::DragValue::new(&mut values.speed_mm_s).range(1.0..=10000.0));
                    ui.label("mm/s");
                    ui.label("");
                    ui.end_row();
                    ui.label("Power max");
                    ui.add(egui::DragValue::new(&mut values.power_pct).range(0.0..=100.0));
                    ui.label("%");
                    ui.label("Power min");
                    ui.add(egui::DragValue::new(&mut values.min_power_pct).range(0.0..=100.0));
                    ui.end_row();
                    ui.label("Durchläufe");
                    ui.add(egui::DragValue::new(&mut values.passes).range(1..=100));
                    ui.label("");
                    ui.checkbox(&mut values.air_assist, "Air Assist");
                    ui.end_row();
                    match process {
                        studio_core::MaterialProcess::Cut => {}
                        studio_core::MaterialProcess::VectorEngrave => {
                            ui.label("Linienabstand");
                            ui.add(
                                egui::DragValue::new(&mut values.line_step_mm).range(0.01..=10.0),
                            );
                            ui.label("mm");
                            ui.label("");
                            ui.end_row();
                        }
                        studio_core::MaterialProcess::RasterEngrave => {
                            ui.label("DPI");
                            ui.add(egui::DragValue::new(&mut values.dpi).range(1.0..=2540.0));
                            ui.label("");
                            ui.checkbox(&mut values.bidirectional, "Bidirektional");
                            ui.end_row();
                        }
                    }
                });
        });
}

fn default_process_values() -> studio_core::MaterialProcessDefaults {
    studio_core::MaterialProcessDefaults::from_layer(&studio_core::Layer::new(0))
}
