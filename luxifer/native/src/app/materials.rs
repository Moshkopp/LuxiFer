//! Design-first-Layerkonfiguration (ADR 0019, experimentell/uncommitted).

use crate::ui::{LayerManagerState, MaterialManagerState};

use super::App;

impl App {
    pub fn open_layer_manager(&mut self) {
        if self.session.layers.is_empty() {
            self.toasts.error("Das Design enthält noch keine Layer.");
            return;
        }
        let material_id = self
            .laser_backend
            .active_profile()
            .and_then(|laser| self.material_library.active_for(&laser.id))
            .map(|material| material.id.clone());
        self.layer_manager = Some(LayerManagerState {
            layers: self
                .session
                .layers
                .iter()
                .map(luxifer_application::LayerParams::from_layer)
                .collect(),
            material_id,
        });
    }

    pub fn layer_manager_save(&mut self) {
        let Some(draft) = self.layer_manager.as_ref() else {
            return;
        };
        match self.session.set_all_layer_params(&draft.layers) {
            Ok(()) => {
                if let (Some(laser), Some(material_id)) = (
                    self.laser_backend.active_profile(),
                    draft.material_id.as_deref(),
                ) {
                    self.material_library
                        .set_active(&laser.id, Some(material_id));
                    if let Err(error) = self.material_library.save() {
                        self.toasts
                            .error(format!("Materialauswahl speichern: {error}"));
                    }
                }
                self.layer_manager = None;
                self.renderer.invalidate_scene();
                self.toasts.success("Layerkonfiguration übernommen.");
            }
            Err(error) => self.app_error = Some(error),
        }
    }

    pub fn open_material_manager(&mut self, create_new: bool) {
        let Some(laser_id) = self
            .laser_backend
            .active_profile()
            .map(|profile| profile.id.clone())
        else {
            self.toasts.error("Zuerst einen Laser auswählen.");
            return;
        };

        let source = self
            .layer_manager
            .as_ref()
            .and_then(|manager| manager.layers.first())
            .map(layer_from_params)
            .unwrap_or_else(|| luxifer_core::Layer::new(0));
        let selected_id = self
            .layer_manager
            .as_ref()
            .and_then(|manager| manager.material_id.as_deref());
        let selected = selected_id.and_then(|id| {
            self.material_library
                .profiles
                .iter()
                .find(|profile| profile.id == id && profile.laser_id == laser_id)
                .cloned()
        });
        let draft = if create_new {
            new_material_profile(&laser_id, &source)
        } else {
            selected.unwrap_or_else(|| new_material_profile(&laser_id, &source))
        };
        self.material_manager = Some(MaterialManagerState {
            is_new: create_new || draft.id.is_empty(),
            draft,
        });
    }

    pub fn material_manager_save(&mut self) {
        let Some(mut profile) = self
            .material_manager
            .as_ref()
            .map(|state| state.draft.clone())
        else {
            return;
        };
        profile.name = profile.name.trim().to_string();
        if profile.name.is_empty() {
            self.toasts.error("Das Material braucht einen Namen.");
            return;
        }
        if profile
            .thickness_mm
            .is_some_and(|value| !value.is_finite() || value <= 0.0)
        {
            self.toasts
                .error("Die Materialstärke muss größer als 0 mm sein.");
            return;
        }
        if profile.id.is_empty() {
            profile.id = format!("material-{}", timestamp_millis());
            self.material_library.profiles.push(profile.clone());
        } else if let Some(existing) = self
            .material_library
            .profiles
            .iter_mut()
            .find(|existing| existing.id == profile.id)
        {
            *existing = profile.clone();
        } else {
            self.material_library.profiles.push(profile.clone());
        }
        self.material_library
            .set_active(&profile.laser_id, Some(&profile.id));
        match self.material_library.save() {
            Ok(_) => {
                if let Some(manager) = self.layer_manager.as_mut() {
                    manager.material_id = Some(profile.id);
                }
                self.material_manager = None;
                self.toasts.success("Materialprofil gespeichert.");
            }
            Err(error) => self
                .toasts
                .error(format!("Materialprofil speichern: {error}")),
        }
    }

    pub fn material_manager_delete(&mut self) {
        let Some(profile) = self
            .material_manager
            .as_ref()
            .map(|state| state.draft.clone())
        else {
            return;
        };
        self.material_library
            .profiles
            .retain(|existing| existing.id != profile.id);
        self.material_library
            .active_by_laser
            .retain(|_, material_id| material_id != &profile.id);
        if let Some(manager) = self.layer_manager.as_mut() {
            if manager.material_id.as_deref() == Some(profile.id.as_str()) {
                manager.material_id = None;
            }
        }
        match self.material_library.save() {
            Ok(_) => {
                self.material_manager = None;
                self.toasts.success("Materialprofil gelöscht.");
            }
            Err(error) => self
                .toasts
                .error(format!("Materialprofil löschen: {error}")),
        }
    }
}

fn new_material_profile(
    laser_id: &str,
    layer: &luxifer_core::Layer,
) -> luxifer_core::MaterialProfile {
    let process = luxifer_core::MaterialProcess::from_layer_mode(layer.mode);
    let mut profile = luxifer_core::MaterialProfile {
        id: String::new(),
        laser_id: laser_id.into(),
        name: "Neues Material".into(),
        thickness_mm: None,
        cut: None,
        vector_engrave: None,
        raster_engrave: None,
    };
    *profile.defaults_mut(process) = Some(luxifer_core::MaterialProcessDefaults::from_layer(layer));
    profile
}

fn layer_from_params(params: &luxifer_application::LayerParams) -> luxifer_core::Layer {
    let mut layer = luxifer_core::Layer::new(0);
    layer.name = params.name.clone();
    layer.mode = params.mode;
    layer.speed_mm_s = params.speed_mm_s;
    layer.power_pct = params.power_pct;
    layer.min_power_pct = params.min_power_pct;
    layer.passes = params.passes;
    layer.air_assist = params.air_assist;
    layer.line_step_mm = params.line_step_mm;
    layer.dpi = params.dpi;
    layer.bidirectional = params.bidirectional;
    layer
}

fn timestamp_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}
