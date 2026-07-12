//! Parameterdialog für die geometrischen Operationen Boolean, Offset und
//! Fillet. Native hält nur den Entwurf; die Ausführung läuft über die Session
//! (mit Auswahlvoraussetzung und Undo).

use luxifer_core::BoolOp;

use super::super::state::{GeoOpDialogState, GeoOpKind};
use super::DialogOutcome;

pub(in crate::ui) fn geo_op_dialog_window(
    ctx: &egui::Context,
    st: &mut GeoOpDialogState,
) -> DialogOutcome {
    let mut outcome = DialogOutcome::None;
    let title = match st.kind {
        GeoOpKind::Boolean => "Boolesche Operation",
        GeoOpKind::Offset => "Offset",
        GeoOpKind::Fillet => "Ecken verrunden",
    };
    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_min_width(300.0);
            match st.kind {
                GeoOpKind::Boolean => {
                    let label = |op: BoolOp| match op {
                        BoolOp::Union => "Vereinigen (A ∪ B)",
                        BoolOp::Intersect => "Schneiden (A ∩ B)",
                        BoolOp::Difference => "Abziehen (A − B)",
                    };
                    egui::ComboBox::from_label("Variante")
                        .selected_text(label(st.bool_op))
                        .show_ui(ui, |ui| {
                            for op in [BoolOp::Union, BoolOp::Intersect, BoolOp::Difference] {
                                ui.selectable_value(&mut st.bool_op, op, label(op));
                            }
                        });
                }
                GeoOpKind::Offset => {
                    ui.horizontal(|ui| {
                        ui.label("Distanz (mm)");
                        ui.add(
                            egui::DragValue::new(&mut st.distance)
                                .range(-100.0..=100.0)
                                .speed(0.1),
                        );
                    });
                }
                GeoOpKind::Fillet => {
                    ui.horizontal(|ui| {
                        ui.label("Radius (mm)");
                        ui.add(
                            egui::DragValue::new(&mut st.radius)
                                .range(0.1..=100.0)
                                .speed(0.1),
                        );
                    });
                }
            }

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Anwenden").clicked() {
                    outcome = DialogOutcome::Commit;
                }
                if ui.button("Abbrechen").clicked() {
                    outcome = DialogOutcome::Cancel;
                }
            });
        });
    outcome
}
