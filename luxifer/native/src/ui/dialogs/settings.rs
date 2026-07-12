//! Einstellungen-Dialog mit Sektionen wie das Tauri-Modal: Oberfläche
//! (Arbeitsplatz, Raster, Theme), Laser (Profil-Verwaltung inkl.
//! Scan-Offset-Kalibrierung, ADR 0007) und Über (git-abgeleitete Version).
//! Native hält nur Entwürfe; Klemmen/Persistenz machen Core bzw. LaserService.

use egui::RichText;
use luxifer_core::ui_settings::{GRID_SIZE_MAX, GRID_SIZE_MIN, INTENSITY_MAX, INTENSITY_MIN};
use luxifer_core::{LaserProfile, LaserRegistry, ScanOffsetPoint};

use crate::ui::state::{SettingsDialogState, SettingsSection};

/// Ergebnis des Einstellungen-Dialogs. Eigenes Enum, weil die Laser-Sektion
/// zusätzlich Profil-Aktionen kennt, die den Dialog offen lassen.
#[derive(Debug, Clone, PartialEq, Default)]
pub(in crate::ui) enum SettingsOutcome {
    #[default]
    None,
    /// GUI-Settings übernehmen und Dialog schließen.
    Commit,
    Cancel,
    /// Laser-Profil-Entwurf speichern (Dialog bleibt offen).
    LaserSave,
    /// Laser-Profil mit dieser ID löschen (Dialog bleibt offen).
    LaserDelete(String),
}

pub(in crate::ui) fn settings_dialog_window(
    ctx: &egui::Context,
    st: &mut SettingsDialogState,
    registry: &LaserRegistry,
) -> SettingsOutcome {
    let mut outcome = SettingsOutcome::None;
    egui::Window::new("Einstellungen")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_min_size(egui::vec2(600.0, 360.0));
            ui.horizontal_top(|ui| {
                // Sektions-Navigation links (wie das Tauri-Modal).
                ui.vertical(|ui| {
                    ui.set_width(120.0);
                    for (section, label) in [
                        (SettingsSection::Oberflaeche, "Oberfläche"),
                        (SettingsSection::Laser, "Laser"),
                        (SettingsSection::Ueber, "Über"),
                    ] {
                        if ui.selectable_label(st.section == section, label).clicked() {
                            st.section = section;
                        }
                    }
                });
                ui.separator();
                ui.vertical(|ui| {
                    egui::ScrollArea::vertical()
                        .max_height(320.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| match st.section {
                            SettingsSection::Oberflaeche => ui_section(ui, st),
                            SettingsSection::Laser => laser_section(ui, st, registry, &mut outcome),
                            SettingsSection::Ueber => about_section(ui),
                        });
                });
            });

            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Speichern").clicked() {
                    outcome = SettingsOutcome::Commit;
                }
                if ui.button("Abbrechen").clicked() {
                    outcome = SettingsOutcome::Cancel;
                }
            });
        });
    outcome
}

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

fn ui_section(ui: &mut egui::Ui, st: &mut SettingsDialogState) {
    let s = &mut st.draft;
    egui::Grid::new("settings_ui")
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
}

/// Laser-Sektion: Profil-Liste + Formular des Entwurfs (inkl. Scan-Offset).
fn laser_section(
    ui: &mut egui::Ui,
    st: &mut SettingsDialogState,
    registry: &LaserRegistry,
    outcome: &mut SettingsOutcome,
) {
    if registry.profiles.is_empty() {
        ui.weak("Noch kein Laser angelegt.");
    }
    for profile in &registry.profiles {
        let is_active = registry.active_id.as_deref() == Some(profile.id.as_str());
        ui.horizontal(|ui| {
            let title = if is_active {
                format!("{}  (aktiv)", profile.name)
            } else {
                profile.name.clone()
            };
            ui.label(RichText::new(title).strong());
            ui.weak(format!("{:?}", profile.kind));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Löschen").clicked() {
                    *outcome = SettingsOutcome::LaserDelete(profile.id.clone());
                }
                if ui.button("Bearbeiten").clicked() {
                    st.laser_draft = Some(profile.clone());
                }
            });
        });
    }
    ui.add_space(4.0);
    if ui.button("+ Neuer Laser").clicked() {
        st.laser_draft = Some(LaserProfile::default());
    }

    let Some(profile) = st.laser_draft.as_mut() else {
        return;
    };
    ui.add_space(8.0);
    ui.separator();
    laser_profile_form(ui, profile);

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button("Profil speichern").clicked() {
            *outcome = SettingsOutcome::LaserSave;
        }
        if ui.button("Verwerfen").clicked() {
            st.laser_draft = None;
        }
    });
}

/// Formular eines Laser-Profils (vormals eigener Dialog): Name, Treiber,
/// Verbindung, Bett und die Scan-Offset-Kalibrierung.
fn laser_profile_form(ui: &mut egui::Ui, profile: &mut LaserProfile) {
    use luxifer_core::{Connection, DriverKind};
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
                    ui.selectable_value(&mut profile.kind, DriverKind::MiniGrbl, "miniGRBL");
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

    // Scan-Offset (Reversal-Korrektur, ADR 0006 §6): Tabelle speed → offset;
    // der Treiber interpoliert linear und extrapoliert über die Ränder.
    ui.add_space(6.0);
    ui.checkbox(
        &mut profile.scan_offset.enabled,
        "Scan-Offset (Reversal-Korrektur) aktiv",
    );
    if profile.scan_offset.enabled {
        ui.weak("Zeilenversatz je Geschwindigkeit — Kanten fransen beim bidirektionalen Rastern sonst aus.");
        let mut remove: Option<usize> = None;
        for (i, pt) in profile.scan_offset.points.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.add(
                    egui::DragValue::new(&mut pt.speed_mm_s)
                        .range(1.0..=10_000.0)
                        .speed(1.0)
                        .suffix(" mm/s"),
                );
                ui.label("→");
                ui.add(
                    egui::DragValue::new(&mut pt.offset_mm)
                        .range(-5.0..=5.0)
                        .speed(0.01)
                        .suffix(" mm"),
                );
                if ui.small_button("✕").clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            profile.scan_offset.points.remove(i);
        }
        if ui.button("+ Stützpunkt").clicked() {
            profile.scan_offset.points.push(ScanOffsetPoint {
                speed_mm_s: 100.0,
                offset_mm: 0.1,
            });
        }
    }
}

fn about_section(ui: &mut egui::Ui) {
    ui.heading("LuxiFer");
    ui.weak("Offline-first Laser-Steuerung.");
    ui.add_space(6.0);
    // Version wächst mit jedem Commit (git describe, siehe build.rs).
    ui.label(format!("Version: {}", env!("LUXIFER_VERSION")));
    ui.label(format!("Commit: {}", env!("LUXIFER_COMMIT")));
}
