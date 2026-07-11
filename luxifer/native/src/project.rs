//! Projekt-Backend fürs native Frontend: aktuelles Projekt halten, speichern
//! (in-place / neue Version), öffnen, auflisten. Nutzt die Core-Projekt-API
//! (ProjectFile, list_projects) — keine Logik-Duplikate.

use luxifer_core::{
    list_projects, project::ProjectFile, projects_dir, state::AppState, ProjectInfo,
};

/// Hält das offene Projekt (oder None = namenloser Arbeitsstand).
#[derive(Default)]
pub struct ProjectBackend {
    pub open: Option<ProjectFile>,
    /// Letzte Statusmeldung (Speichern/Öffnen).
    pub msg: String,
}

impl ProjectBackend {
    /// Alle Projekte (für die Liste im Reiter).
    pub fn list(&self) -> Vec<ProjectInfo> {
        list_projects(&projects_dir())
    }

    pub fn open_name(&self) -> Option<&str> {
        self.open.as_ref().map(|p| p.name.as_str())
    }

    /// Neues Projekt aus dem aktuellen Canvas-Zustand anlegen (noch nicht
    /// gespeichert — erst `save` schreibt auf die Platte).
    pub fn new_from_state(&mut self, state: &AppState, name: &str) {
        self.open = Some(ProjectFile::from_state(state, name, Vec::new()));
        self.msg = format!("Neues Projekt: {name}");
    }

    /// Projekt laden und seinen Zustand zurückgeben (der Aufrufer ersetzt den
    /// Canvas-State damit).
    pub fn open(&mut self, name: &str) -> Option<AppState> {
        match ProjectFile::load_by_name(&projects_dir(), name) {
            Ok(pf) => {
                let state = pf.clone().into_state();
                self.open = Some(pf);
                self.msg = format!("Geöffnet: {name}");
                Some(state)
            }
            Err(e) => {
                self.msg = format!("Öffnen fehlgeschlagen: {e}");
                None
            }
        }
    }

    /// In-place speichern (Strg+S). Synchronisiert das ProjectFile mit dem
    /// aktuellen Canvas-Zustand und schreibt die aktuelle Version.
    pub fn save(&mut self, state: &AppState) {
        let Some(pf) = self.open.as_mut() else {
            self.msg = "Kein Projekt — erst 'Neu' oder Name vergeben.".into();
            return;
        };
        pf.update_from_state(state);
        // Ohne Thumb-Rasterung: leeres PNG (Vorschau ist optional).
        match pf.save_current(&projects_dir(), &[]) {
            Ok(v) => self.msg = format!("Gespeichert ({})", v.label),
            Err(e) => self.msg = format!("Speichern-Fehler: {e}"),
        }
    }

    /// Als neue Version speichern (Shift+Strg+S).
    pub fn save_version(&mut self, state: &AppState) {
        let Some(pf) = self.open.as_mut() else {
            self.msg = "Kein Projekt offen.".into();
            return;
        };
        pf.update_from_state(state);
        match pf.add_version(&projects_dir(), String::new(), &[]) {
            Ok(v) => self.msg = format!("Neue Version {}", v.label),
            Err(e) => self.msg = format!("Versions-Fehler: {e}"),
        }
    }
}
