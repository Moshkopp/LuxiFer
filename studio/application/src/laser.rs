//! Laser-Gerätedienst (ADR 0011, Phase 6): Registry laden/speichern, den aktiven
//! Treiber lazy bauen, Job-Aktionen ausführen, exportieren, jog/home. Koordiniert
//! die Treiber-Lebenszyklen; die UI erzeugt nie selbst einen Treiber.
//!
//! Fehler werden als stabiler [`AppError`] gemeldet. Erfolgsrückmeldungen des
//! Treibers (z. B. „Job gesendet") bleiben nutzerlesbare Strings.

use studio_core::{
    Anchor, Connection, DriverCapabilities, DriverKind, JobAction, JobParams, JobPlan,
    LaserProfile, LaserRegistry, Layer, MachineDriver, MachineSetting, MachineStatus, SavedOrigin,
    Shape, StartMode, StartReference, LASER_PROFILE_SCHEMA_VERSION,
};

use crate::{catalog_sync::enqueue_catalog_profile, AppError, CatalogKind, SharedCatalogRecord};

/// Ob eine Job-Aktion eine offene Geräteverbindung braucht. Kompilieren/
/// Export laufen ohne Gerät.
fn needs_connection(a: JobAction) -> bool {
    !matches!(a, JobAction::ExportFile)
}

/// Verbindungsziel aus dem Profil: IP (Netz) bzw. Gerätepfad (Seriell).
fn connection_target(profile: &LaserProfile) -> String {
    match &profile.connection {
        Connection::Netz { ip, .. } => ip.clone(),
        Connection::Seriell { port, .. } => port.clone(),
    }
}

/// Baut den passenden Treiber aus einem Profil.
fn driver_for(profile: &LaserProfile) -> Box<dyn MachineDriver + Send> {
    match profile.kind {
        DriverKind::Ruida => Box::new(driver_ruida::RuidaDriver::from_profile(profile)),
        _ => Box::new(driver_grbl::GrblDriver::default()),
    }
}

/// Hält die Laser-Registry und den (lazy gebauten) aktiven Treiber.
pub struct LaserService {
    pub registry: LaserRegistry,
    driver: Option<Box<dyn MachineDriver + Send>>,
    driver_id: Option<String>,
    connected_id: Option<String>,
}

impl LaserService {
    /// Baut die maschinenspezifische Bewegungsspur mit denselben Profil- und
    /// Startparametern wie Export/Start.
    pub fn execution_trace(
        &self,
        shapes: &[Shape],
        layers: &[Layer],
        reference: &StartReference,
        anchor_idx: usize,
    ) -> Result<studio_core::ExecutionTrace, AppError> {
        let profile = self
            .active_profile()
            .ok_or_else(|| AppError::new("no_active_laser", "Kein Laser aktiv."))?;
        let (plan, resolved) = self.resolved_plan(shapes, layers, reference, anchor_idx)?;
        let params = JobParams {
            // Die Preview bleibt bei relativen Startmodi an den Projekt-/Bett-
            // koordinaten des Motivs — der Controller wendet den Bezugspunkt
            // erst beim Starten an. Ein gespeicherter Nullpunkt ist dagegen
            // schon app-seitig absolut aufgelöst (Plan bereits verschoben).
            start_mode: StartMode::Absolut,
            anchor: resolved.anchor,
        };
        driver_for(profile)
            .execution_trace(&plan, layers, &params)
            .map_err(|error| {
                AppError::wrap(
                    "execution_trace",
                    "Laserpfad konnte nicht aufgebaut werden.",
                    error,
                )
            })
    }
    /// Liest Maschinenparameter, wenn der aktive Treiber diese Capability hat.
    pub fn read_machine_settings(&mut self) -> Result<Vec<MachineSetting>, AppError> {
        self.with_driver(true, |driver| {
            if !driver.capabilities().machine_settings {
                return Err(AppError::new(
                    "machine_settings_unsupported",
                    "Der aktive Lasertreiber unterstützt keine Maschinendaten.",
                ));
            }
            driver.read_machine_settings().map_err(|error| {
                AppError::wrap(
                    "machine_settings_read",
                    "Maschinendaten lesen fehlgeschlagen.",
                    error.to_string(),
                )
            })
        })
    }

    /// Schreibt geprüfte Rohwerte über den aktiven Treiber und liest sie
    /// anschließend der Bestätigung halber erneut.
    pub fn write_machine_settings(
        &mut self,
        changes: &[(u16, i64)],
    ) -> Result<Vec<MachineSetting>, AppError> {
        self.with_driver(true, |driver| {
            if !driver.capabilities().machine_settings {
                return Err(AppError::new(
                    "machine_settings_unsupported",
                    "Der aktive Lasertreiber unterstützt keine Maschinendaten.",
                ));
            }
            driver.write_machine_settings(changes).map_err(|error| {
                AppError::wrap(
                    "machine_settings_write",
                    "Maschinendaten schreiben fehlgeschlagen.",
                    error.to_string(),
                )
            })?;
            driver.read_machine_settings().map_err(|error| {
                AppError::wrap(
                    "machine_settings_verify",
                    "Maschinendaten wurden geschrieben, konnten aber nicht zur Kontrolle gelesen werden.",
                    error.to_string(),
                )
            })
        })
    }

    pub fn load() -> Self {
        Self {
            registry: LaserRegistry::load(),
            driver: None,
            driver_id: None,
            connected_id: None,
        }
    }

    /// Dienst mit vorgegebener Registry (ohne Platten-I/O) — für Tests.
    #[cfg(test)]
    fn with_registry(registry: LaserRegistry) -> Self {
        Self {
            registry,
            driver: None,
            driver_id: None,
            connected_id: None,
        }
    }

    pub fn active_profile(&self) -> Option<&LaserProfile> {
        self.registry.active()
    }

    pub fn set_active(&mut self, id: &str) {
        if self.registry.active_id.as_deref() == Some(id) {
            return;
        }
        if self.registry.set_active(id) {
            let _ = self.registry.save();
            self.disconnect();
            self.driver = None; // beim nächsten Zugriff neu bauen
            self.driver_id = None;
        }
    }

    /// Legt ein neues Profil an oder aktualisiert ein bestehendes (nach ID).
    pub fn save_profile(&mut self, mut profile: LaserProfile) -> Result<(), AppError> {
        // Studio schreibt immer die höchste vollständig verstandene Version;
        // die Nullpunktliste wird nie still umgedeutet gespeichert.
        profile.schema_version = LASER_PROFILE_SCHEMA_VERSION;
        profile
            .validate_saved_origins()
            .map_err(|message| AppError::new("origin_invalid", message))?;
        let previous = self
            .registry
            .profiles
            .iter()
            .find(|existing| existing.id == profile.id)
            .cloned();
        if profile.id.is_empty() {
            let millis = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            profile.id = format!("laser-{millis}");
            self.registry.add(profile.clone());
        } else if !self.registry.update(profile.clone()) {
            self.registry.add(profile.clone());
        }
        self.registry.save().map_err(|error| {
            AppError::new(
                "laser_registry_write",
                format!("Laserprofile speichern fehlgeschlagen: {error}"),
            )
        })?;
        enqueue_catalog_profile(CatalogKind::LaserProfile, &profile.id, Some(&profile))?;
        // Eine bestehende Verbindung überlebt das Speichern: Nur wenn sich am
        // Profil des gerade gebauten Treibers treiberrelevante Felder geändert
        // haben, wird sauber getrennt und der Treiber verworfen.
        if self.invalidates_driver(&profile, previous.as_ref()) {
            self.disconnect();
            self.driver = None;
            self.driver_id = None;
        }
        Ok(())
    }

    /// Ob ein gespeicherter Profilstand den lazy gebauten Treiber (und damit
    /// eine offene Verbindung) ungültig macht: nur wenn der Treiber für genau
    /// dieses Profil gebaut wurde UND sich Felder geändert haben, die im
    /// Treiber bzw. in der Verbindung stecken (Typ, Verbindungsziel,
    /// Scan-Offset). Name, Bett oder Nullpunkte leben in der Registry und
    /// erfordern keine Trennung.
    fn invalidates_driver(&self, profile: &LaserProfile, previous: Option<&LaserProfile>) -> bool {
        if self.driver_id.as_deref() != Some(profile.id.as_str()) {
            return false;
        }
        previous.is_none_or(|old| {
            old.kind != profile.kind
                || old.connection != profile.connection
                || old.scan_offset != profile.scan_offset
        })
    }

    pub fn delete_profile(&mut self, id: &str) -> Result<(), AppError> {
        self.registry.remove(id);
        self.registry.save().map_err(|error| {
            AppError::new(
                "laser_registry_write",
                format!("Laserprofile speichern fehlgeschlagen: {error}"),
            )
        })?;
        enqueue_catalog_profile::<LaserProfile>(CatalogKind::LaserProfile, id, None)?;
        // Nur das gelöschte Gerät verliert Treiber und Verbindung; andere
        // Profile zu löschen lässt eine bestehende Verbindung in Ruhe.
        if self.driver_id.as_deref() == Some(id) {
            self.disconnect();
            self.driver = None;
            self.driver_id = None;
        }
        Ok(())
    }

    pub fn apply_shared_record(&mut self, record: &SharedCatalogRecord) -> Result<bool, AppError> {
        if record.kind != CatalogKind::LaserProfile {
            return Ok(false);
        }
        // Lokale, noch nicht übertragene Änderungen gewinnen: Der Sync-Worker
        // liefert zyklisch den vollen Katalogstand — ein Datensatz aus einem
        // Zyklus VOR einer gerade gespeicherten lokalen Änderung (z. B. neuer
        // Werkstück-Nullpunkt) darf sie nicht rückgängig machen. Der nächste
        // Zyklus lädt die Outbox hoch bzw. meldet einen echten Konflikt.
        if crate::catalog_sync::has_pending_change(record.kind, &record.id)? {
            return Ok(false);
        }
        let (changed, driver_invalidated) = if record.deleted {
            let existed = self
                .registry
                .profiles
                .iter()
                .any(|profile| profile.id == record.id);
            self.registry.remove(&record.id);
            (
                existed,
                existed && self.driver_id.as_deref() == Some(record.id.as_str()),
            )
        } else {
            let payload = record
                .payload
                .as_deref()
                .ok_or_else(|| AppError::new("catalog_payload", "Laserprofil fehlt."))?;
            // Schemaversion vor der typisierten Deserialisierung prüfen: Ein
            // neueres Profil darf gelesen/angezeigt, aber nie verlustbehaftet
            // in die lokale Registry übernommen werden (ADR 0020).
            let value: serde_json::Value = serde_json::from_str(payload)
                .map_err(|error| AppError::new("catalog_payload", error.to_string()))?;
            let version = value
                .get("schema_version")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(1);
            if version > u64::from(LASER_PROFILE_SCHEMA_VERSION) {
                return Err(AppError::new(
                    "catalog_schema",
                    format!(
                        "Empfangenes Laserprofil nutzt Schemaversion {version}; dieses Studio versteht nur bis {LASER_PROFILE_SCHEMA_VERSION}. Bitte Studio aktualisieren."
                    ),
                ));
            }
            let profile: LaserProfile = serde_json::from_value(value)
                .map_err(|error| AppError::new("catalog_payload", error.to_string()))?;
            profile
                .validate_saved_origins()
                .map_err(|message| AppError::new("catalog_payload", message))?;
            let previous = self
                .registry
                .profiles
                .iter()
                .find(|item| item.id == profile.id)
                .cloned();
            let changed = previous.as_ref() != Some(&profile);
            let invalidated = changed && self.invalidates_driver(&profile, previous.as_ref());
            if !self.registry.update(profile.clone()) {
                self.registry.profiles.push(profile);
            }
            (changed, invalidated)
        };
        if changed {
            if self.registry.active_id.as_ref().is_some_and(|id| {
                !self
                    .registry
                    .profiles
                    .iter()
                    .any(|profile| &profile.id == id)
            }) {
                self.registry.active_id = self
                    .registry
                    .profiles
                    .first()
                    .map(|profile| profile.id.clone());
            }
            self.registry
                .save()
                .map_err(|error| AppError::new("laser_registry_write", error))?;
            // Wie beim lokalen Speichern: Ein empfangener Katalogstand trennt
            // die Verbindung nur, wenn er das Profil des gebauten Treibers
            // treiberrelevant ändert oder löscht.
            if driver_invalidated {
                self.disconnect();
                self.driver = None;
                self.driver_id = None;
            }
        }
        Ok(changed)
    }

    /// Ersetzt die lokale Registry nach einer ausdrücklich gewählten
    /// Sicherungs-Wiederherstellung und verwirft den dazu nicht mehr passenden
    /// lazy Treiber.
    pub fn restore_registry(&mut self, mut registry: LaserRegistry) -> Result<(), AppError> {
        // Auch wiederhergestellte (ggf. alte) Sicherungen werden mit der
        // höchsten verstandenen Schemaversion geschrieben — sonst bliebe ihr
        // Katalog-Upload am Hub-Downgrade-Schutz hängen.
        for profile in &mut registry.profiles {
            profile.schema_version = LASER_PROFILE_SCHEMA_VERSION;
        }
        let previous_ids = self
            .registry
            .profiles
            .iter()
            .map(|profile| profile.id.clone())
            .collect::<Vec<_>>();
        registry.active_id = self
            .registry
            .active_id
            .clone()
            .filter(|id| registry.profiles.iter().any(|profile| &profile.id == id))
            .or_else(|| registry.profiles.first().map(|profile| profile.id.clone()));
        registry.save().map_err(|error| {
            AppError::new(
                "laser_registry_write",
                format!("Laserprofile speichern fehlgeschlagen: {error}"),
            )
        })?;
        for profile in &registry.profiles {
            enqueue_catalog_profile(CatalogKind::LaserProfile, &profile.id, Some(profile))?;
        }
        for id in previous_ids {
            if !registry.profiles.iter().any(|profile| profile.id == id) {
                enqueue_catalog_profile::<LaserProfile>(CatalogKind::LaserProfile, &id, None)?;
            }
        }
        self.registry = registry;
        self.disconnect();
        self.driver = None;
        self.driver_id = None;
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.connected_id.is_some()
            && self.connected_id.as_deref() == self.registry.active_id.as_deref()
    }

    pub fn active_uses_network(&self) -> bool {
        self.active_profile()
            .is_some_and(|profile| matches!(profile.connection, Connection::Netz { .. }))
    }

    pub fn active_lease_identity(&self) -> Option<(String, String)> {
        let profile = self.active_profile()?;
        let Connection::Netz { .. } = profile.connection else {
            return None;
        };
        let target = connection_target(profile);
        Some((
            format!(
                "controller-{}",
                studio_core::assets::content_hash(target.as_bytes())
            ),
            profile.name.clone(),
        ))
    }

    pub fn connect(&mut self) -> Result<(), AppError> {
        let profile = self
            .active_profile()
            .cloned()
            .ok_or_else(|| AppError::new("no_active_laser", "Kein Laser aktiv."))?;
        self.with_driver(false, |driver| {
            let target = connection_target(&profile);
            driver.connect(&target).map_err(|error| {
                AppError::wrap(
                    "laser_connect",
                    format!("Keine Verbindung zum Laser ({target})."),
                    error.to_string(),
                )
            })
        })?;
        self.connected_id = Some(profile.id);
        Ok(())
    }

    pub fn disconnect(&mut self) {
        if let Some(driver) = self.driver.as_mut() {
            driver.disconnect();
        }
        self.connected_id = None;
    }

    /// Verfügbare Job-Aktionen des aktiven Treibers (fürs Panel-Grid). Ohne
    /// aktiven Treiber leer.
    pub fn actions(&mut self) -> Vec<JobAction> {
        self.with_driver(false, |d| Ok(d.actions()))
            .unwrap_or_default()
    }

    /// Stellt sicher, dass der Treiber zum aktiven Profil gebaut ist, und ruft f.
    /// `requires_connection` weist maschinenwirksame Aufrufe ab, solange der
    /// Nutzer nicht ausdrücklich verbunden hat.
    fn with_driver<T>(
        &mut self,
        requires_connection: bool,
        f: impl FnOnce(&mut Box<dyn MachineDriver + Send>) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        let profile = self
            .registry
            .active()
            .ok_or_else(|| {
                AppError::new(
                    "no_active_laser",
                    "Kein Laser aktiv — in den Einstellungen anlegen.",
                )
            })?
            .clone();
        if self.driver_id.as_deref() != Some(profile.id.as_str()) || self.driver.is_none() {
            self.driver = Some(driver_for(&profile));
            self.driver_id = Some(profile.id.clone());
        }
        let driver = self.driver.as_mut().unwrap();
        if requires_connection && self.connected_id.as_deref() != Some(profile.id.as_str()) {
            return Err(AppError::new(
                "laser_not_connected",
                "Laser ist nicht verbunden. Bitte zuerst ausdrücklich verbinden.",
            ));
        }
        f(driver)
    }

    /// Baut den JobPlan MIT Asset-Auflösung — Bild-Layer werden gerastert.
    /// Dieselbe Quelle wie die Vorschau (`EditorSession::job_preview`), damit
    /// die Vorschau nie etwas zeigt, das der echte Job nicht tut (und der Job
    /// nichts auslässt, was die Vorschau zeigt).
    fn plan(&self, shapes: &[Shape], layers: &[Layer]) -> JobPlan {
        let plan = JobPlan::from_shapes_with_assets(shapes, layers, crate::assets::resolve_luma);
        self.active_profile().map_or(plan.clone(), |profile| {
            plan.transformed_for_bed(profile.origin, profile.bed_mm)
        })
    }

    /// Löst die Startreferenz **genau einmal** in der gemeinsamen
    /// Ausführungsspur auf (ADR 0020 §G): Ein gespeicherter Nullpunkt
    /// verschiebt den Plan app-seitig so, dass der gewählte Anker auf der
    /// absoluten Zielkoordinate liegt; der Treiber erhält einen absoluten Job.
    /// Fehlende oder ungültige Referenzen sind ein Fehler — kein stiller
    /// Fallback.
    fn resolved_plan(
        &self,
        shapes: &[Shape],
        layers: &[Layer],
        reference: &StartReference,
        anchor_idx: usize,
    ) -> Result<(JobPlan, JobParams), AppError> {
        let plan = self.plan(shapes, layers);
        let anchor = Anchor::from_index(anchor_idx);
        let Some(profile) = self.active_profile() else {
            return Ok((
                plan,
                JobParams {
                    start_mode: reference.start_mode(),
                    anchor,
                },
            ));
        };
        let anchor = profile.origin.transform_anchor(anchor);
        let plan = match reference {
            StartReference::GespeicherterNullpunkt { id } => {
                let origin = profile.saved_origin(id).ok_or_else(|| {
                    AppError::new(
                        "origin_missing",
                        "Der gewählte gespeicherte Nullpunkt existiert im aktiven Laserprofil nicht. Bitte eine neue Startreferenz wählen.",
                    )
                })?;
                if !profile.saved_origin_usable(origin) {
                    return Err(AppError::new(
                        "origin_invalid",
                        format!(
                            "Nullpunkt „{}“ liegt außerhalb des Arbeitsbereichs. Bitte neu speichern oder entfernen.",
                            origin.name
                        ),
                    ));
                }
                plan.placed_with_anchor_at(anchor, (origin.x_mm, origin.y_mm))
            }
            _ => plan,
        };
        Ok((
            plan,
            JobParams {
                start_mode: reference.start_mode(),
                anchor,
            },
        ))
    }

    /// Führt eine Job-Aktion aus und gibt die Rückmeldung des Treibers zurück.
    pub fn run_action(
        &mut self,
        action: JobAction,
        shapes: &[Shape],
        layers: &[Layer],
        reference: &StartReference,
        anchor_idx: usize,
    ) -> Result<String, AppError> {
        let (plan, jp) = self.resolved_plan(shapes, layers, reference, anchor_idx)?;
        self.with_driver(needs_connection(action), |d| {
            d.run_action(action, &plan, layers, &jp).map_err(|e| {
                AppError::wrap(
                    "laser_action",
                    "Laser-Aktion fehlgeschlagen.",
                    e.to_string(),
                )
            })
        })
    }

    /// Kompiliert den Job und schreibt ihn in eine Datei (Ruida .rd / GRBL .gcode).
    pub fn export_to(
        &mut self,
        path: &std::path::Path,
        shapes: &[Shape],
        layers: &[Layer],
        reference: &StartReference,
        anchor_idx: usize,
    ) -> Result<(), AppError> {
        let (plan, jp) = self.resolved_plan(shapes, layers, reference, anchor_idx)?;
        // Export kompiliert nur — dafür braucht es kein erreichbares Gerät.
        let bytes = self.with_driver(false, |d| {
            d.compile_with(&plan, layers, &jp)
                .map_err(|e| AppError::wrap("laser_export", "Job-Kompilierung fehlgeschlagen.", e))
        })?;
        std::fs::write(path, bytes).map_err(|e| {
            AppError::wrap(
                "laser_export",
                "Datei schreiben fehlgeschlagen.",
                e.to_string(),
            )
        })
    }

    /// Jog: Kopf relativ bewegen; eine explizite Verbindung ist Voraussetzung.
    pub fn jog(&mut self, dx: f64, dy: f64, speed: f64) -> Result<(), AppError> {
        self.with_driver(true, |d| {
            d.jog(dx, dy, speed)
                .map_err(|e| AppError::wrap("laser_jog", "Jog fehlgeschlagen.", e.to_string()))
        })
    }

    pub fn home(&mut self, speed: f64) -> Result<(), AppError> {
        self.with_driver(true, |d| {
            d.home(speed)
                .map_err(|e| AppError::wrap("laser_home", "Home fehlgeschlagen.", e.to_string()))
        })
    }

    // --- Positionslesen und Werkstück-Nullpunkte (ADR 0020) -----------------

    /// Fähigkeiten des aktiven Treibers (für „nicht unterstützt"-Anzeigen).
    pub fn driver_capabilities(&mut self) -> DriverCapabilities {
        self.with_driver(false, |driver| Ok(driver.capabilities()))
            .unwrap_or_default()
    }

    /// Liest den aktuellen Maschinenstatus (Kopfposition) frisch vom Treiber.
    /// Eine ausdrückliche Verbindung ist Voraussetzung; Treiber ohne
    /// Lesefähigkeit melden „nicht unterstützt" statt erfundener Koordinaten.
    pub fn read_status(&mut self) -> Result<MachineStatus, AppError> {
        self.with_driver(true, |driver| {
            if !driver.capabilities().position_read {
                return Err(AppError::new(
                    "position_unsupported",
                    "Der aktive Lasertreiber unterstützt kein Positionslesen.",
                ));
            }
            driver.status().map_err(|error| {
                AppError::wrap(
                    "laser_status",
                    "Maschinenstatus lesen fehlgeschlagen.",
                    error.to_string(),
                )
            })
        })
    }

    /// Liest den am Ruida-Hardwarepanel gesetzten Benutzerursprung. Wird nur
    /// bei angewählter Referenz `Benutzerursprung` gebraucht — Studio setzt
    /// oder verschiebt diesen Ursprung nie.
    pub fn read_user_origin(&mut self) -> Result<(f64, f64), AppError> {
        self.with_driver(true, |driver| {
            if !driver.capabilities().user_origin_read {
                return Err(AppError::new(
                    "user_origin_unsupported",
                    "Der aktive Lasertreiber unterstützt kein Lesen des Benutzerursprungs.",
                ));
            }
            driver.read_origin().map_err(|error| {
                AppError::wrap(
                    "laser_origin_read",
                    "Benutzerursprung lesen fehlgeschlagen.",
                    error.to_string(),
                )
            })
        })
    }

    /// Frisch gelesene, endliche und innerhalb des Profils liegende
    /// Kopfposition — Voraussetzung für Speichern und absolutes Anfahren
    /// (ADR 0020 §F). Scheitert das Statuslesen, bleibt die Aktion gesperrt.
    pub fn read_plausible_position(&mut self) -> Result<MachineStatus, AppError> {
        let bed = self
            .active_profile()
            .map(|profile| profile.bed_mm)
            .ok_or_else(|| AppError::new("no_active_laser", "Kein Laser aktiv."))?;
        let status = self.read_status()?;
        let (x, y) = (status.pos_x_mm, status.pos_y_mm);
        if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 || x > bed.0 || y > bed.1 {
            return Err(AppError::new(
                "position_implausible",
                format!(
                    "Gelesene Kopfposition ({x:.2}/{y:.2} mm) liegt nicht plausibel im Arbeitsbereich."
                ),
            ));
        }
        Ok(status)
    }

    /// Mutiert die Nullpunktliste des aktiven Profils, validiert, speichert
    /// atomar und stellt die Änderung in die Katalog-Outbox — ohne den
    /// verbundenen Treiber zu verwerfen (die Verbindungsparameter ändern sich
    /// hier nicht).
    fn mutate_active_origins(
        &mut self,
        mutate: impl FnOnce(&mut Vec<SavedOrigin>),
    ) -> Result<(), AppError> {
        let mut profile = self
            .active_profile()
            .cloned()
            .ok_or_else(|| AppError::new("no_active_laser", "Kein Laser aktiv."))?;
        mutate(&mut profile.saved_origins);
        // Studio schreibt immer die höchste vollständig verstandene Version.
        profile.schema_version = LASER_PROFILE_SCHEMA_VERSION;
        profile
            .validate_saved_origins()
            .map_err(|message| AppError::new("origin_invalid", message))?;
        self.registry.update(profile.clone());
        self.registry.save().map_err(|error| {
            AppError::new(
                "laser_registry_write",
                format!("Laserprofile speichern fehlgeschlagen: {error}"),
            )
        })?;
        enqueue_catalog_profile(CatalogKind::LaserProfile, &profile.id, Some(&profile))
    }

    /// Speichert eine bereits **frisch gelesene** Kopfposition unter dem Namen
    /// als neuen Nullpunkt im aktiven Laserprofil (ADR 0020 §D: die beim
    /// Auslösen gelesene Position wird gespeichert; ein gecachter Anzeigewert
    /// ist nie die Quelle).
    pub fn add_saved_origin(
        &mut self,
        name: &str,
        x_mm: f64,
        y_mm: f64,
    ) -> Result<SavedOrigin, AppError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::new(
                "origin_name",
                "Der Name des Nullpunkts darf nicht leer sein.",
            ));
        }
        let profile = self
            .active_profile()
            .ok_or_else(|| AppError::new("no_active_laser", "Kein Laser aktiv."))?;
        let bed = profile.bed_mm;
        if !x_mm.is_finite()
            || !y_mm.is_finite()
            || x_mm < 0.0
            || y_mm < 0.0
            || x_mm > bed.0
            || y_mm > bed.1
        {
            return Err(AppError::new(
                "position_implausible",
                format!("Position ({x_mm:.2}/{y_mm:.2} mm) liegt nicht im Arbeitsbereich."),
            ));
        }
        if profile
            .saved_origins
            .iter()
            .any(|origin| origin.name.trim() == name)
        {
            return Err(AppError::new(
                "origin_name_duplicate",
                format!("Es gibt bereits einen Nullpunkt „{name}“ für diesen Laser."),
            ));
        }
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        let origin = SavedOrigin {
            id: format!("origin-{millis}-{}", profile.saved_origins.len()),
            name: name.to_owned(),
            x_mm,
            y_mm,
        };
        let stored = origin.clone();
        self.mutate_active_origins(move |origins| origins.push(origin))?;
        Ok(stored)
    }

    /// Liest die Kopfposition frisch vom Controller (nie einen gecachten
    /// Anzeigewert) und speichert sie unter dem Namen als neuen Nullpunkt im
    /// aktiven Laserprofil (ADR 0020 §D).
    pub fn save_current_position_as_origin(&mut self, name: &str) -> Result<SavedOrigin, AppError> {
        let status = self.read_plausible_position()?;
        self.add_saved_origin(name, status.pos_x_mm, status.pos_y_mm)
    }

    /// Benennt einen Nullpunkt um. Die stabile ID bleibt unverändert.
    pub fn rename_saved_origin(&mut self, id: &str, name: &str) -> Result<(), AppError> {
        let name = name.trim().to_owned();
        if name.is_empty() {
            return Err(AppError::new(
                "origin_name",
                "Der Name des Nullpunkts darf nicht leer sein.",
            ));
        }
        let id = id.to_owned();
        let mut found = false;
        self.mutate_active_origins(|origins| {
            if let Some(origin) = origins.iter_mut().find(|origin| origin.id == id) {
                origin.name = name;
                found = true;
            }
        })?;
        if !found {
            return Err(AppError::new(
                "origin_missing",
                "Der gespeicherte Nullpunkt existiert nicht mehr.",
            ));
        }
        Ok(())
    }

    /// Löscht einen Nullpunkt aus dem aktiven Laserprofil.
    pub fn delete_saved_origin(&mut self, id: &str) -> Result<(), AppError> {
        let id = id.to_owned();
        self.mutate_active_origins(|origins| origins.retain(|origin| origin.id != id))
    }

    /// Fährt den Kopf laserfrei und absolut nach (x, y) mm. Prüft endliche
    /// Werte, die Bettgrenzen des aktiven Profils und dass kein Job läuft —
    /// die UI-Sperre allein ist keine Sicherheitsgrenze (ADR 0020 §F).
    pub fn move_to_position(&mut self, x_mm: f64, y_mm: f64, speed: f64) -> Result<(), AppError> {
        let bed = self
            .active_profile()
            .map(|profile| profile.bed_mm)
            .ok_or_else(|| AppError::new("no_active_laser", "Kein Laser aktiv."))?;
        if !x_mm.is_finite()
            || !y_mm.is_finite()
            || x_mm < 0.0
            || y_mm < 0.0
            || x_mm > bed.0
            || y_mm > bed.1
        {
            return Err(AppError::new(
                "move_out_of_bed",
                format!(
                    "Zielposition ({x_mm:.2}/{y_mm:.2} mm) liegt außerhalb des Arbeitsbereichs."
                ),
            ));
        }
        let status = self.read_plausible_position()?;
        if status.is_running {
            return Err(AppError::new(
                "laser_busy",
                "Während eines laufenden Jobs wird nicht angefahren.",
            ));
        }
        self.with_driver(true, |driver| {
            if !driver.capabilities().absolute_move {
                return Err(AppError::new(
                    "move_unsupported",
                    "Der aktive Lasertreiber unterstützt kein absolutes Anfahren.",
                ));
            }
            driver.move_to(x_mm, y_mm, speed).map_err(|error| {
                AppError::wrap("laser_move", "Anfahren fehlgeschlagen.", error.to_string())
            })
        })
    }

    /// Fährt einen gespeicherten Nullpunkt an. Die ID wird ausschließlich im
    /// aktiven Laserprofil aufgelöst; ungültige Einträge bleiben gesperrt.
    pub fn move_to_saved_origin(&mut self, id: &str, speed: f64) -> Result<(), AppError> {
        let profile = self
            .active_profile()
            .ok_or_else(|| AppError::new("no_active_laser", "Kein Laser aktiv."))?;
        let origin = profile.saved_origin(id).ok_or_else(|| {
            AppError::new(
                "origin_missing",
                "Der gespeicherte Nullpunkt existiert im aktiven Laserprofil nicht.",
            )
        })?;
        if !profile.saved_origin_usable(origin) {
            return Err(AppError::new(
                "origin_invalid",
                format!(
                    "Nullpunkt „{}“ liegt außerhalb des Arbeitsbereichs. Bitte neu speichern oder entfernen.",
                    origin.name
                ),
            ));
        }
        let (x, y) = (origin.x_mm, origin.y_mm);
        self.move_to_position(x, y, speed)
    }

    /// „Ursprung" anfahren: bewegt den Kopf laserfrei zum Bezugspunkt der
    /// gewählten Startreferenz. Absolut → Maschinen-Null 0/0, Benutzerursprung
    /// → controllerseitiger Ursprung, gespeicherter Nullpunkt → dessen
    /// Koordinate. Bei „Aktuelle Position" gibt es nichts anzufahren.
    /// Gibt die nutzerlesbare Rückmeldung zurück.
    pub fn goto_reference(
        &mut self,
        reference: &StartReference,
        speed: f64,
    ) -> Result<String, AppError> {
        match reference {
            StartReference::Absolut => {
                self.move_to_position(0.0, 0.0, speed)?;
                Ok("Maschinen-Nullpunkt wird angefahren.".into())
            }
            StartReference::AktuellePosition => {
                Ok("Referenz ist die aktuelle Kopfposition — keine Bewegung nötig.".into())
            }
            StartReference::Benutzerursprung => {
                // Nicht während eines laufenden Jobs — soweit der Treiber
                // einen Status liefert (ADR 0020 §F).
                if self.driver_capabilities().position_read && self.read_status()?.is_running {
                    return Err(AppError::new(
                        "laser_busy",
                        "Während eines laufenden Jobs wird nicht angefahren.",
                    ));
                }
                self.with_driver(true, |driver| {
                    driver.go_origin(speed).map_err(|error| {
                        AppError::wrap(
                            "laser_move",
                            "Benutzerursprung anfahren fehlgeschlagen.",
                            error.to_string(),
                        )
                    })
                })?;
                Ok("Benutzerursprung wird angefahren.".into())
            }
            StartReference::GespeicherterNullpunkt { id } => {
                let id = id.clone();
                self.move_to_saved_origin(&id, speed)?;
                Ok("Gespeicherter Nullpunkt wird angefahren.".into())
            }
        }
    }
}

#[cfg(test)]
mod tests;
