//! Asset-Auflösung für Jobplanung und Vorschau: liefert dem Core die
//! Graustufen-Pixel eines Bild-Assets aus dem Store.
//!
//! Genau EINE Quelle für Vorschau UND echten Job/Export — die Vorschau darf
//! nichts zeigen, was der Job nicht tut (und umgekehrt). Fehlende oder
//! unlesbare Assets werden übersprungen (der Core lässt den Bild-Layer dann
//! leer); der Fehler wird auf stderr protokolliert, damit er nicht stumm
//! verschwindet.

use std::borrow::Cow;

/// Graustufen-Pixel (row-major `u8`) samt Pixelmaßen zu einer Asset-ID, im
/// Format des `JobPlan::from_shapes_with_assets`-Resolvers.
pub(crate) fn resolve_luma(id: &str) -> Option<(Cow<'static, [u8]>, usize, usize)> {
    let dir = luxifer_core::assets_dir();
    match luxifer_core::load_asset_luma(&dir, &id.to_string()) {
        Ok((pixels, w, h)) => Some((Cow::Owned(pixels), w as usize, h as usize)),
        Err(e) => {
            eprintln!("Bild-Asset {id} für den Job nicht ladbar: {e}");
            None
        }
    }
}
