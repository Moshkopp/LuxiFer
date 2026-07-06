//! Sharon Store: Ablage für Projekte, Assets und Fonts.
//! Erste Implementierung: Dateisystem-basiert.

use sharon_core::ProjectMeta;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("nicht gefunden: {0}")]
    NotFound(Uuid),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}

/// Abstraktion über die Projektablage, damit später andere
/// Backends (z. B. SQLite) möglich sind.
pub trait ProjectStore: Send + Sync {
    fn list(&self) -> Result<Vec<ProjectMeta>, StoreError>;
    fn get(&self, id: Uuid) -> Result<ProjectMeta, StoreError>;
    fn put(&self, meta: ProjectMeta, blob: &[u8]) -> Result<(), StoreError>;
}
