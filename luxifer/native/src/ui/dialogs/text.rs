//! Text-Dialog: Eingabe, Font-Auswahl, Größe → Text als Pfad einfügen.

use crate::app::App;

/// Text-Dialog: Eingabe, Font-Auswahl, Größe → Text als Pfad einfügen.
pub(in crate::ui) fn text_dialog_window(ctx: &egui::Context, app: &mut App) {
    if app.text_dialog.is_none() {
        return;
    }
    let mut close = false;
    let mut commit = false;
    // Font-Liste (Name) für die ComboBox vorbereiten.
    let font_names: Vec<String> = app.fonts.iter().map(|f| f.name.clone()).collect();
    egui::Window::new("Text einfügen")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_min_width(340.0);
            let st = app.text_dialog.as_mut().unwrap();
            ui.label("Text");
            ui.add(
                egui::TextEdit::multiline(&mut st.text)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(6.0);
            egui::Grid::new("text_cfg")
                .num_columns(2)
                .spacing([8.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Font");
                    let current = st
                        .font_idx
                        .and_then(|i| font_names.get(i).cloned())
                        .unwrap_or_else(|| "—".into());
                    egui::ComboBox::from_id_salt("font")
                        .selected_text(current)
                        .width(220.0)
                        .show_ui(ui, |ui| {
                            for (i, name) in font_names.iter().enumerate() {
                                if ui.selectable_label(st.font_idx == Some(i), name).clicked() {
                                    st.font_idx = Some(i);
                                }
                            }
                        });
                    ui.end_row();
                    ui.label("Größe (mm)");
                    ui.add(
                        egui::DragValue::new(&mut st.size_mm)
                            .range(1.0..=500.0)
                            .speed(0.5),
                    );
                    ui.end_row();
                });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Einfügen").clicked() {
                    commit = true;
                }
                if ui.button("Abbrechen").clicked() {
                    close = true;
                }
            });
        });

    if commit && app.commit_text() {
        close = true;
    }
    if close {
        app.text_dialog = None;
    }
}
