//! Welt↔Bildschirm-Transform. Welt = mm (wie im Core), Bildschirm = Pixel.
//! Y zeigt in Welt- wie Bildschirmkoordinaten nach unten (Bildkonvention).

/// Kamera für den Canvas: Welt-Zentrum + Zoom (Pixel pro mm).
#[derive(Clone, Copy)]
pub struct Camera {
    /// Welt-Punkt (mm), der in der Viewport-Mitte liegt.
    pub center: [f32; 2],
    /// Pixel pro mm.
    pub scale: f32,
    /// Viewport-Größe in Pixeln.
    pub viewport: [f32; 2],
}

impl Camera {
    pub fn new() -> Self {
        Self {
            center: [0.0, 0.0],
            scale: 2.0,
            viewport: [1280.0, 800.0],
        }
    }

    /// Welt (mm) → Bildschirm (px, Ursprung oben links).
    pub fn world_to_screen(&self, w: [f64; 2]) -> [f32; 2] {
        let x = (w[0] as f32 - self.center[0]) * self.scale + self.viewport[0] * 0.5;
        let y = (w[1] as f32 - self.center[1]) * self.scale + self.viewport[1] * 0.5;
        [x, y]
    }

    /// Bildschirm (px) → Welt (mm).
    pub fn screen_to_world(&self, s: [f32; 2]) -> [f64; 2] {
        let x = (s[0] - self.viewport[0] * 0.5) / self.scale + self.center[0];
        let y = (s[1] - self.viewport[1] * 0.5) / self.scale + self.center[1];
        [x as f64, y as f64]
    }

    /// Panning um einen Pixel-Delta (Maus-Drag).
    pub fn pan_pixels(&mut self, dx: f32, dy: f32) {
        self.center[0] -= dx / self.scale;
        self.center[1] -= dy / self.scale;
    }

    /// Zoom um einen Faktor, wobei der Welt-Punkt unter `pivot_px` fix bleibt.
    pub fn zoom_at(&mut self, factor: f32, pivot_px: [f32; 2]) {
        let before = self.screen_to_world(pivot_px);
        self.scale = (self.scale * factor).clamp(0.02, 2000.0);
        let after = self.screen_to_world(pivot_px);
        // Zentrum so verschieben, dass der Pivot-Weltpunkt unter dem Cursor bleibt.
        self.center[0] += (before[0] - after[0]) as f32;
        self.center[1] += (before[1] - after[1]) as f32;
    }

    /// Kamera so setzen, dass die Welt-BBox (x, y, w, h) mit Rand einpasst.
    pub fn fit_bbox(&mut self, bbox: [f64; 4], margin: f32) {
        let (bx, by, bw, bh) = (
            bbox[0] as f32,
            bbox[1] as f32,
            bbox[2] as f32,
            bbox[3] as f32,
        );
        self.center = [bx + bw * 0.5, by + bh * 0.5];
        let sx = self.viewport[0] / bw.max(1.0);
        let sy = self.viewport[1] / bh.max(1.0);
        self.scale = (margin * sx.min(sy)).clamp(0.02, 2000.0);
    }
}
