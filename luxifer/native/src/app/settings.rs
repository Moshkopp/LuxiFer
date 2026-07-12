//! GUI-Einstellungen-Workflow (ADR 0002): Dialog-Entwurf öffnen/übernehmen.
//! Validierung/Klemmen und Persistenz (gui-settings.json) macht der Core
//! (`UiSettings::sanitize`/`save`); native wendet nur an (Theme, Raster).

use luxifer_application::AppError;

use super::App;

impl App {
    /// Öffnet den Einstellungen-Dialog mit den aktuellen Werten als Entwurf.
    pub fn open_settings_dialog(&mut self) {
        self.settings_dialog = Some(crate::ui::SettingsDialogState {
            draft: self.ui_settings.clone(),
        });
    }

    /// Übernimmt den Entwurf: klemmen, speichern, anwenden. Bei Erfolg true
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
}
