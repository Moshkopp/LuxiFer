//! Text-Dialog: Eingabe, Font-Auswahl (Familie/Schnitt, Suche), Layout
//! (Größe, Ausrichtung, Abstände) und Live-Vorschau → Text als Pfad einfügen.
//!
//! Der Dialog zeichnet nur: die Vorschau-Konturen berechnet der App-Root über
//! den Core (`App::update_text_preview`) und legt sie gecacht in den Entwurf.

use super::super::state::TextDialogState;
use super::DialogOutcome;
use luxifer_core::text::TextAlign;

/// Zeichnet das Fenster auf den Entwurf `st`; `families` ist die gescannte
/// Font-Familien-Liste (Indizes korrespondieren mit `st.family_idx`).
/// Meldet über `DialogOutcome`, ob der Nutzer einfügen/abbrechen will.
pub(in crate::ui) fn text_dialog_window(
    root_ui: &mut egui::Ui,
    st: &mut TextDialogState,
    families: &[crate::fonts::FontFamily],
) -> DialogOutcome {
    let mut outcome = DialogOutcome::None;
    let title = if st.edit_index.is_some() {
        "Text bearbeiten"
    } else {
        "Text einfügen"
    };
    egui::Window::new(title)
        .order(egui::Order::Foreground)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(root_ui, |ui| {
            ui.set_min_width(420.0);
            ui.set_max_width(420.0);

            // Live-Vorschau des aktuellen Entwurfs (Konturen aus dem Core).
            preview_panel(ui, st);
            ui.add_space(6.0);

            ui.label("Text");
            ui.add(
                egui::TextEdit::multiline(&mut st.text)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(6.0);

            // Font-Auswahl: Suchfeld + Import, darunter die Familien-Liste.
            ui.horizontal(|ui| {
                ui.label("Font");
                let search = egui::TextEdit::singleline(&mut st.search)
                    .hint_text("Suchen…")
                    .desired_width(ui.available_width() - 60.0);
                ui.add(search);
                if !st.search.is_empty() && ui.button("✖").on_hover_text("Suche leeren").clicked()
                {
                    st.search.clear();
                }
                if ui
                    .button("✚")
                    .on_hover_text("Font-Datei importieren (TTF/OTF)")
                    .clicked()
                {
                    st.request_font_import = true;
                }
            });
            family_list(ui, st, families);
            ui.add_space(6.0);

            egui::Grid::new("text_cfg")
                .num_columns(2)
                .spacing([8.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Schnitt");
                    face_combo(ui, st, families);
                    ui.end_row();

                    ui.label("Größe (mm)");
                    ui.add(
                        egui::DragValue::new(&mut st.size_mm)
                            .range(1.0..=500.0)
                            .speed(0.5),
                    );
                    ui.end_row();

                    ui.label("Ausrichtung");
                    ui.horizontal(|ui| {
                        for (align, label) in [
                            (TextAlign::Left, "Links"),
                            (TextAlign::Center, "Zentriert"),
                            (TextAlign::Right, "Rechts"),
                        ] {
                            if ui.selectable_label(st.align == align, label).clicked() {
                                st.align = align;
                            }
                        }
                    });
                    ui.end_row();

                    ui.label("Zeilenabstand");
                    ui.add(
                        egui::DragValue::new(&mut st.line_spacing)
                            .range(0.5..=4.0)
                            .speed(0.02)
                            .fixed_decimals(2)
                            .suffix(" ×"),
                    );
                    ui.end_row();

                    ui.label("Zeichenabstand");
                    ui.add(
                        egui::DragValue::new(&mut st.letter_spacing_mm)
                            .range(-10.0..=100.0)
                            .speed(0.1)
                            .fixed_decimals(1)
                            .suffix(" mm"),
                    );
                    ui.end_row();
                });
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                let commit_label = if st.edit_index.is_some() {
                    "Übernehmen"
                } else {
                    "Einfügen"
                };
                if ui.button(commit_label).clicked() {
                    outcome = DialogOutcome::Commit;
                }
                if ui.button("Abbrechen").clicked() {
                    outcome = DialogOutcome::Cancel;
                }
            });

            // Sicherheitsnetz: Esc bricht ab, Strg+Enter übernimmt (Enter
            // gehört dem mehrzeiligen Textfeld).
            ui.input(|input| {
                if input.key_pressed(egui::Key::Escape) {
                    outcome = DialogOutcome::Cancel;
                }
                if input.modifiers.command && input.key_pressed(egui::Key::Enter) {
                    outcome = DialogOutcome::Commit;
                }
            });
        });
    outcome
}

/// Zeichnet die gecachten Vorschau-Konturen eingepasst in ein festes Panel.
fn preview_panel(ui: &mut egui::Ui, st: &TextDialogState) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 96.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);

    // BBox der Konturen (mm).
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for (pts, _) in &st.preview {
        for &(x, y) in pts {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    if x0 > x1 {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "— keine Vorschau —",
            egui::TextStyle::Body.resolve(ui.style()),
            ui.visuals().weak_text_color(),
        );
        return;
    }

    // Einpassen mit Rand; nie über 1:1 hinaus vergrößern lassen wäre falsch —
    // kleine Texte sollen das Panel füllen dürfen.
    let pad = 10.0_f32;
    let (w_mm, h_mm) = ((x1 - x0).max(1e-6), (y1 - y0).max(1e-6));
    let scale =
        ((rect.width() - 2.0 * pad) as f64 / w_mm).min((rect.height() - 2.0 * pad) as f64 / h_mm);
    let off_x = rect.center().x as f64 - (w_mm * scale) / 2.0 - x0 * scale;
    let off_y = rect.center().y as f64 - (h_mm * scale) / 2.0 - y0 * scale;

    let stroke = egui::Stroke::new(1.0, ui.visuals().strong_text_color());
    for (pts, closed) in &st.preview {
        let screen: Vec<egui::Pos2> = pts
            .iter()
            .map(|&(x, y)| egui::pos2((x * scale + off_x) as f32, (y * scale + off_y) as f32))
            .collect();
        if *closed {
            painter.add(egui::Shape::closed_line(screen, stroke));
        } else {
            painter.add(egui::Shape::line(screen, stroke));
        }
    }
}

/// Ab wie vielen Familien mit gleichem Namens-Präfix (erstes Wort) die Liste
/// sie zu einer aufklappbaren Gruppe zusammenfasst (z. B. „Noto (57)").
const GROUP_MIN: usize = 3;

/// Scrollbare, gefilterte Familien-Liste; Klick wählt die Familie (Schnitt
/// springt auf den Standard-Schnitt der neuen Familie).
///
/// Aufbau: importierte Fonts stehen als eigener Block oben (die Familien-Liste
/// ist entsprechend vorsortiert); System-Familien mit gemeinsamem erstem Wort
/// werden zu aufklappbaren Gruppen zusammengefasst, damit die Liste bei
/// hunderten Systemfonts überschaubar bleibt. Bei aktiver Suche wird flach
/// gefiltert (Treffer wären in zugeklappten Gruppen sonst unsichtbar).
fn family_list(ui: &mut egui::Ui, st: &mut TextDialogState, families: &[crate::fonts::FontFamily]) {
    let needle = st.search.to_lowercase();
    let filtered: Vec<usize> = families
        .iter()
        .enumerate()
        .filter(|(_, fam)| needle.is_empty() || fam.name.to_lowercase().contains(&needle))
        .map(|(i, _)| i)
        .collect();

    egui::Frame::new()
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(4.0)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_height(140.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    let (imported, system): (Vec<usize>, Vec<usize>) = filtered
                        .iter()
                        .copied()
                        .partition(|&i| families[i].imported);

                    if !imported.is_empty() {
                        ui.weak("Eigene Fonts");
                        for &fam_idx in &imported {
                            family_row(ui, st, families, fam_idx);
                        }
                        if !system.is_empty() {
                            ui.separator();
                        }
                    }

                    if needle.is_empty() {
                        grouped_rows(ui, st, families, &system);
                    } else {
                        for &fam_idx in &system {
                            family_row(ui, st, families, fam_idx);
                        }
                    }
                });
            if filtered.is_empty() {
                ui.add_space(4.0);
                ui.weak("  keine Treffer");
                ui.add_space(4.0);
            }
        });
}

/// System-Familien, gebündelt nach erstem Namens-Wort: ab [`GROUP_MIN`]
/// Familien wird das Präfix zu einer zuklappbaren Gruppe (Zustand merkt sich
/// egui pro Präfix); die Gruppe mit der aktuellen Auswahl startet offen.
fn grouped_rows(
    ui: &mut egui::Ui,
    st: &mut TextDialogState,
    families: &[crate::fonts::FontFamily],
    system: &[usize],
) {
    let prefix = |i: usize| {
        families[i]
            .name
            .split_whitespace()
            .next()
            .unwrap_or(&families[i].name)
    };
    let mut pos = 0;
    while pos < system.len() {
        let p = prefix(system[pos]);
        let mut end = pos + 1;
        while end < system.len() && prefix(system[end]).eq_ignore_ascii_case(p) {
            end += 1;
        }
        let group = &system[pos..end];
        if group.len() >= GROUP_MIN {
            let contains_selected = group.iter().any(|&i| st.family_idx == Some(i));
            egui::CollapsingHeader::new(format!("{p} ({})", group.len()))
                .id_salt(("font_group", p))
                .default_open(contains_selected)
                .show(ui, |ui| {
                    for &fam_idx in group {
                        family_row(ui, st, families, fam_idx);
                    }
                });
        } else {
            for &fam_idx in group {
                family_row(ui, st, families, fam_idx);
            }
        }
        pos = end;
    }
}

/// Eine anklickbare Familien-Zeile.
fn family_row(
    ui: &mut egui::Ui,
    st: &mut TextDialogState,
    families: &[crate::fonts::FontFamily],
    fam_idx: usize,
) {
    let selected = st.family_idx == Some(fam_idx);
    if ui
        .selectable_label(selected, &families[fam_idx].name)
        .clicked()
        && !selected
    {
        st.family_idx = Some(fam_idx);
        st.face_idx = families[fam_idx].default_face();
    }
}

/// Schnitt-Auswahl (Regular/Bold/…) der gewählten Familie.
fn face_combo(ui: &mut egui::Ui, st: &mut TextDialogState, families: &[crate::fonts::FontFamily]) {
    let Some(fam) = st.family_idx.and_then(|i| families.get(i)) else {
        ui.weak("—");
        return;
    };
    if st.face_idx >= fam.faces.len() {
        st.face_idx = fam.default_face();
    }
    let current = fam
        .faces
        .get(st.face_idx)
        .map(|f| f.style.as_str())
        .unwrap_or("—");
    egui::ComboBox::from_id_salt("text_face")
        .selected_text(current)
        .width(180.0)
        .show_ui(ui, |ui| {
            for (i, face) in fam.faces.iter().enumerate() {
                if ui.selectable_label(st.face_idx == i, &face.style).clicked() {
                    st.face_idx = i;
                }
            }
        });
}
