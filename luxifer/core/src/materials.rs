//! Lokale, laserbezogene Materialprofile (ADR 0019, experimenteller Prototyp).
//! Sie gehören bewusst nicht zum Projektformat: dasselbe Design darf an einem
//! anderen Arbeitsplatz auf einem anderen Material ausgeführt werden.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Layer, LayerMode};

const MATERIAL_FILE: &str = "material-profile.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialProcess {
    Cut,
    VectorEngrave,
    RasterEngrave,
}

impl MaterialProcess {
    pub const ALL: [Self; 3] = [Self::Cut, Self::VectorEngrave, Self::RasterEngrave];

    pub fn label(self) -> &'static str {
        match self {
            Self::Cut => "Schneiden",
            Self::VectorEngrave => "Vektorgravur",
            Self::RasterEngrave => "Bildgravur",
        }
    }

    pub fn from_layer_mode(mode: LayerMode) -> Self {
        match mode {
            LayerMode::Cut => Self::Cut,
            LayerMode::Fill => Self::VectorEngrave,
            LayerMode::Raster | LayerMode::Image => Self::RasterEngrave,
        }
    }
}

/// Vollständiger Parametersnapshot für genau einen Prozess. Die Felder folgen
/// dem heutigen Layer-Modell; die UI zeigt pro Prozess nur relevante Werte.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialProcessDefaults {
    pub speed_mm_s: f64,
    pub power_pct: f64,
    pub min_power_pct: f64,
    pub passes: u32,
    pub air_assist: bool,
    pub line_step_mm: f64,
    pub dpi: f64,
    pub bidirectional: bool,
}

impl MaterialProcessDefaults {
    pub fn from_layer(layer: &Layer) -> Self {
        Self {
            speed_mm_s: layer.speed_mm_s,
            power_pct: layer.power_pct,
            min_power_pct: layer.min_power_pct,
            passes: layer.passes,
            air_assist: layer.air_assist,
            line_step_mm: layer.line_step_mm,
            dpi: layer.dpi,
            bidirectional: layer.bidirectional,
        }
    }

    pub fn apply_to(&self, layer: &mut Layer) {
        layer.speed_mm_s = self.speed_mm_s;
        layer.power_pct = self.power_pct;
        layer.min_power_pct = self.min_power_pct;
        layer.passes = self.passes;
        layer.air_assist = self.air_assist;
        layer.line_step_mm = self.line_step_mm;
        layer.dpi = self.dpi;
        layer.bidirectional = self.bidirectional;
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialProfile {
    pub id: String,
    /// Konkreter lokaler Laser. Profile werden nicht still übertragen.
    pub laser_id: String,
    pub name: String,
    pub thickness_mm: Option<f64>,
    #[serde(default)]
    pub cut: Option<MaterialProcessDefaults>,
    #[serde(default)]
    pub vector_engrave: Option<MaterialProcessDefaults>,
    #[serde(default)]
    pub raster_engrave: Option<MaterialProcessDefaults>,
}

impl MaterialProfile {
    pub fn display_name(&self) -> String {
        self.thickness_mm.map_or_else(
            || self.name.clone(),
            |thickness| format!("{} · {} mm", self.name, thickness),
        )
    }

    pub fn defaults(&self, process: MaterialProcess) -> Option<&MaterialProcessDefaults> {
        match process {
            MaterialProcess::Cut => self.cut.as_ref(),
            MaterialProcess::VectorEngrave => self.vector_engrave.as_ref(),
            MaterialProcess::RasterEngrave => self.raster_engrave.as_ref(),
        }
    }

    pub fn defaults_mut(
        &mut self,
        process: MaterialProcess,
    ) -> &mut Option<MaterialProcessDefaults> {
        match process {
            MaterialProcess::Cut => &mut self.cut,
            MaterialProcess::VectorEngrave => &mut self.vector_engrave,
            MaterialProcess::RasterEngrave => &mut self.raster_engrave,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialLibrary {
    #[serde(default = "material_format_version")]
    pub version: u32,
    #[serde(default)]
    pub profiles: Vec<MaterialProfile>,
    /// Aktive Materialwahl je Laser; damit bleibt sie lokaler Arbeitskontext.
    #[serde(default)]
    pub active_by_laser: BTreeMap<String, String>,
}

const fn material_format_version() -> u32 {
    1
}

impl Default for MaterialLibrary {
    fn default() -> Self {
        Self {
            version: material_format_version(),
            profiles: Vec::new(),
            active_by_laser: BTreeMap::new(),
        }
    }
}

impl MaterialLibrary {
    pub fn for_laser<'a>(
        &'a self,
        laser_id: &'a str,
    ) -> impl Iterator<Item = &'a MaterialProfile> + 'a {
        self.profiles
            .iter()
            .filter(move |profile| profile.laser_id == laser_id)
    }

    pub fn active_for(&self, laser_id: &str) -> Option<&MaterialProfile> {
        let id = self.active_by_laser.get(laser_id)?;
        self.profiles
            .iter()
            .find(|profile| profile.laser_id == laser_id && &profile.id == id)
    }

    pub fn set_active(&mut self, laser_id: &str, material_id: Option<&str>) -> bool {
        match material_id {
            Some(id)
                if self
                    .profiles
                    .iter()
                    .any(|profile| profile.laser_id == laser_id && profile.id == id) =>
            {
                self.active_by_laser.insert(laser_id.into(), id.into());
                true
            }
            None => {
                self.active_by_laser.remove(laser_id);
                true
            }
            Some(_) => false,
        }
    }

    pub fn load() -> Self {
        Self::load_from(&crate::project::data_root())
    }

    pub fn load_from(dir: &Path) -> Self {
        match std::fs::read_to_string(dir.join(MATERIAL_FILE)) {
            Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> Result<PathBuf, String> {
        self.save_to(&crate::project::data_root())
    }

    pub fn save_to(&self, dir: &Path) -> Result<PathBuf, String> {
        std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
        let path = dir.join(MATERIAL_FILE);
        let json = serde_json::to_string_pretty(self).map_err(|error| error.to_string())?;
        std::fs::write(&path, json).map_err(|error| error.to_string())?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> MaterialProfile {
        MaterialProfile {
            id: "pappel-3".into(),
            laser_id: "laser-a".into(),
            name: "Pappelsperrholz".into(),
            thickness_mm: Some(3.0),
            cut: Some(MaterialProcessDefaults::from_layer(&Layer::new(0))),
            vector_engrave: None,
            raster_engrave: None,
        }
    }

    #[test]
    fn materialwahl_bleibt_auf_den_laser_begrenzt() {
        let mut library = MaterialLibrary::default();
        library.profiles.push(profile());
        assert!(library.set_active("laser-a", Some("pappel-3")));
        assert_eq!(library.active_for("laser-a").unwrap().id, "pappel-3");
        assert!(library.active_for("laser-b").is_none());
        assert!(!library.set_active("laser-b", Some("pappel-3")));
    }

    #[test]
    fn parametersnapshot_uebernimmt_nur_laserwerte() {
        let mut source = Layer::new(0);
        source.speed_mm_s = 18.0;
        source.power_pct = 82.0;
        source.passes = 2;
        let defaults = MaterialProcessDefaults::from_layer(&source);
        let mut target = Layer::with_color(4, [1, 2, 3]);
        let name = target.name.clone();
        defaults.apply_to(&mut target);
        assert_eq!(target.speed_mm_s, 18.0);
        assert_eq!(target.power_pct, 82.0);
        assert_eq!(target.passes, 2);
        assert_eq!(target.color, [1, 2, 3]);
        assert_eq!(target.name, name);
    }

    #[test]
    fn json_roundtrip_erhaelt_aktive_materialwahl() {
        let dir = std::env::temp_dir().join("luxifer_material_profile_test");
        let _ = std::fs::remove_dir_all(&dir);
        let mut library = MaterialLibrary::default();
        library.profiles.push(profile());
        library.set_active("laser-a", Some("pappel-3"));
        library.save_to(&dir).unwrap();
        assert_eq!(MaterialLibrary::load_from(&dir), library);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
