//! Werkzeugleiste (links): 2-spaltiges Icon-Grid, 5 Gruppen wie die
//! Tauri-ToolsPanel. Enthält den geteilten `icon_button`-Helfer.

use egui::Color32;

use crate::app::App;
use crate::tools::Tool;

/// Quadratischer Icon-Button (Werkzeugleiste). `on` = aktiv (Akzent),
/// `dim` = Stub/deaktiviert dezenter. Gibt true bei Klick zurück.
pub(super) fn icon_button(
    ui: &mut egui::Ui,
    side: f32,
    icon: &str,
    tip: &str,
    on: bool,
    dim: bool,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::click());
    let accent = Color32::from_rgb(0x3B, 0x82, 0xF6);
    let bg = if on {
        accent.gamma_multiply(0.85)
    } else if resp.hovered() {
        Color32::from_rgb(0x25, 0x2a, 0x33)
    } else {
        Color32::from_rgb(0x1c, 0x1f, 0x26)
    };
    ui.painter().rect(
        rect,
        7.0,
        bg,
        egui::Stroke::new(1.0, Color32::from_rgb(0x2a, 0x2e, 0x36)),
    );
    let fg = if dim {
        Color32::from_rgb(0x9a, 0xa0, 0xa9)
    } else {
        Color32::from_rgb(0xec, 0xee, 0xf1)
    };
    // Icon-Box zentriert (etwas kleiner als der Button).
    let pad = side * 0.22;
    let ic = egui::Rect::from_min_max(
        rect.min + egui::vec2(pad, pad),
        rect.max - egui::vec2(pad, pad),
    );
    crate::icons::draw(ui.painter(), ic, icon, fg);
    resp.on_hover_text(tip).clicked()
}

/// Werkzeuge in einem 2-Spalten-Grid; gibt das geklickte Werkzeug zurück.
fn tool_grid(ui: &mut egui::Ui, side: f32, gap: f32, cur: Tool, tools: &[Tool]) -> Option<Tool> {
    let mut clicked = None;
    egui::Grid::new(("tg", tools.first().map(|t| t.label()).unwrap_or("")))
        .spacing([gap, gap])
        .show(ui, |ui| {
            for (i, &t) in tools.iter().enumerate() {
                if icon_button(ui, side, t.icon(), t.label(), cur == t, false) {
                    clicked = Some(t);
                }
                if i % 2 == 1 {
                    ui.end_row();
                }
            }
        });
    clicked
}

/// 2-spaltige Werkzeugleiste, 5 Gruppen wie die Tauri-ToolsPanel — nur Icons.
pub(super) fn tools_panel(ui: &mut egui::Ui, app: &mut App) {
    use crate::tools::ToolAction as A;
    ui.add_space(4.0);
    let full = ui.available_width();
    let gap = 4.0;
    let side = ((full - gap) / 2.0).clamp(24.0, 42.0);

    let cur = app.tool;
    // Gruppe 1: Auswahl (breit über beide Spalten).
    if icon_button(
        ui,
        full.min(side * 2.0 + gap),
        "select",
        "Auswahl / Verschieben",
        cur == Tool::Select,
        false,
    ) {
        app.tool = Tool::Select;
    }
    divider(ui);
    // Gruppe 2: Zeichnen & Formen.
    if let Some(t) = tool_grid(
        ui,
        side,
        gap,
        cur,
        &[
            Tool::Rect,
            Tool::Ellipse,
            Tool::Polygon,
            Tool::Line,
            Tool::Polyline,
            Tool::Spline,
            Tool::Bezier,
        ],
    ) {
        app.tool = t;
    }
    // Text (Sofort-Aktion) + Node (Werkzeug) in derselben Gruppe.
    egui::Grid::new("tg_textnode")
        .spacing([gap, gap])
        .show(ui, |ui| {
            if icon_button(ui, side, "text", "Text einfügen (Text→Pfad)", false, false) {
                app.open_text_dialog();
            }
            if icon_button(
                ui,
                side,
                "node",
                "Knoten bearbeiten",
                app.tool == Tool::Node,
                false,
            ) {
                app.tool = Tool::Node;
            }
            ui.end_row();
        });
    divider(ui);
    // Gruppe 3: Operationen. `trim` bleibt Stub (wie Tauri).
    egui::Grid::new("tg_ops")
        .spacing([gap, gap])
        .show(ui, |ui| {
            icon_button(ui, side, "trim", "Trimmen (Vorschau)", false, true);
            if icon_button(ui, side, "bridge", "Haltesteg (Klick+Ziehen)", false, false) {
                app.begin_action(A::Bridge);
            }
            ui.end_row();
            if icon_button(ui, side, "boolean", "Boolean (Auswahl)", false, false) {
                app.begin_action(A::Boolean);
            }
            if icon_button(
                ui,
                side,
                "fillet",
                "Ecken verrunden (Auswahl)",
                false,
                false,
            ) {
                app.begin_action(A::Fillet);
            }
            ui.end_row();
            if icon_button(
                ui,
                side,
                "pattern-fill",
                "Muster füllen (Auswahl)",
                false,
                false,
            ) {
                app.begin_action(A::PatternFill);
            }
            if icon_button(
                ui,
                side,
                "offset",
                "Offset / parallele Kontur (Auswahl)",
                false,
                false,
            ) {
                app.begin_action(A::Offset);
            }
            ui.end_row();
            if icon_button(
                ui,
                side,
                "measure",
                "Messen (Klick+Ziehen)",
                app.tool == Tool::Measure,
                false,
            ) {
                app.tool = Tool::Measure;
            }
            ui.end_row();
        });
    divider(ui);
    // Gruppe 4: Spiegeln.
    egui::Grid::new("tg_mirror")
        .spacing([gap, gap])
        .show(ui, |ui| {
            if icon_button(ui, side, "mirror-h", "Horizontal spiegeln", false, false) {
                app.mirror_h();
            }
            if icon_button(ui, side, "mirror-v", "Vertikal spiegeln", false, false) {
                app.mirror_v();
            }
            ui.end_row();
        });
    divider(ui);
    // Gruppe 5: Untersetzer.
    egui::Grid::new("tg_coaster")
        .spacing([gap, gap])
        .show(ui, |ui| {
            if icon_button(
                ui,
                side,
                "coaster-rect",
                "4×2 eckige Untersetzer",
                false,
                false,
            ) {
                app.insert_coasters(false);
            }
            if icon_button(
                ui,
                side,
                "coaster-circle",
                "4×2 runde Untersetzer",
                false,
                false,
            ) {
                app.insert_coasters(true);
            }
            ui.end_row();
        });
}

/// Dünner horizontaler Trenner zwischen Werkzeuggruppen.
fn divider(ui: &mut egui::Ui) {
    ui.add_space(4.0);
    let w = ui.available_width() * 0.8;
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    let y = rect.center().y;
    let x0 = rect.center().x - w / 2.0;
    ui.painter().line_segment(
        [egui::pos2(x0, y), egui::pos2(x0 + w, y)],
        egui::Stroke::new(1.0, Color32::from_rgb(0x2a, 0x2e, 0x36)),
    );
    ui.add_space(4.0);
}
