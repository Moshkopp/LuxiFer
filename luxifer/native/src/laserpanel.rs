//! Laser-Bedienpanel in egui, nach dem frischen Svelte-Design (ADR 0007 +
//! Redesign): Ampel-Grid (Start grün / Pause orange / Stopp rot / Ursprung blau /
//! Rahmen · Gummiband), Job-Parameter, Job-Nullpunkt-Anker (3×3), Jog-Kreuz +
//! Slider. Ohne echten Treiber-Anschluss im Umbau — Aktionen loggen vorerst.

use egui::{Color32, RichText, Sense, Vec2};

use crate::tools::LaserUi;

/// Farb-Ton der Ampel-Kacheln.
enum Tone {
    Go,
    Warn,
    Stop,
    Nav,
    Neutral,
}

fn tone_colors(t: &Tone) -> (Color32, Color32) {
    // (Füllung, Textfarbe)
    match t {
        Tone::Go => (
            Color32::from_rgb(0x2f, 0xa5, 0x6b),
            Color32::from_rgb(0xea, 0xff, 0xf5),
        ),
        Tone::Warn => (
            Color32::from_rgb(0xe0, 0x93, 0x00),
            Color32::from_rgb(0x24, 0x18, 0x00),
        ),
        Tone::Stop => (
            Color32::from_rgb(0xd2, 0x46, 0x3c),
            Color32::from_rgb(0xff, 0xf0, 0xee),
        ),
        Tone::Nav => (
            Color32::from_rgb(0x35, 0x6f, 0xb0),
            Color32::from_rgb(0xee, 0xf4, 0xff),
        ),
        Tone::Neutral => (
            Color32::from_rgb(0x2a, 0x2f, 0x38),
            Color32::from_rgb(0xec, 0xee, 0xf1),
        ),
    }
}

/// Zeichnet das Panel. Gibt geloggte Aktionen zurück (bis der Treiber dran ist).
pub fn show(ui: &mut egui::Ui, laser: &mut LaserUi) {
    // Kopf: Verbindungsstatus (Demo-Umschalter).
    ui.horizontal(|ui| {
        ui.label(RichText::new("LASER").small().weak());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (txt, col) = if laser.connected {
                ("verbunden", Color32::from_rgb(0x3f, 0xb2, 0x7f))
            } else {
                ("getrennt", Color32::from_gray(0x9a))
            };
            if ui.small_button(RichText::new(txt).color(col)).clicked() {
                laser.connected = !laser.connected;
            }
        });
    });
    ui.add_space(8.0);

    // Ampel-Grid 2×3.
    ui.label(RichText::new("JOB").small().weak());
    ui.add_space(4.0);
    let cells: [(&str, Tone); 6] = [
        ("Start", Tone::Go),
        ("Pause", Tone::Warn),
        ("Stopp", Tone::Stop),
        ("Ursprung", Tone::Nav),
        ("Rahmen", Tone::Neutral),
        ("Gummiband", Tone::Neutral),
    ];
    let avail = ui.available_width();
    let gap = 6.0;
    let cell_w = (avail - 2.0 * gap) / 3.0;
    let cell_h = cell_w * 0.72;
    egui::Grid::new("ampel")
        .spacing(Vec2::splat(gap))
        .show(ui, |ui| {
            for (i, (label, tone)) in cells.iter().enumerate() {
                ampel_cell(ui, label, tone, cell_w, cell_h);
                if i % 3 == 2 {
                    ui.end_row();
                }
            }
        });

    ui.add_space(8.0);
    ui.checkbox(&mut laser.selection_only, "Nur Auswahl lasern");

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    // Job-Nullpunkt-Anker (3×3, touch-taugliche Felder).
    ui.label(RichText::new("JOB-NULLPUNKT").small().weak());
    ui.add_space(4.0);
    anchor_grid(ui, &mut laser.anchor);

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(8.0);

    // Jog-Kreuz.
    ui.label(RichText::new("KOPF").small().weak());
    ui.add_space(4.0);
    jog_cross(ui);
    ui.add_space(8.0);

    // Slider für Schritt/Speed (Wert antippbar über DragValue).
    slider_row(ui, "Schritt", "mm", &mut laser.jog_step, 0.1, 100.0);
    slider_row(ui, "Speed", "mm/s", &mut laser.jog_speed, 1.0, 1000.0);
}

fn ampel_cell(ui: &mut egui::Ui, label: &str, tone: &Tone, w: f32, h: f32) {
    let (fill, text) = tone_colors(tone);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
    let bg = if resp.hovered() {
        fill.gamma_multiply(1.15)
    } else {
        fill
    };
    ui.painter().rect_filled(rect, 8.0, bg);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(13.0),
        text,
    );
    if resp.clicked() {
        log::info!("Laser-Aktion: {label}");
    }
}

fn anchor_grid(ui: &mut egui::Ui, anchor: &mut usize) {
    let size = 40.0;
    let gap = 5.0;
    egui::Grid::new("anchor")
        .spacing(Vec2::splat(gap))
        .show(ui, |ui| {
            for i in 0..9 {
                let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
                let on = *anchor == i;
                let bg = if on {
                    Color32::from_rgb(0x1e, 0x3a, 0x5f)
                } else {
                    Color32::from_black_alpha(64)
                };
                ui.painter().rect_filled(rect, 8.0, bg);
                let dot = if on {
                    Color32::from_rgb(0x3B, 0x82, 0xF6)
                } else {
                    Color32::from_gray(0x9a)
                };
                let r = if on { 5.0 } else { 3.5 };
                ui.painter().circle_filled(rect.center(), r, dot);
                if resp.clicked() {
                    *anchor = i;
                }
                if i % 3 == 2 {
                    ui.end_row();
                }
            }
        });
}

fn jog_cross(ui: &mut egui::Ui) {
    let b = 46.0;
    let gap = 5.0;
    let total = 3.0 * b + 2.0 * gap;
    ui.horizontal(|ui| {
        // Zentrieren.
        let pad = (ui.available_width() - total) * 0.5;
        if pad > 0.0 {
            ui.add_space(pad);
        }
        let (rect, _) = ui.allocate_exact_size(Vec2::new(total, total), Sense::hover());
        let cell = |col: usize, row: usize| -> egui::Rect {
            let x = rect.left() + col as f32 * (b + gap);
            let y = rect.top() + row as f32 * (b + gap);
            egui::Rect::from_min_size(egui::pos2(x, y), Vec2::splat(b))
        };
        // Richtung als selbstgezeichnetes Dreieck/Symbol — schriftunabhängig
        // (egui-Default-Font hat die Unicode-Pfeile nicht).
        let btn = |ui: &mut egui::Ui, r: egui::Rect, dir: JogDir| {
            let resp = ui.allocate_rect(r, Sense::click());
            let bg = if resp.hovered() {
                Color32::from_rgb(0x30, 0x36, 0x40)
            } else {
                Color32::from_rgb(0x24, 0x28, 0x30)
            };
            ui.painter().rect_filled(r, 8.0, bg);
            let c = r.center();
            let fg = Color32::from_gray(0xec);
            let s = 9.0;
            match dir {
                JogDir::Up => tri(
                    ui,
                    [
                        c + Vec2::new(0.0, -s),
                        c + Vec2::new(-s, s * 0.6),
                        c + Vec2::new(s, s * 0.6),
                    ],
                    fg,
                ),
                JogDir::Down => tri(
                    ui,
                    [
                        c + Vec2::new(0.0, s),
                        c + Vec2::new(-s, -s * 0.6),
                        c + Vec2::new(s, -s * 0.6),
                    ],
                    fg,
                ),
                JogDir::Left => tri(
                    ui,
                    [
                        c + Vec2::new(-s, 0.0),
                        c + Vec2::new(s * 0.6, -s),
                        c + Vec2::new(s * 0.6, s),
                    ],
                    fg,
                ),
                JogDir::Right => tri(
                    ui,
                    [
                        c + Vec2::new(s, 0.0),
                        c + Vec2::new(-s * 0.6, -s),
                        c + Vec2::new(-s * 0.6, s),
                    ],
                    fg,
                ),
                JogDir::Home => {
                    // Kleines Haus.
                    ui.painter().circle_filled(c, 4.0, fg);
                }
            }
            if resp.clicked() {
                log::info!("Jog: {dir:?}");
            }
        };
        btn(ui, cell(1, 0), JogDir::Up);
        btn(ui, cell(0, 1), JogDir::Left);
        btn(ui, cell(1, 1), JogDir::Home);
        btn(ui, cell(2, 1), JogDir::Right);
        btn(ui, cell(1, 2), JogDir::Down);
    });
}

#[derive(Debug, Clone, Copy)]
enum JogDir {
    Up,
    Down,
    Left,
    Right,
    Home,
}

/// Ausgefülltes Dreieck aus drei Punkten (für die Jog-Pfeile).
fn tri(ui: &egui::Ui, pts: [egui::Pos2; 3], color: Color32) {
    ui.painter().add(egui::Shape::convex_polygon(
        pts.to_vec(),
        color,
        egui::Stroke::NONE,
    ));
}

fn slider_row(ui: &mut egui::Ui, label: &str, unit: &str, value: &mut f64, min: f64, max: f64) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).weak());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(unit).small().weak());
            ui.add(egui::DragValue::new(value).range(min..=max).speed(0.5));
        });
    });
    ui.add(egui::Slider::new(value, min..=max).show_value(false));
    ui.add_space(4.0);
}
