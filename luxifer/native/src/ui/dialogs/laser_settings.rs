//! Modaler Laser-Einstellungen-Dialog (Profil anlegen/bearbeiten/löschen).

use luxifer_core::LaserProfile;

/// Ergebnis des Laser-Dialogs. Eigenes Enum, weil er zusätzlich „Löschen" kennt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(in crate::ui) enum LaserDialogOutcome {
    #[default]
    None,
    Save,
    Delete,
    Cancel,
}

/// Zeichnet das Fenster auf den Entwurf `profile` und meldet die gewählte
/// Aktion. Keine Mutation außerhalb des Entwurfs; die Persistenz macht der Root.
pub(in crate::ui) fn laser_settings_window(
    ctx: &egui::Context,
    profile: &mut LaserProfile,
) -> LaserDialogOutcome {
    use luxifer_core::{Connection, DriverKind};
    let mut outcome = LaserDialogOutcome::None;
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
                    outcome = LaserDialogOutcome::Save;
                }
                if !profile.id.is_empty() && ui.button("Löschen").clicked() {
                    outcome = LaserDialogOutcome::Delete;
                }
                if ui.button("Abbrechen").clicked() {
                    outcome = LaserDialogOutcome::Cancel;
                }
            });
        });
    outcome
}
