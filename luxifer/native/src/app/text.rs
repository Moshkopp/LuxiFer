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
            request_font_import: false,
        });
    }

    /// Importiert eine Font-Datei (TTF/OTF) in den Asset-Katalog und wählt sie
    /// im offenen Text-Dialog aus. Der Katalog liegt vor den System-Fonts,
    /// damit importierte Fonts auch ohne Systeminstallation verfügbar bleiben.
    pub fn import_font_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Font (TTF/OTF)", &["ttf", "otf"])
            .pick_file()
        else {
            return;
        };
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.toasts.error(format!("Font lesen: {error}"));
                return;
            }
        };
        // Vor dem Ablegen prüfen, ob die Datei überhaupt ein brauchbarer Font
        // ist — sonst landet Datenmüll dauerhaft im Katalog.
        if let Err(error) = luxifer_core::text::text_to_contours(&bytes, "Ag", 20.0) {
            self.toasts.error(format!("Font unbrauchbar: {error}"));
            return;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("font.ttf");
        let ext = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("ttf");
        let meta = match luxifer_core::import_source(
            &luxifer_core::assets_dir(),
            &bytes,
            name,
            ext,
            luxifer_core::AssetKind::Font,
        ) {
            Ok(meta) => meta,
            Err(error) => {
                self.toasts.error(format!("Font importieren: {error}"));
                return;
            }
        };
        self.refresh_asset_catalog();
        self.fonts = crate::fonts::list_fonts();
        // Den frisch importierten Font direkt auswählen (Anzeigename = Stem des
        // Originalnamens, wie in `list_fonts`).
        let stem = std::path::Path::new(&meta.original_name)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(&meta.original_name)
            .to_string();
        let idx = self.fonts.iter().position(|font| font.name == stem);
        if let Some(state) = self.text_dialog.as_mut() {
            state.font_idx = idx.or(state.font_idx);
        }
        self.toasts.success(format!("Font „{stem}“ importiert"));
    }

    pub fn commit_text(&mut self) -> bool {
        let Some(state) = self.text_dialog.as_ref() else {
            return false;
        };
        let Some(font_idx) = state.font_idx else {
            self.toasts.error("Kein Font gewählt");
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
                self.toasts.error(format!("Font lesen: {error}"));
                return false;
            }
        };
        let font_name = font_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("font.ttf");
        let font_ext = font_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("ttf");
        let font_asset = match luxifer_core::import_source(
            &luxifer_core::assets_dir(),
            &font_data,
            font_name,
            font_ext,
            luxifer_core::AssetKind::Font,
        ) {
            Ok(meta) => Some(meta.id),
            Err(error) => {
                self.toasts.error(format!("Font katalogisieren: {error}"));
                return false;
            }
        };
        self.refresh_asset_catalog();
        match luxifer_core::text::text_to_contours(&font_data, &text, size) {
            Ok(contours) if !contours.is_empty() => {
                let meta = luxifer_core::TextMeta {
                    text,
                    font_path: font_path.to_string_lossy().to_string(),
                    font_asset,
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
                self.toasts.error("Text ergab keine Konturen");
                false
            }
            Err(error) => {
                self.toasts.error(format!("Text-Fehler: {error}"));
                false
            }
        }
    }
}
