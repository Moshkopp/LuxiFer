//! Dateibasierte, lokale Persistenz fuer app-globale Einstellungen.
//!
//! Der Core besitzt weiterhin Format, Migration und Validierung; Pfade und
//! Dateisystemzugriffe gehoeren der Application-Schicht.

use std::path::{Path, PathBuf};

use studio_core::laser::LASER_FILE;
use studio_core::ui_settings::{UI_FORMAT_VERSION, UI_SETTINGS_FILE};
use studio_core::{LaserRegistry, UiSettings};

pub fn load_ui_settings() -> UiSettings {
    load_ui_settings_from(&studio_core::data_root())
}

pub fn save_ui_settings(settings: &UiSettings) -> Result<PathBuf, String> {
    save_ui_settings_to(settings, &studio_core::data_root())
}

pub(crate) fn save_ui_settings_to(settings: &UiSettings, dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    let path = dir.join(UI_SETTINGS_FILE);
    std::fs::write(&path, settings.to_json()?).map_err(|error| error.to_string())?;
    Ok(path)
}

pub(crate) fn load_ui_settings_from(dir: &Path) -> UiSettings {
    let path = dir.join(UI_SETTINGS_FILE);
    match std::fs::read_to_string(&path) {
        Ok(json) => {
            let raw = serde_json::from_str::<serde_json::Value>(&json).ok();
            let had_workplace_id = raw
                .as_ref()
                .and_then(|value| value.get("workplace_id"))
                .is_some();
            let needs_format_upgrade = raw
                .as_ref()
                .and_then(|value| value.get("version"))
                .and_then(|version| version.as_u64())
                .is_none_or(|version| version < UI_FORMAT_VERSION as u64);
            let settings = UiSettings::from_json(&json).unwrap_or_default();
            if !had_workplace_id || needs_format_upgrade {
                let _ = save_ui_settings_to(&settings, dir);
            }
            settings
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let settings = UiSettings::default();
            let _ = save_ui_settings_to(&settings, dir);
            settings
        }
        Err(_) => UiSettings::default(),
    }
}

pub(crate) fn load_laser_registry() -> LaserRegistry {
    load_laser_registry_from(&studio_core::data_root())
}

pub(crate) fn save_laser_registry(registry: &LaserRegistry) -> Result<PathBuf, String> {
    save_laser_registry_to(registry, &studio_core::data_root())
}

pub(crate) fn load_laser_registry_from(dir: &Path) -> LaserRegistry {
    let registry: LaserRegistry = match std::fs::read_to_string(dir.join(LASER_FILE)) {
        Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
        Err(_) => LaserRegistry::default(),
    };
    if registry
        .profiles
        .iter()
        .any(|profile| profile.validate_saved_origins().is_err())
    {
        LaserRegistry::default()
    } else {
        registry
    }
}

pub(crate) fn save_laser_registry_to(
    registry: &LaserRegistry,
    dir: &Path,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    let path = dir.join(LASER_FILE);
    let json = serde_json::to_string_pretty(registry).map_err(|error| error.to_string())?;
    std::fs::write(&path, json).map_err(|error| error.to_string())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::{LaserProfile, SavedOrigin};

    #[test]
    fn ui_settings_roundtrip() {
        let _guard = crate::test_env::with_temp_dir("settings-persistence");
        let dir = studio_core::data_root();
        let settings = UiSettings {
            workplace: "Werkstatt-PC".into(),
            ..UiSettings::default()
        };
        save_ui_settings_to(&settings, &dir).unwrap();
        assert_eq!(load_ui_settings_from(&dir).workplace, "Werkstatt-PC");
    }

    #[test]
    fn fehlende_ui_settings_erzeugen_stabilen_default() {
        let _guard = crate::test_env::with_temp_dir("settings-missing");
        let dir = studio_core::data_root();
        let first = load_ui_settings_from(&dir);
        assert!(!first.workplace_id.is_empty());
        assert_eq!(load_ui_settings_from(&dir).workplace_id, first.workplace_id);
    }

    #[test]
    fn laser_registry_roundtrip() {
        let _guard = crate::test_env::with_temp_dir("laser-persistence");
        let dir = studio_core::data_root();
        let mut registry = LaserRegistry::default();
        let profile = LaserProfile {
            id: "laser-a".into(),
            name: "Laser A".into(),
            ..LaserProfile::default()
        };
        registry.add(profile);
        save_laser_registry_to(&registry, &dir).unwrap();
        assert_eq!(load_laser_registry_from(&dir), registry);
    }

    #[test]
    fn beschaedigte_laser_registry_wird_abgelehnt() {
        let _guard = crate::test_env::with_temp_dir("laser-invalid");
        let dir = studio_core::data_root();
        let mut registry = LaserRegistry::default();
        let mut profile = LaserProfile {
            id: "laser-a".into(),
            name: "Laser A".into(),
            ..LaserProfile::default()
        };
        profile.saved_origins = vec![
            SavedOrigin {
                id: "doppelt".into(),
                name: "A".into(),
                x_mm: 1.0,
                y_mm: 1.0,
            },
            SavedOrigin {
                id: "doppelt".into(),
                name: "B".into(),
                x_mm: 2.0,
                y_mm: 2.0,
            },
        ];
        registry.add(profile);
        save_laser_registry_to(&registry, &dir).unwrap();
        assert_eq!(load_laser_registry_from(&dir), LaserRegistry::default());
    }
}
