//! Nativer Text-Workflow: Dialog-Draft, Font-Auflösung und Übergabe der
//! erzeugten Konturen an die Application-Session.

use super::App;
use crate::ui::TextDialogState;

impl App {
    pub fn open_text_dialog(&mut self) {
        if self.fonts.is_empty() {
            self.fonts = crate::fonts::list_fonts();
        }
        let mut state = TextDialogState::default();
        if !self.fonts.is_empty() {
            state.font_idx = Some(0);
        }
        self.text_dialog = Some(state);
    }

    pub fn open_text_editor(&mut self, index: usize) {
        if self.fonts.is_empty() {
            self.fonts = crate::fonts::list_fonts();
        }
        let Some(meta) = self
            .session
            .state()
            .shapes
            .get(index)
            .and_then(|shape| shape.text_meta.clone())
        else {
            return;
        };
        let font_idx = self
            .fonts
            .iter()
            .position(|font| font.path.to_string_lossy() == meta.font_path)
            .or((!self.fonts.is_empty()).then_some(0));
        self.text_dialog = Some(TextDialogState {
            text: meta.text,
            size_mm: meta.size_mm,
            font_idx,
            edit_index: Some(index),
        });
    }

    pub fn commit_text(&mut self) -> bool {
        let Some(state) = self.text_dialog.as_ref() else {
            return false;
        };
        let Some(font_idx) = state.font_idx else {
            self.laser_msg = "Kein Font gewählt".into();
            return false;
        };
        let Some(font) = self.fonts.get(font_idx) else {
            return false;
        };
        let (text, size, edit_index, font_path) = (
            state.text.clone(),
            state.size_mm,
            state.edit_index,
            font.path.clone(),
        );
        let font_data = match std::fs::read(&font_path) {
            Ok(data) => data,
            Err(error) => {
                self.laser_msg = format!("Font lesen: {error}");
                return false;
            }
        };
        match luxifer_core::text::text_to_contours(&font_data, &text, size) {
            Ok(contours) if !contours.is_empty() => {
                let meta = luxifer_core::TextMeta {
                    text,
                    font_path: font_path.to_string_lossy().to_string(),
                    size_mm: size,
                };
                if let Some(index) = edit_index {
                    if let Err(error) = self.session.replace_text_block(index, contours, meta) {
                        self.app_error = Some(error);
                        return false;
                    }
                } else {
                    self.session.selected = self.session.add_text_block(contours, meta);
                }
                self.refresh_accent();
                self.fit_all();
                true
            }
            Ok(_) => {
                self.laser_msg = "Text ergab keine Konturen".into();
                false
            }
            Err(error) => {
                self.laser_msg = format!("Text-Fehler: {error}");
                false
            }
        }
    }
}
