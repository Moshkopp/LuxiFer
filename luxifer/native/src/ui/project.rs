//! Projekt-Browser (Reiter „Projekt"): Liste + Neu/Öffnen/Speichern.
//!
//! Über die `UiAction`-Grenze (ADR 0011): Das Panel erhält den Namensentwurf
//! als `&mut String` (immediate-mode-Textfeld), die Projektliste als `&`-Sicht
//! und liefert Absichten zurück. Der Draft-Lebenszyklus (auslesen/leeren) liegt
//! beim Root.

use egui::RichText;
use luxifer_core::project::ProjectInfo;

use super::action::UiAction;

/// `draft_name` = kurzlebiger „Neu"-Namensentwurf; `projects` = vorhandene
/// Projekte; `open_name` = Name des offenen Projekts (für die Hervorhebung).
pub(super) fn project_browser(
    ui: &mut egui::Ui,
    draft_name: &mut String,
    projects: &[ProjectInfo],
    open_name: Option<&str>,
) -> Vec<UiAction> {
    let mut actions = Vec::new();
    ui.add_space(8.0);
    ui.heading("Projekte");
    ui.add_space(8.0);

    // Aktionszeile: Neu + Speichern.
    ui.horizontal(|ui| {
        ui.label("Neu:");
        ui.add(
            egui::TextEdit::singleline(draft_name)
                .hint_text("Projektname")
                .desired_width(200.0),
        );
        if ui.button("Anlegen").clicked() {
            // Der Root liest den Entwurf und leert ihn.
            actions.push(UiAction::NewProject);
        }
        ui.separator();
        if ui.button("Speichern").clicked() {
            actions.push(UiAction::SaveProject);
        }
        if ui.button("Neue Version").clicked() {
            actions.push(UiAction::SaveProjectVersion);
        }
    });
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    // Projektliste.
    if projects.is_empty() {
        ui.weak("Noch keine Projekte gespeichert.");
        return actions;
    }
    egui::ScrollArea::vertical().show(ui, |ui| {
        for p in projects {
            let is_open = open_name == Some(p.name.as_str());
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
                        actions.push(UiAction::OpenProject(p.name.clone()));
                    }
                });
            });
            ui.separator();
        }
    });
    actions
}
