//! Bild-Layer-Parameter (ADR 0004): nicht-destruktive Verarbeitungsparameter
//! eines Image-Shapes setzen. Validiert die Wertebereiche und erzeugt genau
//! einen Undo-Schritt; das Store-Asset bleibt unberührt.

use luxifer_core::{Geo, ImageParams};

use crate::AppError;

use super::EditorSession;

impl EditorSession {
    /// Ersetzt das Bild durch ein abgeleitetes Crop-Asset und passt seine
    /// Box an den sichtbaren Ausschnitt an. Ein einzelner Undo-Schritt.
    pub fn crop_image(
        &mut self,
        index: usize,
        asset: String,
        crop: [f32; 4],
    ) -> Result<(), AppError> {
        if asset.trim().is_empty()
            || crop.iter().any(|value| !value.is_finite())
            || crop[0] < 0.0
            || crop[1] < 0.0
            || crop[2] > 1.0
            || crop[3] > 1.0
            || crop[2] - crop[0] < 0.01
            || crop[3] - crop[1] < 0.01
        {
            return Err(AppError::new(
                "image_crop_bounds",
                "Der Bildausschnitt ist zu klein oder ungültig.",
            ));
        }
        if !matches!(
            self.state.shapes.get(index).map(|shape| &shape.geo),
            Some(Geo::Image { .. })
        ) {
            return Err(Self::no_image_shape(index));
        }
        self.state.push_undo();
        if let Geo::Image {
            asset: current,
            x,
            y,
            w,
            h,
            ..
        } = &mut self.state.shapes[index].geo
        {
            let (old_x, old_y, old_w, old_h) = (*x, *y, *w, *h);
            *current = asset;
            *x = old_x + old_w * crop[0] as f64;
            *y = old_y + old_h * crop[1] as f64;
            *w = old_w * (crop[2] - crop[0]) as f64;
            *h = old_h * (crop[3] - crop[1]) as f64;
        }
        self.state.dirty = true;
        Ok(())
    }

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

    /// Vektorisiert ein Bild-Shape (Trace): Konturen des Motivs als
    /// geschlossene Polylinien in mm auf dem aktiven Zeichen-Layer, genau ein
    /// Undo-Schritt. Die Tonwert-LUT des Bildes (Helligkeit/Kontrast/Gamma)
    /// wirkt vor der Schwelle — was der Nutzer eingestellt hat, wird getract.
    pub fn trace_image(
        &mut self,
        index: usize,
        threshold: u8,
        invert: bool,
    ) -> Result<Vec<usize>, AppError> {
        let params = match self.state.shapes.get(index).map(|shape| &shape.geo) {
            Some(Geo::Image { params, .. }) => *params,
            _ => return Err(Self::no_image_shape(index)),
        };
        self.trace_image_with_params(index, params, threshold, invert)
    }

    /// Trace mit einem noch nicht gespeicherten Bildparameter-Entwurf. Damit
    /// entspricht das erzeugte Ergebnis exakt der Live-Vorschau im Dialog.
    pub fn trace_image_with_params(
        &mut self,
        index: usize,
        draft_params: ImageParams,
        threshold: u8,
        invert: bool,
    ) -> Result<Vec<usize>, AppError> {
        use luxifer_core::trace::{trace, TraceParams};
        use luxifer_core::ImageMode;

        Self::validate_image_params(&draft_params)?;

        let Some(Geo::Image {
            asset, x, y, w, h, ..
        }) = self.state.shapes.get(index).map(|s| &s.geo)
        else {
            return Err(Self::no_image_shape(index));
        };
        let (asset, bx, by, bw, bh) = (asset.clone(), *x, *y, *w, *h);

        let (px, pw, ph) = luxifer_core::load_asset_luma(&luxifer_core::assets_dir(), &asset)
            .map_err(|e| {
                AppError::wrap(
                    "asset_read",
                    "Bild-Asset konnte nicht geladen werden.",
                    e.to_string(),
                )
            })?;
        // Nur die Tonwert-LUT anwenden (kein Dithering), dann tracen.
        let lut = ImageParams {
            mode: ImageMode::Grayscale,
            ..draft_params
        };
        let gray = luxifer_core::apply_params(&px, &lut, false);
        let contours = trace(
            &gray,
            pw as usize,
            ph as usize,
            &TraceParams {
                threshold,
                invert,
                ..Default::default()
            },
        );
        if contours.is_empty() {
            return Err(AppError::new(
                "trace_empty",
                "Keine Konturen gefunden — Schwelle anpassen?",
            ));
        }
        // Pixel → mm über die Bildbox.
        let (sx, sy) = (bw / pw as f64, bh / ph as f64);
        let mm: Vec<(Vec<(f64, f64)>, bool)> = contours
            .into_iter()
            .map(|c| {
                (
                    c.into_iter()
                        .map(|(px, py)| (bx + px * sx, by + py * sy))
                        .collect(),
                    true,
                )
            })
            .collect();
        Ok(self.state.add_polylines(mm))
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
