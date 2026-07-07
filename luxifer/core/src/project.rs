//! Projektdatei: Speichern/Laden als JSON.
//!
//! Angelehnt an ThorBurns `core/project.rs` (docs/referenz/01-thorburn-analyse.md
//! §3): ein Ordner pro Projekt, darin `projekt.luxi` (JSON) mit Layer- und
//! Shape-Arrays. Bilder folgen später (mit dem Raster-/Job-Teil).
//!
//! Da `Layer`, `Shape` und `Geo` bereits `Serialize`/`Deserialize` sind, ist das
//! Format eine schlanke, versionierte Hülle um den `AppState`.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::model::{Layer, Shape};
use crate::state::AppState;

/// Dateiname der Projektdatei innerhalb des Projektordners.
pub const PROJECT_FILE: &str = "projekt.luxi";

/// Unterordner für festgehaltene Versionen (Snapshot + Thumbnail).
pub const VERSIONS_DIR: &str = "versions";

/// Aktuelle Formatversion.
pub const FORMAT_VERSION: u32 = 1;

/// Erzeugt eine stabile, praktisch eindeutige ID ohne Fremd-Crate (ADR 0003).
///
/// Aufbau: `lx-<zeit-hex>-<zufall-hex>`. Die Zeitkomponente (ns seit Epoche)
/// sorgt für grobe Sortierbarkeit, der Zufallsteil verhindert Kollisionen bei
/// schnellen Aufrufen. Reicht für lokale Identität und späteren Charon-Abgleich.
pub fn gen_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    // Einfacher, ausreichend gestreuter Zufall aus Adresse + Zeit (kein rand-Crate).
    let seed = nanos ^ ((&nanos as *const _ as u128).wrapping_mul(0x9E37_79B9));
    let rand = splitmix64(seed as u64);
    format!("lx-{:x}-{:x}", nanos as u64, rand)
}

/// Kleiner SplitMix64-Streuer für die Zufallskomponente von `gen_id`.
fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Aktueller Zeitpunkt als ISO-8601-artiger UTC-String (`YYYY-MM-DDTHH:MM:SSZ`).
///
/// Bewusst ohne `chrono`/`time`: rechnet aus den Sekunden seit Epoche selbst
/// (proleptischer gregorianischer Kalender). Genügt für Anzeige und Vergleich.
pub fn now_iso8601() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_iso8601(secs)
}

/// Formatiert Sekunden-seit-Epoche als `YYYY-MM-DDTHH:MM:SSZ` (UTC).
fn format_iso8601(secs: u64) -> String {
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (hh, mm, ss) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (year, month, day) = civil_from_days(days as i64);
    format!("{year:04}-{month:02}-{day:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

/// Tage seit 1970-01-01 → (Jahr, Monat, Tag). Howard Hinnants Standard-Algorithmus.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Kurzinfo einer festgehaltenen Version (ADR 0003 §1). Thumbnail liegt als
/// Datei `versions/<id>.png` daneben — nicht im JSON, damit es schlank bleibt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionInfo {
    pub id: String,
    pub created_at: String,
    #[serde(default)]
    pub note: String,
}

/// Serialisierbare Projektdatei (ADR 0003).
///
/// Neue Felder tragen `#[serde(default)]`, damit ältere Dateien ohne Migration
/// laden (Format-Invariante: vorwärts-tolerant).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFile {
    pub version: u32,
    /// Stabile Identität (unveränderlich über Umbenennen). Siehe [`gen_id`].
    #[serde(default = "gen_id")]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Erstellungszeit (ISO-8601 UTC), einmalig gesetzt.
    #[serde(default)]
    pub created_at: String,
    /// Letzte Änderung (ISO-8601 UTC), bei jedem Speichern aktualisiert.
    #[serde(default)]
    pub modified_at: String,
    /// Verweise auf Assets in der zentralen Bibliothek (nur IDs, nie Kopien).
    /// Vorerst leer — der Store kommt mit dem Import (eigene ADR).
    #[serde(default)]
    pub asset_refs: Vec<String>,
    /// Historie bewusst festgehaltener Versionen (Shift+Strg+S).
    #[serde(default)]
    pub versions: Vec<VersionInfo>,
    pub bed_w_mm: f64,
    pub bed_h_mm: f64,
    pub layers: Vec<Layer>,
    pub shapes: Vec<Shape>,
}

impl ProjectFile {
    /// Baut eine **neue** Projektdatei aus dem aktuellen Zustand (frische ID +
    /// Zeitstempel). Für das erste Speichern eines Projekts.
    pub fn from_state(state: &AppState, name: impl Into<String>, tags: Vec<String>) -> Self {
        let now = now_iso8601();
        Self {
            version: FORMAT_VERSION,
            id: gen_id(),
            name: name.into(),
            description: String::new(),
            tags,
            created_at: now.clone(),
            modified_at: now,
            asset_refs: Vec::new(),
            versions: Vec::new(),
            bed_w_mm: state.bed_w_mm,
            bed_h_mm: state.bed_h_mm,
            layers: state.layers.clone(),
            shapes: state.shapes.clone(),
        }
    }

    /// Übernimmt den aktuellen Arbeitsstand (Geometrie + Bett) in eine bereits
    /// existierende Projektdatei und aktualisiert `modified_at`. Identität,
    /// Metadaten und Versionshistorie bleiben erhalten (normales Speichern).
    pub fn update_from_state(&mut self, state: &AppState) {
        self.bed_w_mm = state.bed_w_mm;
        self.bed_h_mm = state.bed_h_mm;
        self.layers = state.layers.clone();
        self.shapes = state.shapes.clone();
        self.modified_at = now_iso8601();
    }

    /// Erzeugt einen frischen `AppState` aus der Projektdatei (leerer Undo-Verlauf,
    /// `dirty = false`).
    pub fn into_state(self) -> AppState {
        let mut state = AppState::new();
        state.active_layer = self.layers.len().saturating_sub(1);
        state.layers = self.layers;
        state.shapes = self.shapes;
        state.bed_w_mm = self.bed_w_mm;
        state.bed_h_mm = self.bed_h_mm;
        state.dirty = false;
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

    /// Lädt ein Projekt über seinen Ordnernamen aus `projects_dir`.
    pub fn load_by_name(projects_dir: &Path, name: &str) -> Result<Self, String> {
        Self::load(&projects_dir.join(name).join(PROJECT_FILE))
    }

    /// Hält den aktuellen Arbeitsstand als **neue Version** fest (Shift+Strg+S):
    /// schreibt `versions/<id>.luxi` (Snapshot) + `versions/<id>.png` (Thumbnail),
    /// hängt eine [`VersionInfo`] an und speichert die Projektdatei neu.
    ///
    /// `thumb_png` sind fertige PNG-Bytes aus dem Frontend (der Core zeichnet
    /// nicht selbst). `note` ist eine optionale Kurznotiz.
    pub fn add_version(
        &mut self,
        projects_dir: &Path,
        note: impl Into<String>,
        thumb_png: &[u8],
    ) -> Result<VersionInfo, String> {
        let proj_dir = projects_dir.join(&self.name);
        let vdir = proj_dir.join(VERSIONS_DIR);
        std::fs::create_dir_all(&vdir).map_err(|e| e.to_string())?;

        let info = VersionInfo {
            id: gen_id(),
            created_at: now_iso8601(),
            note: note.into(),
        };
        // Snapshot: das aktuelle ProjectFile MIT der bereits angehängten Version,
        // damit ein geladener Snapshot dieselbe Historie kennt.
        self.versions.push(info.clone());
        self.modified_at = now_iso8601();

        let snap = vdir.join(format!("{}.luxi", info.id));
        std::fs::write(&snap, self.to_json()?).map_err(|e| e.to_string())?;
        if !thumb_png.is_empty() {
            let png = vdir.join(format!("{}.png", info.id));
            std::fs::write(&png, thumb_png).map_err(|e| e.to_string())?;
        }
        // Projektdatei mit aktualisierter Historie schreiben.
        self.save_to_dir(projects_dir)?;
        Ok(info)
    }

    /// Lädt den Snapshot einer festgehaltenen Version.
    pub fn load_version(projects_dir: &Path, name: &str, version_id: &str) -> Result<Self, String> {
        let path = projects_dir
            .join(name)
            .join(VERSIONS_DIR)
            .join(format!("{version_id}.luxi"));
        Self::load(&path)
    }
}

/// Pfad zum Thumbnail einer Version (`versions/<id>.png`) oder `None`, wenn es
/// keins gibt. Für die Anzeige der Versionsliste im Frontend.
pub fn version_thumb_path(projects_dir: &Path, name: &str, version_id: &str) -> Option<PathBuf> {
    let p = projects_dir
        .join(name)
        .join(VERSIONS_DIR)
        .join(format!("{version_id}.png"));
    p.exists().then_some(p)
}

/// Benennt einen Projektordner um. Die Projekt-`id` bleibt unberührt (Identität
/// hängt an der ID, nicht am Namen — ADR 0003 Invariante 1). Aktualisiert das
/// `name`-Feld in der Projektdatei mit.
pub fn rename_project(projects_dir: &Path, old_name: &str, new_name: &str) -> Result<(), String> {
    if new_name.trim().is_empty() {
        return Err("Neuer Name darf nicht leer sein.".into());
    }
    let old_dir = projects_dir.join(old_name);
    let new_dir = projects_dir.join(new_name);
    if new_dir.exists() {
        return Err(format!("Projekt „{new_name}“ existiert bereits."));
    }
    std::fs::rename(&old_dir, &new_dir).map_err(|e| e.to_string())?;
    // name-Feld in der Datei nachziehen.
    let mut pf = ProjectFile::load_by_name(projects_dir, new_name)?;
    pf.name = new_name.to_string();
    pf.modified_at = now_iso8601();
    pf.save_to_dir(projects_dir)?;
    Ok(())
}

/// Löscht einen Projektordner samt Versionen.
pub fn delete_project(projects_dir: &Path, name: &str) -> Result<(), String> {
    let dir = projects_dir.join(name);
    std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())
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

/// Kurzinfo eines Projekts für die Listenansicht (links im Browser).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProjectInfo {
    pub name: String,
    pub tags: Vec<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub modified_at: String,
}

/// Listet alle Projekte unter `projects_dir()`. Sortiert nach zuletzt geändert
/// (neueste zuerst), damit das aktivste Projekt oben steht.
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
            let pf = ProjectFile::load(&file).ok()?;
            Some(ProjectInfo {
                name,
                tags: pf.tags,
                description: pf.description,
                modified_at: pf.modified_at,
            })
        })
        .collect();
    // Neueste zuerst; bei gleichem/leerem Datum alphabetisch als Fallback.
    infos.sort_by(|a, b| {
        b.modified_at
            .cmp(&a.modified_at)
            .then_with(|| a.name.cmp(&b.name))
    });
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

    #[test]
    fn from_state_hat_id_und_zeitstempel() {
        let s = state_with_two_layers();
        let pf = ProjectFile::from_state(&s, "T", vec![]);
        assert!(pf.id.starts_with("lx-"));
        assert!(pf.created_at.ends_with('Z'));
        assert_eq!(pf.created_at, pf.modified_at);
        assert!(pf.versions.is_empty());
        assert!(pf.asset_refs.is_empty());
    }

    #[test]
    fn gen_id_ist_eindeutig() {
        let a = gen_id();
        let b = gen_id();
        assert_ne!(a, b);
    }

    #[test]
    fn iso8601_formatiert_bekannten_zeitpunkt() {
        // 2021-01-01T00:00:00Z = 1609459200 Sekunden seit Epoche.
        assert_eq!(format_iso8601(1_609_459_200), "2021-01-01T00:00:00Z");
        // Epoche selbst.
        assert_eq!(format_iso8601(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn alte_json_ohne_neue_felder_laedt() {
        // Minimal-JSON wie aus der Zeit vor ADR 0003 (ohne id/versions/…).
        let json = r#"{
            "version": 1, "name": "Alt", "tags": ["x"],
            "bed_w_mm": 300.0, "bed_h_mm": 200.0,
            "layers": [], "shapes": []
        }"#;
        let pf = ProjectFile::from_json(json).unwrap();
        assert_eq!(pf.name, "Alt");
        assert!(pf.id.starts_with("lx-"), "id per serde-default erzeugt");
        assert!(pf.versions.is_empty());
        assert!(pf.description.is_empty());
    }

    #[test]
    fn version_anlegen_und_laden() {
        let dir = std::env::temp_dir().join(format!("luxifer_ver_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let s = state_with_two_layers();
        let mut pf = ProjectFile::from_state(&s, "Proj", vec![]);
        pf.save_to_dir(&dir).unwrap();

        let info = pf
            .add_version(&dir, "erster Stand", b"\x89PNG-fake")
            .unwrap();
        assert_eq!(pf.versions.len(), 1);
        assert_eq!(pf.versions[0].id, info.id);
        // Snapshot + Thumbnail liegen auf der Platte.
        assert!(version_thumb_path(&dir, "Proj", &info.id).is_some());
        let snap = ProjectFile::load_version(&dir, "Proj", &info.id).unwrap();
        assert_eq!(snap.shapes.len(), 2);
        assert_eq!(snap.versions.len(), 1, "Snapshot kennt die eigene Version");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rename_erhaelt_id_und_verschiebt_ordner() {
        let dir = std::env::temp_dir().join(format!("luxifer_ren_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let s = state_with_two_layers();
        let mut pf = ProjectFile::from_state(&s, "Alt", vec![]);
        pf.save_to_dir(&dir).unwrap();
        let id_vorher = pf.id.clone();

        rename_project(&dir, "Alt", "Neu").unwrap();
        assert!(!dir.join("Alt").exists());
        let geladen = ProjectFile::load_by_name(&dir, "Neu").unwrap();
        assert_eq!(geladen.name, "Neu");
        assert_eq!(geladen.id, id_vorher, "id bleibt stabil (Invariante 1)");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_entfernt_projekt() {
        let dir = std::env::temp_dir().join(format!("luxifer_del_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let s = state_with_two_layers();
        ProjectFile::from_state(&s, "Weg", vec![])
            .save_to_dir(&dir)
            .unwrap();
        assert_eq!(list_projects(&dir).len(), 1);
        delete_project(&dir, "Weg").unwrap();
        assert_eq!(list_projects(&dir).len(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn update_from_state_erhaelt_id_und_versionen() {
        let s = state_with_two_layers();
        let mut pf = ProjectFile::from_state(&s, "P", vec![]);
        let id = pf.id.clone();
        pf.versions.push(VersionInfo {
            id: "v1".into(),
            created_at: "2021-01-01T00:00:00Z".into(),
            note: String::new(),
        });
        // Neuer Arbeitsstand mit nur einem Shape.
        let mut s2 = AppState::new();
        s2.add_shape(Geo::Rect {
            x: 0.0,
            y: 0.0,
            w: 5.0,
            h: 5.0,
        });
        pf.update_from_state(&s2);
        assert_eq!(pf.id, id, "Identitaet bleibt");
        assert_eq!(pf.versions.len(), 1, "Historie bleibt");
        assert_eq!(pf.shapes.len(), 1, "Arbeitsstand ersetzt");
    }
}
