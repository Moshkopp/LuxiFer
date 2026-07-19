//! Hintergrundverbindung zum optionalen Hub-Dienst (ADR 0012).

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::time::Duration;

use studio_application::{AppError, HubConnection};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Clone)]
struct HubConfig {
    url: String,
    workplace_id: String,
    workplace_name: String,
    settings: studio_core::UiSettings,
    lasers: studio_core::LaserRegistry,
    materials: studio_core::MaterialLibrary,
}

enum WorkerCommand {
    Configure(Option<Box<HubConfig>>),
    FetchBackups,
}

pub(super) enum HubWorkerResult {
    Syncing(HubConnection),
    Connected(
        HubConnection,
        Result<
            (
                studio_application::HubSyncReport,
                studio_application::SharedCatalogSync,
            ),
            String,
        >,
    ),
    Failed(String),
    Disabled,
    Backups(Vec<studio_application::HubWorkplaceBackup>),
}

enum LeaseCommand {
    Configure(Option<Box<HubConfig>>),
    Acquire {
        controller_id: String,
        controller_name: String,
        force: bool,
    },
    Release,
    Usage(studio_application::LeaseUsage),
}

pub(super) enum LeaseWorkerResult {
    Acquired,
    Denied(studio_application::HubLease),
    Released,
    ReleaseRequested,
    Lost(String),
}

struct ActiveLease {
    controller_id: String,
    controller_name: String,
    token: String,
    usage: studio_application::LeaseUsage,
}

struct PendingLease {
    controller_id: String,
    controller_name: String,
}

pub(super) struct HubRuntime {
    command_tx: Sender<WorkerCommand>,
    result_rx: Receiver<HubWorkerResult>,
    lease_tx: Sender<LeaseCommand>,
    lease_rx: Receiver<LeaseWorkerResult>,
}

impl HubRuntime {
    pub fn new(
        settings: &studio_core::UiSettings,
        lasers: &studio_core::LaserRegistry,
        materials: &studio_core::MaterialLibrary,
    ) -> Result<Self, AppError> {
        let (command_tx, command_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        let (lease_tx, lease_command_rx) = mpsc::channel();
        let (lease_result_tx, lease_rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("hub-heartbeat".into())
            .spawn(move || worker(command_rx, result_tx))
            .map_err(|error| {
                AppError::wrap(
                    "hub_worker_start",
                    "Hub-Synchronisierung konnte nicht gestartet werden.",
                    error.to_string(),
                )
            })?;
        std::thread::Builder::new()
            .name("hub-lease".into())
            .spawn(move || lease_worker(lease_command_rx, lease_result_tx))
            .map_err(|error| {
                AppError::wrap(
                    "hub_lease_worker_start",
                    "Hub-Maschinenkoordination konnte nicht gestartet werden.",
                    error.to_string(),
                )
            })?;
        let runtime = Self {
            command_tx,
            result_rx,
            lease_tx,
            lease_rx,
        };
        runtime.configure(settings, lasers, materials);
        Ok(runtime)
    }

    pub fn configure(
        &self,
        settings: &studio_core::UiSettings,
        lasers: &studio_core::LaserRegistry,
        materials: &studio_core::MaterialLibrary,
    ) {
        let config = settings.hub_enabled.then(|| {
            Box::new(HubConfig {
                url: settings.hub_url.clone(),
                workplace_id: settings.workplace_id.clone(),
                workplace_name: settings.workplace.clone(),
                settings: settings.clone(),
                lasers: lasers.clone(),
                materials: materials.clone(),
            })
        });
        let _ = self.command_tx.send(WorkerCommand::Configure(config));
        let lease_config = settings.hub_enabled.then(|| {
            Box::new(HubConfig {
                url: settings.hub_url.clone(),
                workplace_id: settings.workplace_id.clone(),
                workplace_name: settings.workplace.clone(),
                settings: settings.clone(),
                lasers: lasers.clone(),
                materials: materials.clone(),
            })
        });
        let _ = self.lease_tx.send(LeaseCommand::Configure(lease_config));
    }

    pub fn try_result(&self) -> Option<HubWorkerResult> {
        self.result_rx.try_recv().ok()
    }

    pub fn fetch_backups(&self) {
        let _ = self.command_tx.send(WorkerCommand::FetchBackups);
    }

    pub fn acquire_lease(&self, controller_id: String, controller_name: String, force: bool) {
        let _ = self.lease_tx.send(LeaseCommand::Acquire {
            controller_id,
            controller_name,
            force,
        });
    }

    pub fn release_lease(&self) {
        let _ = self.lease_tx.send(LeaseCommand::Release);
    }

    pub fn set_lease_usage(&self, usage: studio_application::LeaseUsage) {
        let _ = self.lease_tx.send(LeaseCommand::Usage(usage));
    }

    pub fn try_lease_result(&self) -> Option<LeaseWorkerResult> {
        self.lease_rx.try_recv().ok()
    }
}

fn lease_worker(command_rx: Receiver<LeaseCommand>, result_tx: Sender<LeaseWorkerResult>) {
    let mut config: Option<Box<HubConfig>> = None;
    let mut active: Option<ActiveLease> = None;
    let mut pending: Option<PendingLease> = None;
    loop {
        match command_rx.recv_timeout(HEARTBEAT_INTERVAL) {
            Ok(LeaseCommand::Configure(next)) => {
                let coordination_changed = match (config.as_deref(), next.as_deref()) {
                    (Some(previous), Some(next)) => {
                        previous.url != next.url
                            || previous.workplace_id != next.workplace_id
                            || previous.workplace_name != next.workplace_name
                    }
                    (None, None) => false,
                    _ => true,
                };
                if coordination_changed {
                    release_active(config.as_deref(), &mut active);
                    pending = None;
                }
                config = next;
            }
            Ok(LeaseCommand::Acquire {
                controller_id,
                controller_name,
                force,
            }) => {
                let Some(current) = config.as_ref() else {
                    continue;
                };
                match studio_application::acquire_lease(
                    &current.url,
                    &controller_id,
                    &controller_name,
                    &current.workplace_id,
                    &current.workplace_name,
                    force,
                ) {
                    Ok(reply) if reply.granted => {
                        active = reply.token.clone().map(|token| ActiveLease {
                            controller_id,
                            controller_name,
                            token,
                            usage: studio_application::LeaseUsage::Idle,
                        });
                        pending = None;
                        let _ = result_tx.send(LeaseWorkerResult::Acquired);
                    }
                    Ok(reply) => {
                        pending = reply.release_requested.then_some(PendingLease {
                            controller_id,
                            controller_name,
                        });
                        let _ = result_tx.send(LeaseWorkerResult::Denied(reply));
                    }
                    Err(error) => {
                        let _ = result_tx.send(LeaseWorkerResult::Lost(error.message().into()));
                    }
                }
            }
            Ok(LeaseCommand::Release) => {
                pending = None;
                release_active(config.as_deref(), &mut active);
                let _ = result_tx.send(LeaseWorkerResult::Released);
            }
            Ok(LeaseCommand::Usage(usage)) => {
                if let Some(lease) = active.as_mut() {
                    lease.usage = usage;
                    if let Some(current) = config.as_ref() {
                        let _ = studio_application::heartbeat_lease(
                            &current.url,
                            &lease.controller_id,
                            &current.workplace_id,
                            &lease.token,
                            lease.usage,
                        );
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                let Some(current) = config.as_ref() else {
                    continue;
                };
                if active.is_none() {
                    let Some(request) = pending.as_ref() else {
                        continue;
                    };
                    match studio_application::acquire_lease(
                        &current.url,
                        &request.controller_id,
                        &request.controller_name,
                        &current.workplace_id,
                        &current.workplace_name,
                        false,
                    ) {
                        Ok(reply) if reply.granted => {
                            active = reply.token.map(|token| ActiveLease {
                                controller_id: request.controller_id.clone(),
                                controller_name: request.controller_name.clone(),
                                token,
                                usage: studio_application::LeaseUsage::Idle,
                            });
                            pending = None;
                            let _ = result_tx.send(LeaseWorkerResult::Acquired);
                        }
                        Ok(_) => {}
                        Err(error) => {
                            pending = None;
                            let _ = result_tx.send(LeaseWorkerResult::Lost(error.message().into()));
                        }
                    }
                    continue;
                }
                let Some(lease) = active.as_ref() else {
                    continue;
                };
                let (controller_id, controller_name, usage) = (
                    lease.controller_id.clone(),
                    lease.controller_name.clone(),
                    lease.usage,
                );
                match studio_application::heartbeat_lease(
                    &current.url,
                    &controller_id,
                    &current.workplace_id,
                    &lease.token,
                    usage,
                ) {
                    Ok(reply)
                        if reply.release_requested
                            && usage == studio_application::LeaseUsage::Idle =>
                    {
                        release_active(config.as_deref(), &mut active);
                        let _ = result_tx.send(LeaseWorkerResult::ReleaseRequested);
                    }
                    Ok(_) => {}
                    // Hub erreichbar, aber der Lease ist dort unbekannt (409;
                    // z. B. Hub-Neustart — Leases liegen nur im Speicher):
                    // still neu anfordern, statt laufende lokale Arbeit zu
                    // unterbrechen. Getrennt wird erst, wenn nachweislich ein
                    // anderer Arbeitsplatz das Gerät hält.
                    Err(error) if error.code() == "hub_status" => {
                        match studio_application::acquire_lease(
                            &current.url,
                            &controller_id,
                            &controller_name,
                            &current.workplace_id,
                            &current.workplace_name,
                            false,
                        ) {
                            Ok(reply) if reply.granted => {
                                if let (Some(lease), Some(token)) = (active.as_mut(), reply.token) {
                                    lease.token = token;
                                }
                            }
                            Ok(reply) => {
                                active = None;
                                let holder = reply
                                    .holder_name
                                    .unwrap_or_else(|| "einen anderen Arbeitsplatz".into());
                                let _ = result_tx.send(LeaseWorkerResult::Lost(format!(
                                    "Gerät ist inzwischen durch {holder} belegt."
                                )));
                            }
                            // Hub zwischenzeitlich wieder weg: Lease behalten,
                            // nächster Zyklus versucht es erneut.
                            Err(_) => {}
                        }
                    }
                    // Hub nicht erreichbar: Koordination setzt aus, die lokale
                    // Verbindung bleibt bestehen (Hub ist nie Voraussetzung
                    // für lokale Arbeit). Der Lease wird mit altem Token
                    // weiterversucht, sobald der Hub wieder antwortet.
                    Err(_) => {}
                }
            }
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

fn release_active(config: Option<&HubConfig>, active: &mut Option<ActiveLease>) {
    if let (Some(current), Some(lease)) = (config, active.as_ref()) {
        let _ = studio_application::release_lease(
            &current.url,
            &lease.controller_id,
            &current.workplace_id,
            &lease.token,
        );
    }
    *active = None;
}

fn worker(command_rx: Receiver<WorkerCommand>, result_tx: Sender<HubWorkerResult>) {
    let mut config: Option<Box<HubConfig>> = None;
    let mut event_cursor = 0_u64;
    let mut server_instance: Option<String> = None;
    loop {
        match config.as_ref() {
            Some(current) => {
                let current = current.clone();
                let connection = studio_application::connect_hub(
                    &current.url,
                    &current.workplace_id,
                    &current.workplace_name,
                );
                let connected = connection.is_ok();
                let result = connection
                    .map(|connection| {
                        if server_instance.as_deref()
                            != Some(connection.handshake.instance_id.as_str())
                        {
                            event_cursor = 0;
                            server_instance = Some(connection.handshake.instance_id.clone());
                        }
                        if result_tx
                            .send(HubWorkerResult::Syncing(connection.clone()))
                            .is_err()
                        {
                            return HubWorkerResult::Connected(
                                connection,
                                Err("Hub-Statuskanal wurde geschlossen.".into()),
                            );
                        }
                        let capabilities = connection.handshake.capabilities.clone();
                        let sync = studio_application::sync_assets(&current.url)
                            .and_then(|mut report| {
                                let catalog = studio_application::sync_shared_catalog(
                                    &current.url,
                                    &current.workplace_id,
                                    &capabilities,
                                )?;
                                report.backups_uploaded =
                                    studio_application::upload_workplace_backups(
                                        &current.url,
                                        &current.settings,
                                        &current.lasers,
                                        &current.materials,
                                    )?;
                                let projects = studio_application::sync_project_revisions(
                                    &current.url,
                                    &current.workplace_id,
                                )?;
                                report.uploaded += projects.uploaded;
                                report.pending += projects.pending;
                                report.received += projects.received;
                                Ok((report, catalog))
                            })
                            .map_err(|error| error.message().to_owned());
                        HubWorkerResult::Connected(connection, sync)
                    })
                    .unwrap_or_else(|error| HubWorkerResult::Failed(error.message().to_owned()));
                if result_tx.send(result).is_err() {
                    return;
                }
                if connected {
                    match studio_application::wait_for_project_event(
                        &current.url,
                        &current.workplace_id,
                        event_cursor,
                    ) {
                        Ok(event) => event_cursor = event.cursor,
                        Err(_) => match command_rx.recv_timeout(HEARTBEAT_INTERVAL) {
                            Ok(WorkerCommand::Configure(next)) => {
                                config = next;
                                event_cursor = 0;
                                server_instance = None;
                            }
                            Ok(WorkerCommand::FetchBackups) => {
                                send_backups(&current, &result_tx);
                            }
                            Err(RecvTimeoutError::Timeout) => {}
                            Err(RecvTimeoutError::Disconnected) => return,
                        },
                    }
                } else {
                    match command_rx.recv_timeout(HEARTBEAT_INTERVAL) {
                        Ok(WorkerCommand::Configure(next)) => {
                            config = next;
                            event_cursor = 0;
                            server_instance = None;
                        }
                        Ok(WorkerCommand::FetchBackups) => {
                            send_backups(&current, &result_tx);
                        }
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                }
                match command_rx.try_recv() {
                    Ok(WorkerCommand::Configure(next)) => {
                        config = next;
                        event_cursor = 0;
                        server_instance = None;
                    }
                    Ok(WorkerCommand::FetchBackups) => {
                        send_backups(&current, &result_tx);
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => return,
                }
            }
            None => match command_rx.recv() {
                Ok(WorkerCommand::Configure(next)) => {
                    config = next;
                    event_cursor = 0;
                    server_instance = None;
                    if config.is_none() && result_tx.send(HubWorkerResult::Disabled).is_err() {
                        return;
                    }
                }
                Ok(WorkerCommand::FetchBackups) => {}
                Err(_) => return,
            },
        }
    }
}

fn send_backups(config: &HubConfig, result_tx: &Sender<HubWorkerResult>) {
    let result = match studio_application::list_workplace_backups(&config.url) {
        Ok(backups) => HubWorkerResult::Backups(backups),
        Err(error) => HubWorkerResult::Failed(error.message().to_owned()),
    };
    let _ = result_tx.send(result);
}
