//! Modale Dialoge (egui-Fenster). Native hält jeweils nur den Entwurf; die
//! Mutation läuft über die Session bzw. die temporären Backends.

mod laser_settings;
mod layer;
mod text;

pub(super) use laser_settings::laser_settings_window;
pub(super) use layer::layer_dialog_window;
pub(super) use text::text_dialog_window;
