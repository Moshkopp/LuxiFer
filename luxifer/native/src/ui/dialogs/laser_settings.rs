//! Modaler Laser-Einstellungen-Dialog (Profil anlegen/bearbeiten/löschen).

use crate::app::App;

/// Modaler Laser-Einstellungen-Dialog (Profil anlegen/bearbeiten/löschen).
pub(in crate::ui) fn laser_settings_window(ctx: &egui::Context, app: &mut App) {
    use luxifer_core::{Connection, DriverKind};
    let Some(mut profile) = app.laser_settings.take() else {
        return;
    };
    let mut action: Option<&str> = None;
    egui::Window::new("Laser-Einstellungen")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_min_width(320.0);
            egui::Grid::new("laser_cfg")
                .num_columns(2)
                .spacing([8.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(&mut profile.name);
                    ui.end_row();

                    ui.label("Typ");
                    egui::ComboBox::from_id_salt("kind")
                        .selected_text(format!("{:?}", profile.kind))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut profile.kind, DriverKind::Ruida, "Ruida");
                            ui.selectable_value(&mut profile.kind, DriverKind::Grbl, "GRBL");
                            ui.selectable_value(
                                &mut profile.kind,
                                DriverKind::MiniGrbl,
                                "miniGRBL",
                            );
                        });
                    ui.end_row();

                    // Verbindung: je nach Treiber Netz (IP) oder Seriell (Port).
                    match &mut profile.connection {
                        Connection::Netz { ip, .. } => {
                            ui.label("IP-Adresse");
                            ui.text_edit_singleline(ip);
                            ui.end_row();
                        }
                        Connection::Seriell { port, baud } => {
                            ui.label("Port");
                            ui.text_edit_singleline(port);
                            ui.end_row();
                            ui.label("Baud");
                            ui.add(egui::DragValue::new(baud));
                            ui.end_row();
                        }
                    }

                    ui.label("Bett B×H (mm)");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut profile.bed_mm.0).speed(1.0));
                        ui.label("×");
                        ui.add(egui::DragValue::new(&mut profile.bed_mm.1).speed(1.0));
                    });
                    ui.end_row();
                });

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Speichern").clicked() {
                    action = Some("save");
                }
                if !profile.id.is_empty() && ui.button("Löschen").clicked() {
                    action = Some("delete");
                }
                if ui.button("Abbrechen").clicked() {
                    action = Some("cancel");
                }
            });
        });

    match action {
        Some("save") => {
            app.laser_settings = Some(profile);
            app.save_laser_settings();
        }
        Some("delete") => {
            app.delete_laser_profile(&profile.id.clone());
        }
        Some("cancel") => {}
        // Keine Aktion + Fenster noch offen → Bearbeitungsstand behalten.
        _ => app.laser_settings = Some(profile),
    }
}
