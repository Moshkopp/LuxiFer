//! Toast-Meldungen oben rechts: fahren vom rechten Rand herein, stehen kurz
//! und fahren wieder hinaus. Reiner Präsentationszustand — kurze Erfolgs-
//! (grün) und Fehler-/Warnmeldungen (rot) der Workflows; schwere Fehler laufen
//! weiterhin zusätzlich über den Banner (`app_error`), der stehen bleibt, bis
//! der Nutzer ihn schließt.

use std::time::Instant;

use egui::{Align2, Color32, RichText, Rounding, Stroke};

/// Phasen der Lebensdauer in Sekunden.
const SLIDE_IN: f32 = 0.25;
const HOLD: f32 = 3.5;
const SLIDE_OUT: f32 = 0.30;
/// Maximale Textbreite; zugleich die Strecke, die ein Toast hereinfährt.
const WIDTH: f32 = 380.0;
/// Abstand zum Fensterrand und zwischen gestapelten Toasts.
const MARGIN: f32 = 12.0;
/// Textgröße — bewusst größer als der Panel-Standard, Toasts sind flüchtig.
const TEXT_SIZE: f32 = 16.0;

#[derive(Clone, Copy)]
enum ToastKind {
    Success,
    Error,
}

impl ToastKind {
    /// Signalfarbe (Punkt + Randton). Grün/Rot passend zum dunklen Theme;
    /// das Rot entspricht dem Fehler-Banner.
    fn color(self) -> Color32 {
        match self {
            ToastKind::Success => Color32::from_rgb(0x4a, 0xde, 0x80),
            ToastKind::Error => Color32::from_rgb(0xf8, 0x71, 0x71),
        }
    }
}

struct Toast {
    text: String,
    kind: ToastKind,
    born: Instant,
    /// Stabile egui-Id, damit ein Toast beim Ablauf seiner Vorgänger nicht
    /// die Identität wechselt.
    id: u64,
}

#[derive(Default)]
pub struct Toasts {
    items: Vec<Toast>,
    next_id: u64,
}

/// Weiche S-Kurve (smoothstep) für Ein-/Ausfahren.
fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Toasts {
    /// Grüner Erfolgs-/Statustoast.
    pub fn success(&mut self, text: impl Into<String>) {
        self.push(text, ToastKind::Success);
    }

    /// Roter Fehler-/Warntoast (für leichte Fehler, die keinen Banner brauchen).
    pub fn error(&mut self, text: impl Into<String>) {
        self.push(text, ToastKind::Error);
    }

    fn push(&mut self, text: impl Into<String>, kind: ToastKind) {
        self.items.push(Toast {
            text: text.into(),
            kind,
            born: Instant::now(),
            id: self.next_id,
        });
        self.next_id = self.next_id.wrapping_add(1);
    }

    /// Zeichnet alle aktiven Toasts (Aufruf am Ende von `ui::build`, damit sie
    /// über den Panels liegen) und entfernt abgelaufene.
    pub fn show(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        self.items
            .retain(|t| now.duration_since(t.born).as_secs_f32() < SLIDE_IN + HOLD + SLIDE_OUT);
        if self.items.is_empty() {
            return;
        }

        // Unter der Topbar beginnen (available_rect klammert die Panels aus),
        // horizontal aber am echten Fensterrand hereinfahren.
        let top = ctx.available_rect().top() + MARGIN;
        let right = ctx.screen_rect().right() - MARGIN;

        // Panel-Fläche des Themes (apply_theme), leicht durchscheinend.
        let fill = Color32::from_rgba_unmultiplied(0x1c, 0x1f, 0x26, 0xf0);

        let mut y = top;
        for t in &self.items {
            let age = now.duration_since(t.born).as_secs_f32();
            // 0 = ganz drin, 1 = ganz draußen.
            let out = if age < SLIDE_IN {
                1.0 - ease(age / SLIDE_IN)
            } else if age > SLIDE_IN + HOLD {
                ease((age - SLIDE_IN - HOLD) / SLIDE_OUT)
            } else {
                0.0
            };
            let x = right + out * (WIDTH + 2.0 * MARGIN);
            let color = t.kind.color();

            let response = egui::Area::new(egui::Id::new(("toast", t.id)))
                .order(egui::Order::Foreground)
                .interactable(false)
                .pivot(Align2::RIGHT_TOP)
                .fixed_pos(egui::pos2(x, y))
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(fill)
                        .stroke(Stroke::new(1.5, color.gamma_multiply(0.6)))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(egui::Margin::symmetric(16.0, 12.0))
                        .show(ui, |ui| {
                            ui.set_max_width(WIDTH);
                            ui.horizontal(|ui| {
                                // Signalpunkt in der Statusfarbe statt Icon.
                                let (dot, _) = ui.allocate_exact_size(
                                    egui::vec2(12.0, 12.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().circle_filled(dot.center(), 6.0, color);
                                ui.label(RichText::new(&t.text).size(TEXT_SIZE));
                            });
                        });
                })
                .response;
            y += response.rect.height() + 8.0;
        }
        // Animation läuft — bis alle Toasts weg sind weiterzeichnen.
        ctx.request_repaint();
    }
}
