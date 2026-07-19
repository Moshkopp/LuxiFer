//! Persistente lokale Projekt-Outbox für Charon (ADR 0012).
//!
//! Jeder Eintrag besitzt eine eigene Payload-Kopie. Damit bleibt eine bereits
//! eingereihte Revision unveränderlich, auch wenn Strg+S dieselbe sichtbare
//! Projektversion später erneut aktualisiert.

use std::path::{Path, PathBuf};

use luxifer_core::{assets::content_hash, data_root, datetime, ProjectFile};
use serde::{Deserialize, Serialize};

use crate::AppError;

const OUTBOX_DIR: &str = "sync/outbox";
const MANIFEST_FILE: &str = "manifest.json";
const PAYLOAD_FILE: &str = "payload.luxi";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutboxStatus {
    Pending,
    Uploaded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxEntry {
    pub revision_id: String,
    pub project_id: String,
    pub project_name: String,
    pub project_version_id: String,
    pub parent_revision_id: Option<String>,
    pub workplace_id: String,
    pub queued_at: String,
    pub content_hash: String,
    pub payload_file: String,
    pub status: OutboxStatus,
    #[serde(default)]
    pub last_error: Option<String>,
}

impl OutboxEntry {
    pub fn payload_path(&self) -> PathBuf {
        outbox_dir()
            .join(&self.revision_id)
            .join(&self.payload_file)
    }
}

pub fn enqueue_project_snapshot(
    project: &ProjectFile,
    project_version_id: &str,
    workplace_id: &str,
    snapshot_path: &Path,
) -> Result<OutboxEntry, AppError> {
    let payload = std::fs::read(snapshot_path).map_err(|error| {
        AppError::wrap(
            "outbox_snapshot_read",
            "Gespeicherter Projektstand konnte nicht für Charon vorgemerkt werden.",
            error.to_string(),
        )
    })?;
    let root = outbox_dir();
    std::fs::create_dir_all(&root).map_err(outbox_write_error)?;
    let previous = latest_for_project(&root, &project.id)?;
    let revision_id = datetime::gen_id();
    let entry = OutboxEntry {
        revision_id: revision_id.clone(),
        project_id: project.id.clone(),
        project_name: project.name.clone(),
        project_version_id: project_version_id.to_owned(),
        parent_revision_id: previous.map(|entry| entry.revision_id),
        workplace_id: workplace_id.to_owned(),
        queued_at: datetime::now_iso8601(),
        content_hash: content_hash(&payload),
        payload_file: PAYLOAD_FILE.into(),
        status: OutboxStatus::Pending,
        last_error: None,
    };

    let temp_dir = root.join(format!(".{}.tmp", entry.revision_id));
    let final_dir = root.join(&entry.revision_id);
    std::fs::create_dir(&temp_dir).map_err(outbox_write_error)?;
    let result = (|| {
        std::fs::write(temp_dir.join(PAYLOAD_FILE), &payload).map_err(outbox_write_error)?;
        let manifest = serde_json::to_vec_pretty(&entry).map_err(|error| {
            AppError::wrap(
                "outbox_json",
                "Charon-Outbox konnte nicht serialisiert werden.",
                error.to_string(),
            )
        })?;
        std::fs::write(temp_dir.join(MANIFEST_FILE), manifest).map_err(outbox_write_error)?;
        std::fs::rename(&temp_dir, &final_dir).map_err(outbox_write_error)
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
    result?;
    Ok(entry)
}

pub fn list_outbox() -> Result<Vec<OutboxEntry>, AppError> {
    read_entries(&outbox_dir())
}

/// Stellt sicher, dass jede lokal gespeicherte Projektversion mindestens einen
/// unveränderlichen Outbox-Snapshot besitzt. Das ermöglicht den vollständigen
/// Wiederaufbau eines leeren Charon, ohne das Projekt erneut zu speichern.
pub(crate) fn seed_saved_projects(workplace_id: &str) -> Result<(), AppError> {
    let projects_dir = luxifer_core::projects_dir();
    let mut known = list_outbox()?
        .into_iter()
        .map(|entry| {
            (
                entry.project_id,
                entry.project_version_id,
                entry.content_hash,
            )
        })
        .collect::<Vec<_>>();
    for info in luxifer_core::list_projects(&projects_dir) {
        let project = ProjectFile::load_by_name(&projects_dir, &info.name).map_err(|error| {
            AppError::wrap(
                "project_inventory_read",
                format!("Projekt {} konnte nicht abgeglichen werden.", info.name),
                error,
            )
        })?;
        for version in &project.versions {
            let snapshot = projects_dir
                .join(&project.name)
                .join(luxifer_core::project::VERSIONS_DIR)
                .join(format!("{}.luxi", version.id));
            let payload = std::fs::read(&snapshot).map_err(|error| {
                AppError::wrap(
                    "project_inventory_read",
                    "Gespeicherte Projektversion konnte nicht abgeglichen werden.",
                    error.to_string(),
                )
            })?;
            let hash = content_hash(&payload);
            if known.iter().any(|(project_id, version_id, content_hash)| {
                project_id == &project.id && version_id == &version.id && content_hash == &hash
            }) {
                continue;
            }
            let entry = enqueue_project_snapshot(&project, &version.id, workplace_id, &snapshot)?;
            known.push((
                entry.project_id,
                entry.project_version_id,
                entry.content_hash,
            ));
        }
    }
    Ok(())
}

pub(crate) fn set_outbox_status(
    revision_id: &str,
    status: OutboxStatus,
    last_error: Option<String>,
) -> Result<(), AppError> {
    let dir = outbox_dir().join(revision_id);
    let manifest_path = dir.join(MANIFEST_FILE);
    let bytes = std::fs::read(&manifest_path).map_err(outbox_read_error)?;
    let mut entry: OutboxEntry = serde_json::from_slice(&bytes).map_err(|error| {
        AppError::wrap(
            "outbox_json",
            "Charon-Outbox enthält ungültige Daten.",
            error.to_string(),
        )
    })?;
    entry.status = status;
    entry.last_error = last_error;
    let bytes = serde_json::to_vec_pretty(&entry).map_err(|error| {
        AppError::wrap(
            "outbox_json",
            "Charon-Outbox konnte nicht serialisiert werden.",
            error.to_string(),
        )
    })?;
    let temp_path = dir.join(".manifest.tmp");
    std::fs::write(&temp_path, bytes).map_err(outbox_write_error)?;
    std::fs::rename(temp_path, manifest_path).map_err(outbox_write_error)
}

fn latest_for_project(root: &Path, project_id: &str) -> Result<Option<OutboxEntry>, AppError> {
    Ok(read_entries(root)?
        .into_iter()
        .filter(|entry| entry.project_id == project_id)
        .max_by(|a, b| a.revision_id.cmp(&b.revision_id)))
}

fn read_entries(root: &Path) -> Result<Vec<OutboxEntry>, AppError> {
    let dirs = match std::fs::read_dir(root) {
        Ok(dirs) => dirs,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(outbox_read_error(error)),
    };
    let mut entries = Vec::new();
    for dir in dirs {
        let dir = dir.map_err(outbox_read_error)?;
        if !dir.file_type().map_err(outbox_read_error)?.is_dir()
            || dir.file_name().to_string_lossy().starts_with('.')
        {
            continue;
        }
        let bytes = std::fs::read(dir.path().join(MANIFEST_FILE)).map_err(outbox_read_error)?;
        let entry: OutboxEntry = serde_json::from_slice(&bytes).map_err(|error| {
            AppError::wrap(
                "outbox_json",
                "Charon-Outbox enthält ungültige Daten.",
                error.to_string(),
            )
        })?;
        entries.push(entry);
    }
    entries.sort_by(|a, b| a.revision_id.cmp(&b.revision_id));
    Ok(entries)
}

fn outbox_dir() -> PathBuf {
    data_root().join(OUTBOX_DIR)
}

fn outbox_write_error(error: std::io::Error) -> AppError {
    AppError::wrap(
        "outbox_write",
        "Charon-Outbox konnte nicht geschrieben werden.",
        error.to_string(),
    )
}

fn outbox_read_error(error: std::io::Error) -> AppError {
    AppError::wrap(
        "outbox_read",
        "Charon-Outbox konnte nicht gelesen werden.",
        error.to_string(),
    )
}
