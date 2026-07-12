//! Bild-Layer-Parameter (ADR 0004): nicht-destruktive Verarbeitungsparameter
//! eines Image-Shapes setzen. Validiert die Wertebereiche und erzeugt genau
//! einen Undo-Schritt; das Store-Asset bleibt unberührt.

use luxifer_core::{Geo, ImageParams};

use crate::AppError;

use super::EditorSession;

impl EditorSession {
    /// Setzt die Bildverarbeitungs-Parameter eines Image-Shapes in genau einem
    /// Undo-Schritt. Fehlerfälle (kein solcher Index, kein Bild-Shape, ungültige
    /// Werte) mutieren nichts.
    pub fn set_image_params(&mut self, index: usize, params: ImageParams) -> Result<(), AppError> {
        let shape = self
            .state
            .shapes
            .get(index)
            .ok_or_else(|| Self::no_image_shape(index))?;
        if !matches!(shape.geo, Geo::Image { .. }) {
            return Err(Self::no_image_shape(index));
        }
        Self::validate_image_params(&params)?;

        self.state.push_undo();
        if let Geo::Image { params: p, .. } = &mut self.state.shapes[index].geo {
            *p = params;
        }
        self.state.dirty = true;
        Ok(())
    }

    fn validate_image_params(params: &ImageParams) -> Result<(), AppError> {
        if !(0.1..=3.0).contains(&params.gamma) || !params.gamma.is_finite() {
            return Err(AppError::new(
                "image_gamma",
                "Gamma muss zwischen 0.1 und 3.0 liegen.",
            ));
        }
        if !(-100..=100).contains(&params.brightness) {
            return Err(AppError::new(
                "image_brightness",
                "Helligkeit muss zwischen -100 und +100 liegen.",
            ));
        }
        if !(-100..=100).contains(&params.contrast) {
            return Err(AppError::new(
                "image_contrast",
                "Kontrast muss zwischen -100 und +100 liegen.",
            ));
        }
        Ok(())
    }

    fn no_image_shape(index: usize) -> AppError {
        AppError::new(
            "not_an_image",
            format!("An Position {index} liegt kein Bild-Objekt."),
        )
    }
}
