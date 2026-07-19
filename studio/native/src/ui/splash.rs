//! Start-Splash als egui-Overlay — Portierung der Tauri-Glaskarte
//! (splash.html): dunkler Verlauf mit orange/blauem Glow, Glanz-Sweep,
//! pulsierend leuchtendes Logo, Farbverlaufs-Schriftzug und git-Version.
//! Reiner Präsentationszustand; Sichtbarkeit/Dauer kommen aus den
//! GUI-Settings (`show_splash`/`splash_ms`), Klick/Taste überspringt
//! (das Verschlucken der Events macht der App-Root).

use std::time::Instant;

use egui::{Align2, Color32, FontId, Mesh, Pos2, Rect, Stroke};

/// Einblendzeit (Aufsteigen der Karte) und Ausblendzeit in Sekunden.
const RISE_S: f32 = 0.4;
const FADE_S: f32 = 0.3;
/// Kartengröße (Logo links, Text rechts — wie die Tauri-Karte).
const CARD: egui::Vec2 = egui::vec2(560.0, 260.0);

pub struct Splash {
    start: Instant,
    logo: Option<egui::TextureHandle>,
}

/// Weiche S-Kurve.
fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Farbe mit skaliertem Alpha (für das globale Ein-/Ausblenden).
fn faded(color: Color32, alpha: f32) -> Color32 {
    let a = (color.a() as f32 * alpha).round() as u8;
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), a)
}

impl Splash {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            logo: None,
        }
    }

    /// Zeichnet den Splash über allem. Liefert false, wenn er abgelaufen ist —
    /// der Root wirft ihn dann weg.
    pub fn show(&mut self, ui: &mut egui::Ui, min_ms: u32) -> bool {
        let elapsed = self.start.elapsed().as_secs_f32();
        let total = (min_ms as f32 / 1000.0).max(RISE_S + FADE_S);
        if elapsed >= total {
            return false;
        }
        // Globales Alpha: aufsteigen am Anfang, ausblenden am Ende.
        let a_in = ease(elapsed / RISE_S);
        let a_out = ease((total - elapsed) / FADE_S);
        let alpha = a_in.min(a_out);
        let rise = (1.0 - a_in) * 14.0;

        // Tooltip-Ebene: liegt über Panels, Dialogen UND Toasts.
        let painter = ui.ctx().layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("splash"),
        ));
        let screen = ui.max_rect();
        // Backdrop: deckt die App ab, blendet mit.
        painter.rect_filled(
            screen,
            0.0,
            faded(Color32::from_rgb(0x0c, 0x0d, 0x10), alpha),
        );

        let card = Rect::from_center_size(screen.center() + egui::vec2(0.0, rise), CARD);
        let clip = painter.with_clip_rect(card);

        // Karten-Verlauf (160°: oben links heller, unten rechts fast schwarz).
        let mut bg = Mesh::default();
        let c = |r: u8, g: u8, b: u8| faded(Color32::from_rgb(r, g, b), alpha);
        bg.colored_vertex(card.left_top(), c(0x24, 0x28, 0x32));
        bg.colored_vertex(card.right_top(), c(0x1a, 0x1d, 0x25));
        bg.colored_vertex(card.right_bottom(), c(0x10, 0x12, 0x16));
        bg.colored_vertex(card.left_bottom(), c(0x17, 0x1a, 0x20));
        bg.add_triangle(0, 1, 2);
        bg.add_triangle(0, 2, 3);
        clip.add(egui::Shape::mesh(bg));

        // Radiale Glows: orange oben links, blau unten rechts.
        radial_glow(
            &clip,
            card.left_top() + egui::vec2(card.width() * 0.18, card.height() * 0.12),
            card.width() * 0.65,
            faded(Color32::from_rgba_unmultiplied(240, 150, 40, 41), alpha),
        );
        radial_glow(
            &clip,
            card.right_bottom(),
            card.width() * 0.6,
            faded(Color32::from_rgba_unmultiplied(60, 110, 200, 41), alpha),
        );

        // Glanz-Sweep: schräges helles Band, das alle 2,4 s durchläuft.
        let sweep_t = ((elapsed - 0.4).max(0.0) % 2.4) / 1.45;
        if sweep_t < 1.0 {
            let x = card.left() + (card.width() + 300.0) * sweep_t - 150.0;
            let skew = card.height() * 0.27;
            let band = 90.0;
            let mut sweep = Mesh::default();
            let white = |a: u8| faded(Color32::from_rgba_unmultiplied(255, 255, 255, a), alpha);
            let (top, bottom) = (card.top(), card.bottom());
            sweep.colored_vertex(Pos2::new(x + skew, top), white(0));
            sweep.colored_vertex(Pos2::new(x + skew + band * 0.6, top), white(20));
            sweep.colored_vertex(Pos2::new(x + skew + band, top), white(0));
            sweep.colored_vertex(Pos2::new(x, bottom), white(0));
            sweep.colored_vertex(Pos2::new(x + band * 0.6, bottom), white(20));
            sweep.colored_vertex(Pos2::new(x + band, bottom), white(0));
            sweep.add_triangle(0, 1, 4);
            sweep.add_triangle(0, 4, 3);
            sweep.add_triangle(1, 2, 5);
            sweep.add_triangle(1, 5, 4);
            clip.add(egui::Shape::mesh(sweep));
        }

        // Rahmen + oberer Licht-Rand („Glas").
        painter.rect_stroke(
            card,
            14.0,
            Stroke::new(
                1.0,
                faded(Color32::from_rgba_unmultiplied(255, 255, 255, 31), alpha),
            ),
            egui::StrokeKind::Inside,
        );
        let shine = Rect::from_min_max(
            Pos2::new(card.left() + card.width() * 0.12, card.top()),
            Pos2::new(card.right() - card.width() * 0.12, card.top() + 1.0),
        );
        painter.rect_filled(
            shine,
            0.0,
            faded(Color32::from_rgba_unmultiplied(255, 255, 255, 90), alpha),
        );

        // Logo links, mit pulsierendem orangem Glow dahinter.
        let logo_rect = Rect::from_center_size(
            Pos2::new(card.left() + 140.0, card.center().y),
            egui::vec2(168.0, 160.0),
        );
        let pulse = 0.5 + 0.5 * (elapsed * 2.1).sin();
        radial_glow(
            &clip,
            logo_rect.center(),
            150.0 + 20.0 * pulse,
            faded(
                Color32::from_rgba_unmultiplied(240, 150, 40, (70.0 + 60.0 * pulse) as u8),
                alpha,
            ),
        );
        let texture = self.logo.get_or_insert_with(|| load_logo(ui.ctx()));
        painter.image(
            texture.id(),
            logo_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            faded(Color32::WHITE, alpha),
        );

        // Schriftzug: Farbverlauf weiß → warm → orange über gestaffelte
        // Clip-Bereiche (egui kennt keinen Text-Gradient).
        let text_pos = Pos2::new(card.left() + 250.0, card.center().y - 78.0);
        let font = FontId::proportional(44.0);
        let studio_name = studio_core::branding::STUDIO_NAME;
        let name = studio_name.rsplit_once(' ').map_or_else(
            || studio_name.to_owned(),
            |(brand, role)| format!("{brand}\n{role}"),
        );
        let steps = [
            (0.0, Color32::WHITE),
            (0.45, Color32::from_rgb(0xff, 0xd9, 0xa8)),
            (0.70, Color32::from_rgb(0xff, 0x9a, 0x3c)),
        ];
        let text_width = 200.0;
        for (from, color) in steps {
            let region = Rect::from_min_max(
                Pos2::new(text_pos.x + text_width * from, card.top()),
                card.right_bottom(),
            );
            painter.with_clip_rect(region.intersect(card)).text(
                text_pos,
                Align2::LEFT_TOP,
                &name,
                font.clone(),
                faded(color, alpha),
            );
        }
        painter.text(
            text_pos + egui::vec2(2.0, 108.0),
            Align2::LEFT_TOP,
            "Laser Studio",
            FontId::proportional(13.0),
            faded(Color32::from_rgba_unmultiplied(255, 255, 255, 102), alpha),
        );
        let commit = env!("STUDIO_COMMIT");
        let version = if commit == "-" {
            env!("STUDIO_VERSION").to_string()
        } else {
            format!("{} · {}", env!("STUDIO_VERSION"), commit)
        };
        painter.text(
            text_pos + egui::vec2(2.0, 134.0),
            Align2::LEFT_TOP,
            version,
            FontId::proportional(12.5),
            faded(Color32::from_rgba_unmultiplied(255, 255, 255, 140), alpha),
        );

        // Animation läuft — weiterzeichnen, bis der Splash vorbei ist.
        ui.request_repaint();
        true
    }
}

/// Weicher radialer Glow als Fächer-Mesh: Zentrum in `color`, Rand transparent.
fn radial_glow(painter: &egui::Painter, center: Pos2, radius: f32, color: Color32) {
    const SEGMENTS: u32 = 40;
    let mut mesh = Mesh::default();
    mesh.colored_vertex(center, color);
    for i in 0..=SEGMENTS {
        let angle = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        mesh.colored_vertex(
            center + radius * egui::vec2(angle.cos(), angle.sin()),
            Color32::TRANSPARENT,
        );
    }
    for i in 1..=SEGMENTS {
        mesh.add_triangle(0, i, i + 1);
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// Lädt das eingebettete Logo-PNG als egui-Textur (einmalig, lazy).
fn load_logo(ctx: &egui::Context) -> egui::TextureHandle {
    let bytes = include_bytes!("../../assets/splash-logo.png");
    let image = image::load_from_memory(bytes)
        .map(|img| img.to_rgba8())
        .unwrap_or_else(|_| image::RgbaImage::new(1, 1));
    let size = [image.width() as usize, image.height() as usize];
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
    ctx.load_texture("splash-logo", color_image, egui::TextureOptions::LINEAR)
}
