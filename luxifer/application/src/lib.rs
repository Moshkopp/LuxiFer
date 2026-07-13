//! UI-unabhängige Anwendungsschicht von LuxiFer.
//!
//! Diese Schicht besitzt die laufende Editor-Sitzung und koordiniert
//! vollständige Anwendungsfälle. Sie kennt weder egui/winit/wgpu noch Tauri.

mod assets;
mod charon;
mod error;
mod laser;
mod project;
mod session;
mod sync_outbox;
#[cfg(test)]
mod test_env;

pub use charon::{connect_charon, CharonConnection, CharonHandshake, CharonWorkplace};
pub use error::AppError;
pub use laser::LaserService;
pub use luxifer_driver_ruida::{RuidaMachineSetting, RuidaSettingUnit};
pub use project::{ProjectDetail, ProjectService};
pub use session::{BoxShape, EditorSession, LayerParams, LayerToggle, PointPath};
pub use sync_outbox::{list_outbox, OutboxEntry, OutboxStatus};
