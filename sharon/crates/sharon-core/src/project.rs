use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Metadaten eines synchronisierten Projekts. Der Canvas-Inhalt
/// selbst ist für Sharon ein opakes Blob — die Fachlogik lebt in der GUI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub id: Uuid,
    pub name: String,
    pub version: u64,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}
