//! Laser-Backend fürs native Panel: Registry laden/speichern, aktiven Treiber
//! bauen (wie `driver_for` in der Tauri-App), Aktionen ausführen, exportieren.
//! Nutzt dieselben Core-Typen und Treiber-Crates wie die Tauri-App — keine
//! Logik-Duplikate, nur ohne IPC.

use luxifer_core::{
    Anchor, DriverKind, JobAction, JobParams, JobPlan, LaserProfile, LaserRegistry, Layer,
    MachineDriver, Shape, StartMode,
};

/// Baut den passenden Treiber aus einem Profil (analog Tauri `driver_for`).
fn driver_for(profile: &LaserProfile) -> Box<dyn MachineDriver + Send> {
    match profile.kind {
        DriverKind::Ruida => Box::new(luxifer_driver_ruida::RuidaDriver::from_profile(profile)),
        _ => Box::new(luxifer_driver_grbl::GrblDriver::default()),
    }
}

/// Hält die Laser-Registry und den (lazy gebauten) aktiven Treiber.
pub struct LaserBackend {
    pub registry: LaserRegistry,
    driver: Option<Box<dyn MachineDriver + Send>>,
    driver_id: Option<String>,
}

impl LaserBackend {
    pub fn load() -> Self {
        Self {
            registry: LaserRegistry::load(),
            driver: None,
            driver_id: None,
        }
    }

    pub fn active_profile(&self) -> Option<&LaserProfile> {
        self.registry.active()
    }

    pub fn set_active(&mut self, id: &str) {
        if self.registry.set_active(id) {
            let _ = self.registry.save();
            self.driver = None; // beim nächsten Zugriff neu bauen
        }
    }

    pub fn save_profile(&mut self, mut profile: LaserProfile) {
        if profile.id.is_empty() {
            let millis = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            profile.id = format!("laser-{millis}");
            self.registry.add(profile);
        } else if !self.registry.update(profile.clone()) {
            self.registry.add(profile);
        }
        let _ = self.registry.save();
        self.driver = None;
    }

    pub fn delete_profile(&mut self, id: &str) {
        self.registry.remove(id);
        let _ = self.registry.save();
        self.driver = None;
    }

    /// Aktions-Schlüssel des aktiven Treibers (fürs Panel-Grid).
    pub fn actions(&mut self) -> Vec<JobAction> {
        self.with_driver(|d| Ok(d.actions())).unwrap_or_default()
    }

    /// Stellt sicher, dass der Treiber zum aktiven Profil gebaut ist, und ruft f.
    fn with_driver<T>(
        &mut self,
        f: impl FnOnce(&mut Box<dyn MachineDriver + Send>) -> Result<T, String>,
    ) -> Result<T, String> {
        let profile = self
            .registry
            .active()
            .ok_or_else(|| "Kein Laser aktiv — in den Einstellungen anlegen.".to_string())?
            .clone();
        if self.driver_id.as_deref() != Some(profile.id.as_str()) || self.driver.is_none() {
            self.driver = Some(driver_for(&profile));
            self.driver_id = Some(profile.id.clone());
        }
        f(self.driver.as_mut().unwrap())
    }

    /// Baut den JobPlan aus (ggf. nur selektierten) Shapes. Bild-Assets werden
    /// hier noch nicht aufgelöst (kommt mit dem Bild-Import); reine Vektor-Jobs.
    fn plan(shapes: &[Shape], layers: &[Layer]) -> JobPlan {
        JobPlan::from_shapes(shapes, layers)
    }

    fn job_params(start_mode: StartMode, anchor_idx: usize) -> JobParams {
        JobParams {
            start_mode,
            anchor: Anchor::from_index(anchor_idx),
        }
    }

    /// Führt eine Job-Aktion aus. Gibt die Rückmeldung des Treibers (oder Fehler).
    #[allow(clippy::too_many_arguments)]
    pub fn run_action(
        &mut self,
        action: JobAction,
        shapes: &[Shape],
        layers: &[Layer],
        start_mode: StartMode,
        anchor_idx: usize,
    ) -> Result<String, String> {
        let plan = Self::plan(shapes, layers);
        let jp = Self::job_params(start_mode, anchor_idx);
        self.with_driver(|d| {
            d.run_action(action, &plan, layers, &jp)
                .map_err(|e| e.to_string())
        })
    }

    /// Kompiliert den Job und schreibt ihn in eine Datei (Ruida .rd / GRBL .gcode).
    pub fn export_to(
        &mut self,
        path: &std::path::Path,
        shapes: &[Shape],
        layers: &[Layer],
        start_mode: StartMode,
        anchor_idx: usize,
    ) -> Result<(), String> {
        let plan = Self::plan(shapes, layers);
        let jp = Self::job_params(start_mode, anchor_idx);
        let bytes = self.with_driver(|d| d.compile_with(&plan, layers, &jp))?;
        std::fs::write(path, bytes).map_err(|e| e.to_string())
    }

    /// Jog: Kopf relativ bewegen (verbindet der Treiber selbst, falls nötig).
    pub fn jog(&mut self, dx: f64, dy: f64, speed: f64) -> Result<(), String> {
        self.with_driver(|d| d.jog(dx, dy, speed).map_err(|e| e.to_string()))
    }

    pub fn home(&mut self, speed: f64) -> Result<(), String> {
        self.with_driver(|d| d.home(speed).map_err(|e| e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luxifer_core::geometry::Geo;
    use luxifer_core::{Connection, DriverKind, LaserProfile};

    /// Backend mit einer festen Registry (ohne Platten-I/O) für Tests.
    fn backend_with_ruida() -> LaserBackend {
        let profile = LaserProfile {
            id: "test-ruida".into(),
            name: "Test-Ruida".into(),
            kind: DriverKind::Ruida,
            connection: Connection::Netz {
                ip: "192.168.1.100".into(),
                port: None,
            },
            bed_mm: (600.0, 400.0),
            ..Default::default()
        };
        let mut registry = LaserRegistry::default();
        registry.add(profile);
        registry.set_active("test-ruida");
        LaserBackend {
            registry,
            driver: None,
            driver_id: None,
        }
    }

    fn one_rect() -> (Vec<Shape>, Vec<Layer>) {
        let mut s = luxifer_core::AppState::new();
        s.add_shape(Geo::Rect {
            x: 10.0,
            y: 10.0,
            w: 50.0,
            h: 30.0,
        });
        (s.shapes.clone(), s.layers.clone())
    }

    #[test]
    fn aktiver_ruida_meldet_aktionen() {
        let mut b = backend_with_ruida();
        let actions = b.actions();
        assert!(!actions.is_empty(), "Ruida sollte Aktionen melden");
        // Ruida bietet mindestens Senden und Export an.
        assert!(actions.iter().any(|a| matches!(a, JobAction::SendJob)));
        assert!(actions.iter().any(|a| matches!(a, JobAction::ExportFile)));
    }

    #[test]
    fn export_erzeugt_ruida_bytes() {
        let mut b = backend_with_ruida();
        let (shapes, layers) = one_rect();
        let tmp = std::env::temp_dir().join("luxifer_test_job.rd");
        let r = b.export_to(&tmp, &shapes, &layers, luxifer_core::StartMode::Absolut, 4);
        assert!(r.is_ok(), "Export sollte klappen: {r:?}");
        let bytes = std::fs::read(&tmp).unwrap();
        assert!(!bytes.is_empty(), "Ruida-Job darf nicht leer sein");
        let _ = std::fs::remove_file(&tmp);
    }
}
