//! Modale Dialoge (egui-Fenster). Native hält jeweils nur den Entwurf; die
//! Mutation läuft über die Session bzw. die temporären Backends.
//!
//! Über die `UiAction`-Grenze (ADR 0011): Ein Dialog bekommt seinen Entwurf als
//! `&mut`-Draft (nicht `&mut App`) und meldet nur, ob der Nutzer übernehmen oder
//! abbrechen will. Den Draft-Lebenszyklus (Übernahme/Verwerfen) führt der Root.

mod geo_op;
mod guard;
mod image;
mod laser_manager;
mod layer;
mod project_save;
mod settings;
mod text;

pub(super) use geo_op::geo_op_dialog_window;
pub(super) use guard::guard_dialog;
pub(super) use image::{image_dialog_window, ImageDialogOutcome};
pub(super) use laser_manager::{laser_manager_window, LaserManagerOutcome};
pub(super) use layer::layer_dialog_window;
pub(super) use project_save::project_save_dialog_window;
pub(super) use settings::{settings_dialog_window, SettingsOutcome};
pub(super) use text::text_dialog_window;

/// Was ein Dialog nach einem Frame will. `None` = weiter offen, keine Aktion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum DialogOutcome {
    /// Fenster bleibt offen, Nutzer bearbeitet weiter.
    #[default]
    None,
    /// Nutzer will den Entwurf übernehmen.
    Commit,
    /// Nutzer hat abgebrochen.
    Cancel,
}
