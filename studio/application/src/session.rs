mod actions;
mod drawing;
mod images;
mod layers;
mod preview;
mod selection;

pub use layers::LayerParams;

use std::ops::Deref;

use studio_core::AppState;

use crate::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxShape {
    Rect,
    Ellipse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointPath {
    Polyline,
    Spline,
    Bezier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerToggle {
    Visible,
    Enabled,
    Locked,
    AirAssist,
}

/// Laufende, UI-unabhängige Editor-Sitzung.
///
/// `Deref`/`DerefMut` sind eine bewusst vorübergehende Migrationsbrücke für
/// noch nicht extrahierte Native-Abläufe. Neue Anwendungsfälle erhalten
/// benannte Methoden in den verantwortlichen Session-Modulen.
#[derive(Debug, Default)]
pub struct EditorSession {
    state: AppState,
    edit_start: Option<AppState>,
}

impl EditorSession {
    pub fn new(state: AppState) -> Self {
        Self {
            state,
            edit_start: None,
        }
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    /// Explizite Migrationsbruecke fuer bestehende Tests und Datenmigrationen.
    /// Produktive GUI-Ablaufe duerfen ausschliesslich benannte Session-Methoden
    /// verwenden. Im Gegensatz zum frueheren `DerefMut` ist jeder Restzugriff
    /// auffindbar und kann schrittweise entfernt werden.
    #[doc(hidden)]
    pub fn state_mut_for_migration(&mut self) -> &mut AppState {
        &mut self.state
    }

    pub fn replace_state(&mut self, state: AppState) -> AppState {
        self.edit_start = None;
        std::mem::replace(&mut self.state, state)
    }

    /// Ob ungespeicherte Änderungen vorliegen (für den Dirty-Guard).
    pub fn is_dirty(&self) -> bool {
        self.state.dirty
    }

    /// Nach erfolgreichem Speichern: der Zustand gilt als gesichert.
    pub fn mark_saved(&mut self) {
        self.state.mark_saved();
    }

    pub(super) fn require_selection(&self, action: &str) -> Result<(), AppError> {
        if self.state.selected.is_empty() {
            Err(AppError::new(
                "selection_required",
                format!("Für „{action}“ muss mindestens ein Objekt ausgewählt sein."),
            ))
        } else {
            Ok(())
        }
    }

    pub fn delete_selected(&mut self) -> Result<(), AppError> {
        if self.state.selected.is_empty() {
            return Err(AppError::new(
                "selection_required",
                "Zum Löschen muss mindestens ein Objekt ausgewählt sein.",
            ));
        }
        self.state.delete_selected();
        Ok(())
    }

    pub fn undo(&mut self) -> bool {
        self.state.undo()
    }

    pub fn redo(&mut self) -> bool {
        self.state.redo()
    }

    /// Aktualisiert die Arbeitsflaeche aus einem Maschinenprofil. Die GUI kennt
    /// dadurch weder den mutierbaren Core-Zustand noch dessen interne Felder.
    pub fn set_bed_size(&mut self, width_mm: f64, height_mm: f64) {
        self.state.bed_w_mm = width_mm;
        self.state.bed_h_mm = height_mm;
    }

    /// Interner Anwendungsfall fuer den Fill-Diagnosemodus der nativen Ansicht.
    pub fn toggle_vector_fill_modes(&mut self) {
        let any_cut = self
            .state
            .layers
            .iter()
            .any(|layer| layer.mode == studio_core::LayerMode::Cut);
        let target = if any_cut {
            studio_core::LayerMode::Fill
        } else {
            studio_core::LayerMode::Cut
        };
        for layer in &mut self.state.layers {
            if matches!(
                layer.mode,
                studio_core::LayerMode::Cut | studio_core::LayerMode::Fill
            ) {
                layer.mode = target;
            }
        }
    }
}

impl Deref for EditorSession {
    type Target = AppState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

#[cfg(test)]
mod tests;
