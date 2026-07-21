//! GUI-Einstellungen-Workflow (ADR 0002): Dialog-Entwurf öffnen/übernehmen,
//! Ausschließlich softwareweite Studio-Einstellungen (ADR 0002).

use crate::ui::{HubTestStatus, SettingsDialogState, SettingsSection};
use studio_application::AppError;

use super::App;

impl App {
    /// Öffnet den Einstellungen-Dialog (Zahnrad) mit den aktuellen Werten.
    pub fn open_settings_dialog(&mut self) {
        self.settings_dialog = Some(SettingsDialogState {
            draft: self.ui_settings.clone(),
            section: SettingsSection::Arbeitsplatz,
            hub_status: self.hub_status.clone(),
            hub_sync_error: self.hub_sync_error.clone(),
            hub_backups: Vec::new(),
            backup_restore_confirm: None,
            shortcut_recording: None,
            shortcut_conflict: None,
            shortcut_error: None,
            confirm_shortcut_defaults: false,
        });
    }

    pub fn load_hub_backups(&self) {
        self.hub_runtime.fetch_backups();
    }

    pub fn restore_hub_backup(&mut self, index: usize) {
        let Some(backup) = self
            .settings_dialog
            .as_ref()
            .and_then(|dialog| dialog.hub_backups.get(index))
            .cloned()
        else {
            return;
        };
        // Den unmittelbar vorherigen lokalen Stand zuerst einreihen. Die
        // Configure-Kommandos sind geordnet; danach folgt der restaurierte Stand.
        self.hub_runtime.configure(
            &self.ui_settings,
            &self.laser_backend.registry,
            self.material_service.library(),
        );
        let result = match backup.kind {
            studio_application::HubBackupKind::UiSettings => {
                studio_core::UiSettings::from_json(&backup.payload).and_then(|settings| {
                    if settings.grid_size_mm != self.ui_settings.grid_size_mm {
                        self.renderer.invalidate_scene();
                    }
                    studio_application::save_ui_settings(&settings)?;
                    self.canvas.invert_marquee_direction = settings.invert_marquee_direction;
                    self.ui_settings = settings.clone();
                    if let Some(dialog) = self.settings_dialog.as_mut() {
                        dialog.draft = settings;
                    }
                    Ok(())
                })
            }
            studio_application::HubBackupKind::LaserProfiles => {
                serde_json::from_str::<studio_core::LaserRegistry>(&backup.payload)
                    .map_err(|error| error.to_string())
                    .and_then(|registry| {
                        self.laser_backend
                            .restore_registry(registry)
                            .map_err(|error| error.message().to_owned())
                    })
            }
            studio_application::HubBackupKind::MaterialProfiles => {
                serde_json::from_str::<studio_core::MaterialLibrary>(&backup.payload)
                    .map_err(|error| error.to_string())
                    .and_then(|library| {
                        self.material_service
                            .restore_library(library)
                            .map_err(|error| error.message().to_owned())
                    })
            }
        };
        match result {
            Ok(()) => {
                if let Some(dialog) = self.settings_dialog.as_mut() {
                    dialog.backup_restore_confirm = None;
                }
                self.apply_active_laser_workspace();
                self.hub_runtime.configure(
                    &self.ui_settings,
                    &self.laser_backend.registry,
                    self.material_service.library(),
                );
                self.toasts.success(format!(
                    "Sicherung von {} wiederhergestellt.",
                    backup.workplace_name
                ));
            }
            Err(message) => {
                self.app_error = Some(AppError::new("hub_backup_restore", message));
            }
        }
    }

    pub fn prepare_hub_backup_restore(&mut self, index: usize) {
        let Some(backup) = self
            .settings_dialog
            .as_ref()
            .and_then(|dialog| dialog.hub_backups.get(index))
            .cloned()
        else {
            return;
        };
        let summary = match backup.kind {
            studio_application::HubBackupKind::UiSettings => {
                match studio_core::UiSettings::from_json(&backup.payload) {
                    Ok(target) if target == self.ui_settings => {
                        vec!["Der gesicherte Stand entspricht dem lokalen Stand.".into()]
                    }
                    Ok(_) => vec![
                        "Oberfläche, Bedienung und Arbeitsplatz-Einstellungen werden ersetzt."
                            .into(),
                    ],
                    Err(error) => vec![format!("Sicherung ist ungültig: {error}")],
                }
            }
            studio_application::HubBackupKind::LaserProfiles => {
                serde_json::from_str::<studio_core::LaserRegistry>(&backup.payload).map_or_else(
                    |error| vec![format!("Sicherung ist ungültig: {error}")],
                    |target| {
                        profile_summary(
                            &self.laser_backend.registry.profiles,
                            &target.profiles,
                            |profile| (&profile.id, &profile.name),
                        )
                    },
                )
            }
            studio_application::HubBackupKind::MaterialProfiles => {
                serde_json::from_str::<studio_core::MaterialLibrary>(&backup.payload).map_or_else(
                    |error| vec![format!("Sicherung ist ungültig: {error}")],
                    |target| {
                        profile_summary(
                            &self.material_service.library().profiles,
                            &target.profiles,
                            |profile| (&profile.id, &profile.name),
                        )
                    },
                )
            }
        };
        if let Some(dialog) = self.settings_dialog.as_mut() {
            dialog.backup_restore_confirm =
                Some(crate::ui::BackupRestoreConfirmation { index, summary });
        }
    }

    pub fn test_hub_connection(&mut self) {
        let Some(state) = self.settings_dialog.as_mut() else {
            return;
        };
        state.hub_status = match studio_application::connect_hub(
            &state.draft.hub_url,
            &state.draft.workplace_id,
            &state.draft.workplace,
        ) {
            Ok(connection) => HubTestStatus::Connected(connection),
            Err(error) => HubTestStatus::Failed(error.message().to_string()),
        };
    }

    /// Übernimmt den GUI-Entwurf: klemmen, speichern, anwenden. Bei Erfolg true
    /// (Dialog schließen); bei Schreibfehler bleibt der Dialog offen und der
    /// Fehler erscheint im zentralen Kanal.
    pub fn commit_settings_dialog(&mut self) -> bool {
        let Some(st) = self.settings_dialog.as_ref() else {
            return false;
        };
        let mut draft = st.draft.clone();
        draft.sanitize();
        if let Err(message) = draft.shortcut_bindings.validate() {
            self.app_error = Some(AppError::new("shortcut_conflict", message));
            return false;
        }
        if let Err(error) = studio_application::save_ui_settings(&draft) {
            self.app_error = Some(AppError::new(
                "settings_write",
                format!("Einstellungen speichern fehlgeschlagen: {error}"),
            ));
            return false;
        }
        // Rasterweite steckt im gecachten Basis-Vertexpuffer → neu aufbauen.
        if draft.grid_size_mm != self.ui_settings.grid_size_mm {
            self.renderer.invalidate_scene();
        }
        self.canvas.invert_marquee_direction = draft.invert_marquee_direction;
        self.egui_ctx.set_zoom_factor(draft.ui_scale);
        self.ui_settings = draft;
        self.hub_runtime.configure(
            &self.ui_settings,
            &self.laser_backend.registry,
            self.material_service.library(),
        );
        self.toasts.success("Einstellungen gespeichert.");
        true
    }

    /// Übernimmt das jüngste Ergebnis des Hintergrund-Heartbeats in den
    /// sichtbaren Anwendungszustand. Netzwerkzugriff findet nie hier statt.
    pub fn poll_hub(&mut self) -> bool {
        let Some(result) = self.hub_runtime.try_result() else {
            return false;
        };
        self.hub_status = match result {
            super::hub::HubWorkerResult::Syncing(connection) => HubTestStatus::Syncing(connection),
            super::hub::HubWorkerResult::Connected(connection, sync) => {
                match sync {
                    Ok((report, catalog)) => {
                        // Sichtbare, aber nicht blockierende Meldungen: ein
                        // alter Hub setzt nur den Laserprofil-Sync aus (ADR
                        // 0020); Konflikte verlangen eine bewusste Auflösung.
                        self.hub_sync_error = if let Some(blocked) = &catalog.laser_sync_blocked {
                            Some(blocked.clone())
                        } else if catalog.conflicts.is_empty() {
                            None
                        } else {
                            Some(format!(
                                "{} Profilkonflikt(e) mit Hub. Die lokalen Profile wurden nicht überschrieben.",
                                catalog.conflicts.len()
                            ))
                        };
                        let mut catalog_changed = false;
                        let was_connected = self.laser_backend.is_connected();
                        for record in &catalog.records {
                            let result = match record.kind {
                                studio_application::CatalogKind::LaserProfile => {
                                    self.laser_backend.apply_shared_record(record)
                                }
                                studio_application::CatalogKind::MaterialProfile => {
                                    self.material_service.apply_shared_record(record)
                                }
                            };
                            match result {
                                Ok(changed) => catalog_changed |= changed,
                                Err(error) => {
                                    self.hub_sync_error = Some(error.message().to_owned());
                                }
                            }
                        }
                        if catalog_changed {
                            self.apply_active_laser_workspace();
                            self.hub_runtime.configure(
                                &self.ui_settings,
                                &self.laser_backend.registry,
                                self.material_service.library(),
                            );
                            // Hat ein empfangener Katalogstand die Verbindung
                            // beendet, den Hub-Lease mit abgeben (Lease ohne
                            // Verbindung blockiert das Gerät nur scheinbar).
                            if was_connected && !self.laser_backend.is_connected() {
                                self.hub_runtime.release_lease();
                            }
                        }
                        if report.received > 0 {
                            self.refresh_project_inbox();
                        }
                        if report.assets_uploaded > 0 || report.assets_downloaded > 0 {
                            if report.assets_downloaded > 0 {
                                self.refresh_asset_catalog();
                            }
                            self.image_dirty = true;
                        }
                    }
                    Err(message) => self.hub_sync_error = Some(message),
                }
                HubTestStatus::Connected(connection)
            }
            super::hub::HubWorkerResult::Failed(message) => {
                self.hub_sync_error = None;
                HubTestStatus::Failed(message)
            }
            super::hub::HubWorkerResult::Disabled => {
                self.hub_sync_error = None;
                HubTestStatus::Idle
            }
            super::hub::HubWorkerResult::Backups(backups) => {
                if let Some(dialog) = self.settings_dialog.as_mut() {
                    dialog.hub_backups = backups;
                }
                return true;
            }
        };
        if let Some(dialog) = self.settings_dialog.as_mut() {
            dialog.hub_status = self.hub_status.clone();
            dialog.hub_sync_error = self.hub_sync_error.clone();
        }
        true
    }
}

fn profile_summary<T, F>(current: &[T], target: &[T], identity: F) -> Vec<String>
where
    T: PartialEq,
    F: Fn(&T) -> (&String, &String),
{
    let mut lines = Vec::new();
    for item in target {
        let (id, name) = identity(item);
        match current.iter().find(|existing| identity(existing).0 == id) {
            None => lines.push(format!("+ {name}")),
            Some(existing) if existing != item => lines.push(format!("~ {name}")),
            Some(_) => {}
        }
    }
    for item in current {
        let (id, name) = identity(item);
        if !target.iter().any(|existing| identity(existing).0 == id) {
            lines.push(format!("- {name}"));
        }
    }
    if lines.is_empty() {
        lines.push("Der gesicherte Stand entspricht dem lokalen Stand.".into());
    }
    lines
}
