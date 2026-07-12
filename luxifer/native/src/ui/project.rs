//! Projekt-Browser (Reiter „Projekt"): Liste + Neu/Öffnen/Speichern.

use egui::RichText;

use crate::app::App;

/// Projekt-Browser (Reiter „Projekt"): Liste + Neu/Öffnen/Speichern.
pub(super) fn project_browser(ui: &mut egui::Ui, app: &mut App) {
    ui.add_space(8.0);
    ui.heading("Projekte");
    ui.add_space(8.0);

    // Aktionszeile: Neu + Speichern.
    ui.horizontal(|ui| {
        ui.label("Neu:");
        ui.add(
            egui::TextEdit::singleline(&mut app.new_project_name)
                .hint_text("Projektname")
                .desired_width(200.0),
        );
        if ui.button("Anlegen").clicked() {
            let name = app.new_project_name.clone();
            app.project_new(&name);
            app.new_project_name.clear();
        }
        ui.separator();
        if ui.button("Speichern").clicked() {
            app.project_save();
        }
        if ui.button("Neue Version").clicked() {
            app.project_save_version();
        }
    });
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    // Projektliste.
    let projects = app.project.list();
    if projects.is_empty() {
        ui.weak("Noch keine Projekte gespeichert.");
        return;
    }
    let open_name = app.project.open_name().map(|s| s.to_string());
    egui::ScrollArea::vertical().show(ui, |ui| {
        for p in &projects {
            let is_open = open_name.as_deref() == Some(p.name.as_str());
            ui.horizontal(|ui| {
                let title = if is_open {
                    RichText::new(&p.name).strong()
                } else {
                    RichText::new(&p.name)
                };
                ui.label(title);
                if !p.modified_at.is_empty() {
                    ui.weak(RichText::new(&p.modified_at).small());
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Öffnen").clicked() {
                        app.project_open(&p.name);
                    }
                });
            });
            ui.separator();
        }
    });
}
