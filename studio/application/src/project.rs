//! Projekt-, Versions- und Asset-Lebenszyklus als UI-unabhängiger Dienst
//! (ADR 0011, Phase 3). Kapselt das offene Projekt und koordiniert die
//! Core-Projekt-API (`ProjectFile`, `list_projects`, `rename_project`,
//! `delete_project`). Fehler werden als stabiler [`AppError`] gemeldet, nicht
//! als roher String.
//!
//! Speichern ist bewusst manuell (kein Autosave): `save` schreibt die aktuelle
//! Version in-place, `save_version` legt eine neue an. Der Dirty-Schutz ist eine
//! reine Abfrage (`AppState::dirty`); die Warn-/Abbruch-Entscheidung trifft die
//! aufrufende Oberfläche.

use std::path::{Path, PathBuf};

use studio_core::{project::ProjectFile, project::VersionInfo, state::AppState, ProjectInfo};

use crate::AppError;

pub(crate) fn projects_path() -> PathBuf {
    studio_core::projects_dir()
}

pub(crate) fn list_project_infos() -> Vec<ProjectInfo> {
    let Ok(entries) = std::fs::read_dir(projects_path()) else {
        return Vec::new();
    };
    let mut projects = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let project = load_project(&name).ok()?;
            Some(ProjectInfo {
                name,
                tags: project.tags,
                description: project.description,
                modified_at: project.modified_at,
            })
        })
        .collect::<Vec<_>>();
    projects.sort_by(|left, right| {
        right
            .modified_at
            .cmp(&left.modified_at)
            .then_with(|| left.name.cmp(&right.name))
    });
    projects
}

pub(crate) fn load_project(name: &str) -> Result<ProjectFile, String> {
    let path = projects_path()
        .join(name)
        .join(studio_core::project::PROJECT_FILE);
    let json = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    ProjectFile::from_json(&json)
}

pub(crate) fn load_project_version(name: &str, version_id: &str) -> Result<ProjectFile, String> {
    let path = projects_path()
        .join(name)
        .join(studio_core::project::VERSIONS_DIR)
        .join(format!("{version_id}.laserproj"));
    let json = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    ProjectFile::from_json(&json)
}

pub(crate) fn list_project_files() -> Result<Vec<ProjectFile>, String> {
    list_project_infos()
        .into_iter()
        .map(|info| load_project(&info.name))
        .collect()
}

pub(crate) fn save_project_file(project: &ProjectFile) -> Result<PathBuf, String> {
    let directory = projects_path().join(&project.name);
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let path = directory.join(studio_core::project::PROJECT_FILE);
    std::fs::write(&path, project.to_json()?).map_err(|error| error.to_string())?;
    Ok(path)
}

fn save_current_project(project: &mut ProjectFile) -> Result<VersionInfo, String> {
    let version = project.prepare_save_current()?;
    write_version_snapshot(project, &version.id, &[])?;
    save_project_file(project)?;
    Ok(version)
}

fn add_project_version(project: &mut ProjectFile) -> Result<VersionInfo, String> {
    let version = project.add_version_metadata(String::new());
    write_version_snapshot(project, &version.id, &[])?;
    save_project_file(project)?;
    Ok(version)
}

fn delete_project_version(
    project: &mut ProjectFile,
    version_id: &str,
) -> Result<Option<ProjectFile>, String> {
    let promoted_id = project.remove_version_metadata(version_id)?;
    let versions = projects_path()
        .join(&project.name)
        .join(studio_core::project::VERSIONS_DIR);
    let _ = std::fs::remove_file(versions.join(format!("{version_id}.laserproj")));
    let _ = std::fs::remove_file(versions.join(format!("{version_id}.png")));

    let promoted = if let Some(promoted_id) = promoted_id {
        let snapshot = load_project_version(&project.name, &promoted_id)?;
        project.apply_version_geometry(&snapshot);
        Some(snapshot)
    } else {
        None
    };
    save_project_file(project)?;
    Ok(promoted)
}

fn write_version_snapshot(
    project: &ProjectFile,
    version_id: &str,
    thumbnail_png: &[u8],
) -> Result<(), String> {
    let directory = projects_path()
        .join(&project.name)
        .join(studio_core::project::VERSIONS_DIR);
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    std::fs::write(
        directory.join(format!("{version_id}.laserproj")),
        project.to_json()?,
    )
    .map_err(|error| error.to_string())?;
    if !thumbnail_png.is_empty() {
        std::fs::write(directory.join(format!("{version_id}.png")), thumbnail_png)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn rename_project(old_name: &str, new_name: &str) -> Result<(), String> {
    if new_name.trim().is_empty() {
        return Err("Neuer Name darf nicht leer sein.".into());
    }
    let old_directory = projects_path().join(old_name);
    let new_directory = projects_path().join(new_name);
    if new_directory.exists() {
        return Err(format!("Projekt „{new_name}“ existiert bereits."));
    }
    std::fs::rename(old_directory, new_directory).map_err(|error| error.to_string())?;
    let mut project = load_project(new_name)?;
    project.name = new_name.to_owned();
    project.modified_at = studio_core::datetime::now_iso8601();
    save_project_file(&project)?;
    Ok(())
}

fn delete_project(name: &str) -> Result<(), String> {
    std::fs::remove_dir_all(projects_path().join(name)).map_err(|error| error.to_string())
}

/// UI-unabhängige Detailsicht eines Projekts für den Browser: Metadaten und
/// Versionsliste, ohne Geometrie. Kommt für das offene Projekt aus dem
/// Speicher, sonst aus einer nur-lesenden Dateiladung.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectDetail {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub created_at: String,
    pub modified_at: String,
    pub versions: Vec<VersionInfo>,
    /// ID der aktuellen Version (die, die im Canvas landet).
    pub current_version: String,
}

impl ProjectDetail {
    fn from_file(pf: &ProjectFile) -> Self {
        Self {
            name: pf.name.clone(),
            description: pf.description.clone(),
            tags: pf.tags.clone(),
            created_at: pf.created_at.clone(),
            modified_at: pf.modified_at.clone(),
            versions: pf.versions.clone(),
            current_version: pf.current_version.clone(),
        }
    }
}

/// Hält das offene Projekt und dessen Ablageort. Ohne offenes Projekt ist der
/// Arbeitsstand „namenlos" (erst Anlegen/Speichern vergibt einen Namen).
#[derive(Default)]
pub struct ProjectService {
    open: Option<ProjectFile>,
}

impl ProjectService {
    pub fn new() -> Self {
        Self::default()
    }

    /// Projektverzeichnis (plattformneutral über den Core bestimmt).
    fn dir() -> PathBuf {
        projects_path()
    }

    // ---- Abfragen -----------------------------------------------------------

    /// Alle Projekte, sortiert nach zuletzt geändert (neueste zuerst).
    pub fn list(&self) -> Vec<ProjectInfo> {
        list_project_infos()
    }

    /// Name des offenen Projekts, falls eines offen ist.
    pub fn open_name(&self) -> Option<&str> {
        self.open.as_ref().map(|p| p.name.as_str())
    }

    /// Ob gerade ein Projekt offen ist.
    pub fn has_open(&self) -> bool {
        self.open.is_some()
    }

    /// Löst die Bindung an das aktuell geöffnete Projekt, ohne dessen Dateien
    /// zu verändern. Der aufrufende Client kann anschließend einen leeren,
    /// ungespeicherten Editorzustand einsetzen.
    pub fn close(&mut self) {
        self.open = None;
    }

    /// Versionsliste des offenen Projekts (leer, wenn keins offen ist).
    pub fn versions(&self) -> &[VersionInfo] {
        self.open
            .as_ref()
            .map(|p| p.versions.as_slice())
            .unwrap_or(&[])
    }

    /// ID der aktuellen Version des offenen Projekts.
    pub fn current_version_id(&self) -> Option<&str> {
        self.open.as_ref().map(|p| p.current_version.as_str())
    }

    /// Friert den zuletzt erfolgreich gespeicherten Stand als unveränderliche
    /// lokale Sync-Revision ein. Ein Fehler hier macht das Projektspeichern
    /// nicht rückgängig; diese Grenze entscheidet der aufrufende Client.
    pub fn queue_current_for_sync(
        &self,
        workplace_id: &str,
    ) -> Result<crate::OutboxEntry, AppError> {
        let project = self.require_open_ref()?;
        let version_id = &project.current_version;
        let snapshot = Self::dir()
            .join(&project.name)
            .join(studio_core::project::VERSIONS_DIR)
            .join(format!("{version_id}.laserproj"));
        crate::sync_outbox::enqueue_project_snapshot(project, version_id, workplace_id, &snapshot)
    }

    /// Detailsicht eines Projekts für den Browser (Metadaten + Versionen),
    /// ohne das offene Projekt zu wechseln. Für das offene Projekt kommt die
    /// Sicht aus dem Speicher, sonst wird die Projektdatei nur gelesen.
    pub fn detail(&self, name: &str) -> Result<ProjectDetail, AppError> {
        if let Some(pf) = self.open.as_ref().filter(|p| p.name == name) {
            return Ok(ProjectDetail::from_file(pf));
        }
        let pf = load_project(name).map_err(|e| {
            AppError::wrap(
                "project_read",
                format!("Projekt {name} konnte nicht gelesen werden."),
                e,
            )
        })?;
        Ok(ProjectDetail::from_file(&pf))
    }

    /// Zustand eines Projekts nur lesen (z. B. für eine Vorschau im Browser).
    /// Wechselt das offene Projekt **nicht** und mutiert nichts.
    pub fn peek_state(&self, name: &str) -> Result<AppState, AppError> {
        let pf = load_project(name).map_err(|e| {
            AppError::wrap(
                "project_read",
                format!("Projekt {name} konnte nicht gelesen werden."),
                e,
            )
        })?;
        Ok(pf.into_state())
    }

    // ---- Lebenszyklus -------------------------------------------------------

    /// Neues Projekt aus dem aktuellen Zustand anlegen und sofort speichern.
    /// Der Name darf nicht leer sein; die Beschreibung ist optional.
    pub fn new_project(
        &mut self,
        state: &AppState,
        name: &str,
        description: &str,
    ) -> Result<(), AppError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::new(
                "project_name_empty",
                "Bitte einen Projektnamen angeben.",
            ));
        }
        let mut pf = ProjectFile::from_state(state, name, Vec::new());
        pf.description = description.trim().to_string();
        save_project_file(&pf).map_err(|e| {
            AppError::wrap("project_write", "Projekt konnte nicht angelegt werden.", e)
        })?;
        save_current_project(&mut pf).map_err(|e| {
            AppError::wrap("project_write", "Projekt konnte nicht angelegt werden.", e)
        })?;
        self.open = Some(pf);
        Ok(())
    }

    /// Projekt laden und seinen Zustand zurückgeben (der Aufrufer ersetzt den
    /// Editorzustand damit). Bei Fehler bleibt das bisher offene Projekt erhalten.
    pub fn open(&mut self, name: &str) -> Result<AppState, AppError> {
        let pf = load_project(name).map_err(|e| {
            AppError::wrap(
                "project_read",
                format!("Projekt {name} konnte nicht geöffnet werden."),
                e,
            )
        })?;
        let state = pf.clone().into_state();
        self.open = Some(pf);
        Ok(state)
    }

    /// In-place speichern (aktuelle Version). Erfordert ein offenes Projekt.
    pub fn save(&mut self, state: &AppState) -> Result<VersionInfo, AppError> {
        let pf = self.require_open_mut()?;
        pf.update_from_state(state);
        save_current_project(pf)
            .map_err(|e| AppError::wrap("project_write", "Speichern fehlgeschlagen.", e))
    }

    /// Als neue Version speichern.
    pub fn save_version(&mut self, state: &AppState) -> Result<VersionInfo, AppError> {
        let pf = self.require_open_mut()?;
        pf.update_from_state(state);
        add_project_version(pf)
            .map_err(|e| AppError::wrap("project_write", "Neue Version fehlgeschlagen.", e))
    }

    /// Eine bestimmte Version laden und ihren Zustand zurückgeben; sie wird zum
    /// kanonischen offenen Zustand.
    pub fn open_version(&mut self, version_id: &str) -> Result<AppState, AppError> {
        let name = self.require_open_ref()?.name.clone();
        let pf = load_project_version(&name, version_id).map_err(|e| {
            AppError::wrap("version_read", "Version konnte nicht geladen werden.", e)
        })?;
        let state = pf.clone().into_state();
        self.open = Some(pf);
        Ok(state)
    }

    /// Eine Version löschen. Die letzte Version schützt der Core. War es die
    /// **aktuelle** Version, befördert der Core die vorherige und gibt deren
    /// Zustand zurück — der Aufrufer MUSS den Canvas dann darauf setzen, sonst
    /// zeigt der Editor stillschweigend veraltete Geometrie.
    pub fn delete_version(&mut self, version_id: &str) -> Result<Option<AppState>, AppError> {
        let pf = self.require_open_mut()?;
        let promoted = delete_project_version(pf, version_id).map_err(|e| {
            AppError::wrap("version_delete", "Version konnte nicht gelöscht werden.", e)
        })?;
        Ok(promoted.map(|snap| snap.into_state()))
    }

    /// Projekt umbenennen. Benennt das offene Projekt bei Bedarf mit um.
    pub fn rename(&mut self, old_name: &str, new_name: &str) -> Result<(), AppError> {
        let new_name = new_name.trim();
        if new_name.is_empty() {
            return Err(AppError::new(
                "project_name_empty",
                "Bitte einen neuen Projektnamen angeben.",
            ));
        }
        rename_project(old_name, new_name)
            .map_err(|e| AppError::wrap("project_rename", "Umbenennen fehlgeschlagen.", e))?;
        if let Some(pf) = self.open.as_mut() {
            if pf.name == old_name {
                pf.name = new_name.to_string();
            }
        }
        Ok(())
    }

    /// Projekt löschen. Ist es das offene Projekt, wird es geschlossen.
    pub fn delete(&mut self, name: &str) -> Result<(), AppError> {
        delete_project(name)
            .map_err(|e| AppError::wrap("project_delete", "Löschen fehlgeschlagen.", e))?;
        if self.open_name() == Some(name) {
            self.open = None;
        }
        Ok(())
    }

    /// Die Projektdatei nach `ziel` exportieren (Kopie der `projekt.laserproj`).
    pub fn export(&self, name: &str, ziel: &Path) -> Result<(), AppError> {
        let src = Self::dir().join(name).join("projekt.laserproj");
        std::fs::copy(&src, ziel).map_err(|e| {
            AppError::wrap("project_export", "Export fehlgeschlagen.", e.to_string())
        })?;
        Ok(())
    }

    // ---- Helfer -------------------------------------------------------------

    fn require_open_ref(&self) -> Result<&ProjectFile, AppError> {
        self.open.as_ref().ok_or_else(Self::no_open_project)
    }

    fn require_open_mut(&mut self) -> Result<&mut ProjectFile, AppError> {
        self.open.as_mut().ok_or_else(Self::no_open_project)
    }

    fn no_open_project() -> AppError {
        AppError::new(
            "no_open_project",
            "Kein Projekt offen — erst anlegen oder öffnen.",
        )
    }
}

#[cfg(test)]
mod tests;
