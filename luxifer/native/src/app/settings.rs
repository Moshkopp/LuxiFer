//! GUI-Einstellungen-Workflow (ADR 0002): Dialog-Entwurf öffnen/übernehmen,
//! inklusive der Laser-Profil-Verwaltung in der Laser-Sektion (ADR 0007).
//! Validierung/Klemmen und Persistenz machen Core (`UiSettings`) bzw.
//! `LaserService`; native wendet nur an (Theme, Raster).

use luxifer_application::AppError;
use luxifer_core::LaserProfile;

use crate::ui::{SettingsDialogState, SettingsSection};

use super::App;

impl App {
    /// Öffnet den Einstellungen-Dialog (Zahnrad) mit den aktuellen Werten.
    pub fn open_settings_dialog(&mut self) {
        self.settings_dialog = Some(SettingsDialogState {
            draft: self.ui_settings.clone(),
            section: SettingsSection::Oberflaeche,
            laser_draft: None,
        });
    }

    /// Öffnet die Einstellungen direkt in der Laser-Sektion (aus dem
    /// Laser-Panel): `edit_active` lädt das aktive Profil als Entwurf,
    /// sonst startet ein neues.
    pub fn open_laser_settings(&mut self, edit_active: bool) {
        let laser_draft = Some(if edit_active {
            self.laser_backend
                .active_profile()
                .cloned()
                .unwrap_or_default()
        } else {
            LaserProfile::default()
        });
        self.settings_dialog = Some(SettingsDialogState {
            draft: self.ui_settings.clone(),
            section: SettingsSection::Laser,
            laser_draft,
        });
    }

    /// Übernimmt den GUI-Entwurf: klemmen, speichern, anwenden. Bei Erfolg true
    /// (Dialog schließen); bei Schreibfehler bleibt der Dialog offen und der
    /// Fehler erscheint im zentralen Kanal.
    pub fn commit_settings_dialog(&mut self) -> bool {
        let Some(st) = self.settings_dialog.as_ref() else {
            return false;
        };
        let mut draft = st.draft.clone();
        draft.sanitize();
        if let Err(error) = draft.save() {
            self.app_error = Some(AppError::new(
                "settings_write",
                format!("Einstellungen speichern fehlgeschlagen: {error}"),
            ));
            return false;
        }
        // Rasterweite steckt im gecachten Basis-Vertexpuffer → neu aufbauen.
        if draft.grid_size_mm != self.ui_settings.grid_size_mm {
            self.renderer.invalidate_scene();
        }
        self.ui_settings = draft;
        self.toasts.success("Einstellungen gespeichert.");
        true
    }

    /// Speichert den Laser-Profil-Entwurf aus der Laser-Sektion. Ein neues
    /// Profil wird aktiv, wenn noch keins aktiv war (wie bisher).
    pub fn settings_laser_save(&mut self) {
        let Some(profile) = self
            .settings_dialog
            .as_mut()
            .and_then(|st| st.laser_draft.take())
        else {
            return;
        };
        let is_new = profile.id.is_empty();
        self.laser_backend.save_profile(profile);
        if is_new && self.laser_backend.active_profile().is_none() {
            if let Some(profile) = self.laser_backend.registry.profiles.last() {
                let id = profile.id.clone();
                self.laser_backend.set_active(&id);
            }
        }
        self.toasts.success("Laser-Profil gespeichert.");
    }

    /// Löscht ein Laser-Profil aus der Laser-Sektion.
    pub fn settings_laser_delete(&mut self, id: &str) {
        self.laser_backend.delete_profile(id);
        // Ein Entwurf zum gelöschten Profil wäre verwaist — verwerfen.
        if let Some(st) = self.settings_dialog.as_mut() {
            if st.laser_draft.as_ref().is_some_and(|p| p.id == id) {
                st.laser_draft = None;
            }
        }
        self.toasts.success("Laser-Profil gelöscht.");
    }
}
