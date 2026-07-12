//! UI-unabhängige Anwendungsschicht von LuxiFer.
//!
//! Diese Schicht besitzt die laufende Editor-Sitzung und koordiniert
//! vollständige Anwendungsfälle. Sie kennt weder egui/winit/wgpu noch Tauri.

mod assets;
mod error;
mod laser;
mod project;
mod session;
#[cfg(test)]
mod test_env;

pub use error::AppError;
pub use laser::LaserService;
pub use project::{ProjectDetail, ProjectService};
pub use session::{BoxShape, EditorSession, LayerParams, LayerToggle, PointPath};
