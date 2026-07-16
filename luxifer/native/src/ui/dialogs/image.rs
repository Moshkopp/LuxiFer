//! Bildparameter-Dialog (Doppelklick auf ein Bild-Objekt). Bearbeitet die
//! nicht-destruktiven Verarbeitungsparameter (ADR 0004) und bietet das
//! Vektorisieren (Trace) an; Native hält nur den Entwurf, Speichern läuft
//! über `EditorSession::set_image_params`, Trace über
//! `EditorSession::trace_image`.

use luxifer_core::ImageMode;

use super::super::state::{ImageDialogPage, ImageDialogState};

/// Ergebnis des Bild-Dialogs. `Trace` vektorisiert das Bild mit den
/// Trace-Reglern des Entwurfs (der Dialog bleibt dabei offen).
#[derive(PartialEq, Eq)]
pub(in crate::ui) enum ImageDialogOutcome {
    None,
    Save,
    Cancel,
    Trace,
    Crop,
}

pub(in crate::ui) fn image_dialog_window(
    root_ui: &mut egui::Ui,
    st: &mut ImageDialogState,
) -> ImageDialogOutcome {
    let mut outcome = ImageDialogOutcome::None;
    let title = match st.page {
        ImageDialogPage::Settings => "Bild bearbeiten",
        ImageDialogPage::Trace => "Bild vektorisieren",
        ImageDialogPage::Crop => "Bild zuschneiden",
    };
    egui::Window::new(title)
        .order(egui::Order::Foreground)
        .collapsible(false)
        .resizable(true)
        .default_size(egui::vec2(860.0, 430.0))
        .min_size(egui::vec2(720.0, 380.0))
        .max_height(520.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(root_ui, |ui| {
            match st.page {
                ImageDialogPage::Settings => ui.columns(2, |columns| {
                    settings_panel(&mut columns[0], st);
                    preview_panel(&mut columns[1], st, "LIVE-VORSCHAU");
                }),
                ImageDialogPage::Trace => ui.columns(2, |columns| {
                    trace_panel(&mut columns[0], st, &mut outcome);
                    preview_panel(&mut columns[1], st, "ERFASSTE BEREICHE");
                }),
                ImageDialogPage::Crop => ui.columns(2, |columns| {
                    crop_panel(&mut columns[0], st, &mut outcome);
                    preview_panel(&mut columns[1], st, "AUSSCHNITT-VORSCHAU");
                }),
            };

            ui.separator();
            ui.horizontal(|ui| match st.page {
                ImageDialogPage::Settings => {
                    if ui.button("Speichern").clicked() {
                        outcome = ImageDialogOutcome::Save;
                    }
                    if ui.button("Abbrechen").clicked() {
                        outcome = ImageDialogOutcome::Cancel;
                    }
                }
                ImageDialogPage::Trace => {
                    if ui.button("Zurück").clicked() {
                        return_to_settings(st);
                    }
                    if ui.button("Schließen").clicked() {
                        outcome = ImageDialogOutcome::Cancel;
                    }
                }
                ImageDialogPage::Crop => {
                    if ui.button("Zurück").clicked() {
                        return_to_settings(st);
                    }
                    if ui.button("Schließen").clicked() {
                        outcome = ImageDialogOutcome::Cancel;
                    }
                }
            });
        });
    outcome
}

fn settings_panel(ui: &mut egui::Ui, st: &mut ImageDialogState) {
    ui.label(egui::RichText::new("EINSTELLUNGEN").small().weak());
    let p = &mut st.params;
    egui::Grid::new("image_cfg")
        .num_columns(2)
        .spacing([8.0, 8.0])
        .show(ui, |ui| {
            ui.label("Modus");
            let mode_label = |m: ImageMode| match m {
                ImageMode::Grayscale => "Graustufe",
                ImageMode::Threshold => "Schwelle",
                ImageMode::Floyd => "Floyd–Steinberg",
                ImageMode::Jarvis => "Jarvis",
                ImageMode::Stucki => "Stucki",
                ImageMode::Atkinson => "Atkinson",
                ImageMode::Bayer => "Bayer 4×4",
                ImageMode::LaserRuns => "Laser-Runs",
            };
            egui::ComboBox::from_id_salt("image_mode")
                .selected_text(mode_label(p.mode))
                .width(220.0)
                .show_ui(ui, |ui| {
                    for m in [
                        ImageMode::Grayscale,
                        ImageMode::Threshold,
                        ImageMode::Floyd,
                        ImageMode::Jarvis,
                        ImageMode::Stucki,
                        ImageMode::Atkinson,
                        ImageMode::Bayer,
                        ImageMode::LaserRuns,
                    ] {
                        ui.selectable_value(&mut p.mode, m, mode_label(m));
                    }
                });
            ui.end_row();

            if p.mode == ImageMode::Threshold {
                ui.label("Schwelle");
                ui.add(egui::Slider::new(&mut p.threshold, 0..=255));
                ui.end_row();
            }

            ui.label("Helligkeit");
            ui.add(egui::Slider::new(&mut p.brightness, -100..=100));
            ui.end_row();

            ui.label("Kontrast");
            ui.add(egui::Slider::new(&mut p.contrast, -100..=100));
            ui.end_row();

            ui.label("Gamma");
            ui.add(egui::Slider::new(&mut p.gamma, 0.1..=3.0));
            ui.end_row();

            ui.label("Invertieren (Canvas)");
            ui.checkbox(&mut p.invert_editor, "");
            ui.end_row();

            ui.label("Invertieren (Laser)");
            ui.checkbox(&mut p.invert_laser, "");
            ui.end_row();
        });

    ui.add_space(16.0);
    ui.separator();
    if ui.button("Vektorisieren …").clicked() {
        st.page = ImageDialogPage::Trace;
        st.preview_key = None;
        st.preview_zoom = 1.0;
        st.preview_pan = egui::Vec2::ZERO;
    }
    ui.weak("Öffnet die Trace-Einstellungen mit eigener Ergebnisvorschau.");
    ui.add_space(6.0);
    if ui.button("Zuschneiden …").clicked() {
        st.page = ImageDialogPage::Crop;
        reset_preview_view(st);
    }
    ui.weak("Öffnet das Zuschneiden als eigenen Arbeitsbereich.");
}

fn crop_panel(ui: &mut egui::Ui, st: &mut ImageDialogState, outcome: &mut ImageDialogOutcome) {
    ui.label(egui::RichText::new("SCHNITTKANTEN").small().weak());
    ui.add_space(6.0);
    let mut left = st.crop_rect[0] * 100.0;
    let mut top = st.crop_rect[1] * 100.0;
    let mut right = (1.0 - st.crop_rect[2]) * 100.0;
    let mut bottom = (1.0 - st.crop_rect[3]) * 100.0;
    egui::Grid::new("image_crop")
        .num_columns(2)
        .spacing([8.0, 10.0])
        .show(ui, |ui| {
            for (label, value) in [
                ("Links", &mut left),
                ("Oben", &mut top),
                ("Rechts", &mut right),
                ("Unten", &mut bottom),
            ] {
                ui.label(label);
                ui.add(egui::Slider::new(value, 0.0..=99.0).suffix(" %"));
                ui.end_row();
            }
        });
    let max_horizontal = 99.0;
    if left + right > max_horizontal {
        right = max_horizontal - left;
    }
    if top + bottom > 99.0 {
        bottom = 99.0 - top;
    }
    st.crop_rect = [
        left / 100.0,
        top / 100.0,
        1.0 - right / 100.0,
        1.0 - bottom / 100.0,
    ];
    ui.add_space(8.0);
    if ui.button("Vollen Bildbereich wiederherstellen").clicked() {
        st.crop_rect = [0.0, 0.0, 1.0, 1.0];
        st.preview_key = None;
    }
    ui.add_space(12.0);
    if ui.button("Ausschnitt anwenden").clicked() {
        *outcome = ImageDialogOutcome::Crop;
    }
    ui.weak(
        "Das Originalasset bleibt erhalten; Undo stellt die vorherige Bildreferenz wieder her.",
    );
}

fn reset_preview_view(st: &mut ImageDialogState) {
    st.preview_key = None;
    st.preview_zoom = 1.0;
    st.preview_pan = egui::Vec2::ZERO;
}

fn return_to_settings(st: &mut ImageDialogState) {
    st.page = ImageDialogPage::Settings;
    reset_preview_view(st);
}

fn trace_panel(ui: &mut egui::Ui, st: &mut ImageDialogState, outcome: &mut ImageDialogOutcome) {
    ui.label(egui::RichText::new("TRACE-EINSTELLUNGEN").small().weak());
    ui.add_space(6.0);
    egui::Grid::new("image_trace")
        .num_columns(2)
        .spacing([8.0, 10.0])
        .show(ui, |ui| {
            ui.label("Schwelle");
            ui.add(egui::Slider::new(&mut st.trace_threshold, 0..=255));
            ui.end_row();
            ui.label("Invertieren");
            ui.checkbox(&mut st.trace_invert, "");
            ui.end_row();
        });
    ui.add_space(12.0);
    if ui.button("Konturen erzeugen").clicked() {
        *outcome = ImageDialogOutcome::Trace;
    }
    ui.weak("Schwarz zeigt die erfassten Motivbereiche. Das Originalbild bleibt unverändert.");
}

fn preview_panel(ui: &mut egui::Ui, st: &mut ImageDialogState, label: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).small().weak());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("Ansicht zurücksetzen").clicked() {
                st.preview_zoom = 1.0;
                st.preview_pan = egui::Vec2::ZERO;
            }
        });
    });

    let desired = egui::vec2(ui.available_width(), 340.0);
    let (response, painter) = ui.allocate_painter(desired, egui::Sense::drag());
    let rect = response.rect;
    painter.rect_filled(rect, 8.0, ui.visuals().extreme_bg_color);
    painter.rect_stroke(
        rect,
        8.0,
        egui::Stroke::new(1.0, ui.visuals().window_stroke.color),
        egui::StrokeKind::Inside,
    );

    if response.dragged() {
        st.preview_pan += response.drag_delta();
    }
    if response.hovered() {
        let scroll = ui.input(|input| input.smooth_scroll_delta.y);
        if scroll != 0.0 {
            st.preview_zoom = (st.preview_zoom * (scroll * 0.002).exp()).clamp(0.1, 20.0);
        }
    }

    if let Some(texture) = &st.preview {
        let original = texture.size_vec2();
        let viewport = rect.shrink(16.0).size();
        let fit = (viewport.x / original.x).min(viewport.y / original.y);
        let size = original * fit * st.preview_zoom;
        let image_rect = egui::Rect::from_center_size(rect.center() + st.preview_pan, size);
        painter.image(
            texture.id(),
            image_rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    } else if let Some(error) = &st.preview_error {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            error,
            egui::TextStyle::Body.resolve(ui.style()),
            ui.visuals().error_fg_color,
        );
    } else {
        ui.ctx().request_repaint();
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Vorschau wird geladen …",
            egui::TextStyle::Body.resolve(ui.style()),
            ui.visuals().weak_text_color(),
        );
    }

    if response.hovered() {
        response
            .on_hover_cursor(egui::CursorIcon::Grab)
            .on_hover_text("Ziehen: verschieben · Mausrad: zoomen");
    }
}
