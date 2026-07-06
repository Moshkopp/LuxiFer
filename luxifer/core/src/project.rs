//! Projektdatei: Speichern/Laden als JSON.
//!
//! Angelehnt an ThorBurns `core/project.rs` (docs/referenz/01-thorburn-analyse.md
//! §3): ein Ordner pro Projekt, darin `projekt.luxi` (JSON) mit Layer- und
//! Shape-Arrays. Bilder folgen später (mit dem Raster-/Job-Teil).
//!
//! Da `Layer`, `Shape` und `Geo` bereits `Serialize`/`Deserialize` sind, ist das
//! Format eine schlanke, versionierte Hülle um den `AppState`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::{Layer, Shape};
use crate::state::AppState;

/// Dateiname der Projektdatei innerhalb des Projektordners.
pub const PROJECT_FILE: &str = "projekt.luxi";

/// Aktuelle Formatversion.
pub const FORMAT_VERSION: u32 = 1;

/// Serialisierbare Projektdatei.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFile {
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub bed_w_mm: f64,
    pub bed_h_mm: f64,
    pub layers: Vec<Layer>,
    pub shapes: Vec<Shape>,
}

impl ProjectFile {
    /// Baut die Projektdatei aus dem aktuellen Zustand.
    pub fn from_state(state: &AppState, name: impl Into<String>, tags: Vec<String>) -> Self {
        Self {
            version: FORMAT_VERSION,
            name: name.into(),
            tags,
            bed_w_mm: state.bed_w_mm,
            bed_h_mm: state.bed_h_mm,
            layers: state.layers.clone(),
            shapes: state.shapes.clone(),
        }
    }

    /// Erzeugt einen frischen `AppState` aus der Projektdatei (leerer Undo-Verlauf).
    pub fn into_state(self) -> AppState {
        let mut state = AppState::new();
        state.active_layer = self.layers.len().saturating_sub(1);
        state.layers = self.layers;
        state.shapes = self.shapes;
        state.bed_w_mm = self.bed_w_mm;
        state.bed_h_mm = self.bed_h_mm;
        state
    }

    /// JSON-Text (hübsch formatiert).
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    /// Aus JSON-Text.
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| e.to_string())
    }

    /// Schreibt die Projektdatei nach `<dir>/<name>/projekt.luxi`.
    pub fn save_to_dir(&self, projects_dir: &Path) -> Result<PathBuf, String> {
        let proj_dir = projects_dir.join(&self.name);
        std::fs::create_dir_all(&proj_dir).map_err(|e| e.to_string())?;
        let path = proj_dir.join(PROJECT_FILE);
        std::fs::write(&path, self.to_json()?).map_err(|e| e.to_string())?;
        Ok(path)
    }

    /// Lädt eine Projektdatei aus einem Pfad.
    pub fn load(path: &Path) -> Result<Self, String> {
        let json = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        Self::from_json(&json)
    }
}

/// Basis-Datenverzeichnis. Reihenfolge: `LUXIFER_DATA_DIR` → `$XDG_DATA_HOME/luxifer`
/// → `$HOME/.local/share/luxifer` → `.` (Notnagel). Plattformneutral.
pub fn data_root() -> PathBuf {
    if let Ok(dir) = std::env::var("LUXIFER_DATA_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("luxifer");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("luxifer");
        }
    }
    PathBuf::from(".")
}

/// Projektordner (`<data_root>/Projekte`).
pub fn projects_dir() -> PathBuf {
    data_root().join("Projekte")
}

/// Kurzinfo eines Projekts für die Listenansicht.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectInfo {
    pub name: String,
    pub tags: Vec<String>,
}

/// Listet alle Projekte unter `projects_dir()`, alphabetisch nach Name.
pub fn list_projects(projects_dir: &Path) -> Vec<ProjectInfo> {
    let Ok(entries) = std::fs::read_dir(projects_dir) else {
        return vec![];
    };
    let mut infos: Vec<ProjectInfo> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let file = e.path().join(PROJECT_FILE);
            if !file.exists() {
                return None;
            }
            let name = e.file_name().into_string().ok()?;
            let tags = ProjectFile::load(&file).map(|p| p.tags).unwrap_or_default();
            Some(ProjectInfo { name, tags })
        })
        .collect();
    infos.sort_by(|a, b| a.name.cmp(&b.name));
    infos
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Geo;

    fn state_with_two_layers() -> AppState {
        let mut s = AppState::new();
        s.add_shape(Geo::Rect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
        });
        s.selected.clear();
        s.activate_color([0x3B, 0x82, 0xF6]); // pending blau
        s.add_shape(Geo::Ellipse {
            cx: 50.0,
            cy: 50.0,
            rx: 20.0,
            ry: 10.0,
        });
        s
    }

    #[test]
    fn roundtrip_json_erhaelt_layer_und_shapes() {
        let s = state_with_two_layers();
        let pf = ProjectFile::from_state(&s, "Test", vec!["deko".into()]);
        let json = pf.to_json().unwrap();
        let back = ProjectFile::from_json(&json).unwrap();
        assert_eq!(pf, back);
        assert_eq!(back.layers.len(), 2);
        assert_eq!(back.shapes.len(), 2);
        assert_eq!(back.tags, vec!["deko".to_string()]);
    }

    #[test]
    fn into_state_setzt_aktiven_layer_auf_letzten() {
        let s = state_with_two_layers();
        let pf = ProjectFile::from_state(&s, "Test", vec![]);
        let restored = pf.into_state();
        assert_eq!(restored.layers.len(), 2);
        assert_eq!(restored.active_layer, 1);
        assert!(!restored.can_undo(), "frischer Undo-Verlauf");
    }

    #[test]
    fn save_und_load_ueber_tempdir() {
        let dir = std::env::temp_dir().join(format!("luxifer_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let s = state_with_two_layers();
        let pf = ProjectFile::from_state(&s, "MeinProjekt", vec!["a".into()]);
        let path = pf.save_to_dir(&dir).unwrap();
        assert!(path.exists());

        let loaded = ProjectFile::load(&path).unwrap();
        assert_eq!(loaded.name, "MeinProjekt");
        assert_eq!(loaded.shapes.len(), 2);

        let infos = list_projects(&dir);
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].name, "MeinProjekt");
        assert_eq!(infos[0].tags, vec!["a".to_string()]);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
