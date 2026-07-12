use std::ops::{Deref, DerefMut};

use luxifer_core::AppState;

use crate::AppError;

/// Laufende, UI-unabhängige Editor-Sitzung.
///
/// `Deref`/`DerefMut` sind eine bewusst vorübergehende Migrationsbrücke für
/// noch nicht extrahierte Native-Abläufe. Neue Anwendungsfälle erhalten
/// benannte Methoden auf `EditorSession`; der Direktzugriff wird mit jedem
/// vertikalen Schnitt kleiner und am Ende entfernt.
#[derive(Debug, Default)]
pub struct EditorSession {
    state: AppState,
}

impl EditorSession {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn state_mut_for_migration(&mut self) -> &mut AppState {
        &mut self.state
    }

    pub fn replace_state(&mut self, state: AppState) -> AppState {
        std::mem::replace(&mut self.state, state)
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
}

impl Deref for EditorSession {
    type Target = AppState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl DerefMut for EditorSession {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

#[cfg(test)]
mod tests {
    use luxifer_core::Geo;

    use super::*;

    fn session_with_rect() -> EditorSession {
        let mut state = AppState::new();
        state.add_shape(Geo::Rect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
        });
        EditorSession::new(state)
    }

    #[test]
    fn loeschen_ohne_auswahl_liefert_stabilen_fehler_ohne_mutation() {
        let mut session = EditorSession::default();

        let error = session.delete_selected().unwrap_err();

        assert_eq!(error.code(), "selection_required");
        assert!(session.shapes.is_empty());
        assert!(!session.dirty);
    }

    #[test]
    fn loeschen_undo_und_redo_bleiben_ein_zusammenhaengender_ablauf() {
        let mut session = session_with_rect();

        session.delete_selected().unwrap();
        assert!(session.shapes.is_empty());
        assert!(session.dirty);

        assert!(session.undo());
        assert_eq!(session.shapes.len(), 1);
        assert_eq!(session.selected, vec![0]);

        assert!(session.redo());
        assert!(session.shapes.is_empty());
        assert!(session.selected.is_empty());
    }

    #[test]
    fn undo_und_redo_ohne_historie_sind_sichere_no_ops() {
        let mut session = EditorSession::default();

        assert!(!session.undo());
        assert!(!session.redo());
        assert!(!session.dirty);
    }
}
