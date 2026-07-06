//! Sharon Core: Domänenmodelle des optionalen LuxiFer-Servers.
//! Sharon koordiniert (Projekte, Assets, Fonts, Sessions) —
//! er steuert niemals selbst eine Maschine.

pub mod project;
pub mod session;

pub use project::*;
pub use session::*;
