//! egui-Panels: Werkzeugleiste (links), Layer + Palette (rechts). Bewusst nah an
//! den frischen Svelte-Designs (aktive-Farbe-Markierung, klare Sektionen). Alle
//! Aktionen laufen über den Core — die Panels halten keinen eigenen Wahrheits-
//! Zustand.

use egui::{Color32, RichText};
use luxifer_core::model::SWATCH_COLORS;

use crate::app::App;
use crate::laserpanel;
use crate::tools::{Tab, Tool};

fn c32(rgb: [u8; 3]) -> Color32 {
    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

pub fn build(ctx: &egui::Context, app: &mut App) {
    apply_theme(ctx);

    let left = egui::SidePanel::left("tools")
        .exact_width(96.0)
        .resizable(false)
        .show(ctx, |ui| tools_panel(ui, app));
    app.left_w = left.response.rect.width();

    let right = egui::SidePanel::right("inspector")
        .exact_width(260.0)
        .resizable(false)
        .show(ctx, |ui| {
            // Reiter-Umschalter: Design-Inspektor ↔ Laser-Bedienung.
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(app.tab == Tab::Design, "  Design  ")
                    .clicked()
                {
                    app.tab = Tab::Design;
                }
                if ui
                    .selectable_label(app.tab == Tab::Laser, "  Laser  ")
                    .clicked()
                {
                    app.tab = Tab::Laser;
                }
            });
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(8.0);

            match app.tab {
                Tab::Design => {
                    layers_panel(ui, app);
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);
                    palette_panel(ui, app);
                }
                Tab::Laser => laserpanel::show(ui, &mut app.laser),
            }
        });
    app.right_w = right.response.rect.width();

    // Statuszeile unten: FPS + aktives Tool (der native Perf-Beleg live).
    egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{:.0} fps", app.fps)).monospace());
            ui.separator();
            ui.label(format!("Werkzeug: {}", app.tool.label()));
            ui.separator();
            ui.label(format!("{} Objekte", app.state.shapes.len()));
        });
    });
}

fn tools_panel(ui: &mut egui::Ui, app: &mut App) {
    ui.add_space(6.0);
    ui.label(RichText::new("WERKZEUG").small().weak());
    ui.add_space(4.0);
    for t in [Tool::Select, Tool::Rect, Tool::Ellipse, Tool::Polygon] {
        let on = app.tool == t;
        if ui
            .add_sized(
                [ui.available_width(), 34.0],
                egui::SelectableLabel::new(on, t.label()),
            )
            .clicked()
        {
            app.tool = t;
        }
    }
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);
    if ui.button("↶ Undo").clicked() {
        app.state.undo();
    }
    if ui.button("↷ Redo").clicked() {
        app.state.redo();
    }

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);
    ui.label(RichText::new("DATEI").small().weak());
    if ui.button("📂 Öffnen…").clicked() {
        app.import_dialog();
    }
    // Schnellzugriff auf die große Testdatei (Aztec) für den Fill-Stresstest.
    let aztec = std::path::Path::new("/home/moshy/Schreibtisch/Aztec.svg");
    if aztec.exists() && ui.button("⬇ Aztec laden").clicked() {
        app.import_path(aztec);
    }
    if ui.button("▦ Fill an/aus").clicked() {
        app.toggle_fill();
    }
}

fn layers_panel(ui: &mut egui::Ui, app: &mut App) {
    ui.label(RichText::new("EBENEN").small().weak());
    ui.add_space(4.0);
    if app.state.layers.is_empty() {
        ui.weak("Keine Ebenen — zeichne etwas.");
        return;
    }
    // Von oben (letzter Layer) nach unten anzeigen.
    let n = app.state.layers.len();
    for i in (0..n).rev() {
        let (color, name, mut visible, count) = {
            let l = &app.state.layers[i];
            let cnt = app.state.shapes.iter().filter(|s| s.layer_id == i).count();
            (l.color, l.name.clone(), l.visible, cnt)
        };
        ui.horizontal(|ui| {
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::click());
            ui.painter().rect_filled(rect, 4.0, c32(color));
            if resp.clicked() {
                app.pick_color(color);
            }
            if ui.checkbox(&mut visible, "").changed() {
                app.state.layers[i].visible = visible;
            }
            ui.label(format!("{name}  ·  {count}"));
        });
    }
}

fn palette_panel(ui: &mut egui::Ui, app: &mut App) {
    ui.label(RichText::new("FARBE").small().weak());
    ui.add_space(6.0);
    let active = app.accent;
    ui.horizontal_wrapped(|ui| {
        for &sw in SWATCH_COLORS {
            let is_active = sw == active;
            let size = if is_active { 26.0 } else { 22.0 };
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
            let r = size * 0.5;
            ui.painter().circle_filled(rect.center(), r, c32(sw));
            if is_active {
                // Heller Ring mit dunklem Absatz — wie in der Svelte-Palette.
                ui.painter()
                    .circle_stroke(rect.center(), r + 1.5, (2.0, Color32::from_gray(20)));
                ui.painter()
                    .circle_stroke(rect.center(), r + 3.0, (2.0, Color32::from_gray(235)));
            }
            if resp.hovered() {
                ui.painter()
                    .circle_stroke(rect.center(), r, (1.5, Color32::WHITE));
            }
            if resp.clicked() {
                app.pick_color(sw);
            }
        }
    });
}

/// Dunkles Theme, an den Svelte-Look angelehnt (kühles Blau-Grau).
fn apply_theme(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = Color32::from_rgb(0x14, 0x17, 0x1c);
    v.window_fill = Color32::from_rgb(0x17, 0x1a, 0x20);
    v.override_text_color = Some(Color32::from_rgb(0xec, 0xee, 0xf1));
    v.widgets.inactive.bg_fill = Color32::from_rgb(0x24, 0x28, 0x30);
    v.selection.bg_fill = Color32::from_rgb(0x3B, 0x82, 0xF6);
    ctx.set_visuals(v);
}
