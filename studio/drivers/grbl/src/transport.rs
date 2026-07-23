//! Serieller GRBL-Transport. Hält den Port dauerhaft offen und reicht nur
//! bereits geparste Protokollzeilen an den Treiber weiter.

use std::collections::VecDeque;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use studio_core::{DriverConsoleBuffer, DriverConsoleDirection, DriverConsoleLine, DriverError};

use crate::protocol::{parse_line, GrblLine, GrblStatus};

const IO_TIMEOUT: Duration = Duration::from_millis(100);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
const COMMAND_TIMEOUT: Duration = Duration::from_secs(2);
// Einzelne zustandsändernde Befehle (z. B. M4 nach einer längeren G0-Fahrt)
// werden von grblHAL mit dem Planer synchronisiert und erst nach Abschluss der
// vorausgehenden Bewegung quittiert. Das Jobfenster bleibt begrenzt, muss aber
// reale Maschinenfahrten abdecken.
const PROGRAM_ACK_TIMEOUT: Duration = Duration::from_secs(30);
// M3/M4/M5 werden von grblHAL mit dem Planer synchronisiert. Besonders das
// abschließende M5 wird erst nach allen zuvor angenommenen Bewegungen
// quittiert und benötigt deshalb ein jobtaugliches, weiterhin begrenztes
// Wartefenster.
const PLANNER_SYNC_TIMEOUT: Duration = Duration::from_secs(30 * 60);
// Legacy-GRBL garantiert 128 Byte seriellen RX-Puffer. Zwölf Byte Reserve
// vermeiden Randfälle durch Firmware-/Zeilenendebehandlung; grblHAL-Builds
// mit größerem Puffer funktionieren damit ebenfalls, nur konservativer.
const STREAM_RX_WINDOW: usize = 116;
const JOB_ABORTED: &str = "GRBL-Job wurde abgebrochen.";

#[derive(Clone)]
pub struct SerialTransport {
    requests: mpsc::Sender<TransportRequest>,
    priority: mpsc::Sender<PriorityRequest>,
    port_name: String,
    baud: u32,
    console: DriverConsoleBuffer,
}

enum TransportRequest {
    Command(String, mpsc::Sender<Result<(), DriverError>>),
    Program(Vec<u8>, mpsc::Sender<Result<usize, DriverError>>),
}

enum PriorityRequest {
    Status(mpsc::Sender<Result<GrblStatus, DriverError>>),
    Stop(u8, mpsc::Sender<Result<(), DriverError>>),
}

impl SerialTransport {
    pub fn connect(port_name: &str, baud: u32) -> Result<Self, DriverError> {
        let mut port = serialport::new(port_name, baud)
            .timeout(IO_TIMEOUT)
            .open()
            .map_err(|error| transport_error("Seriellen Port öffnen", error))?;
        let console = DriverConsoleBuffer::default();
        handshake_port(&mut *port, &console)?;
        let last_logged_status = Arc::new(Mutex::new(None));
        let (requests, receiver) = mpsc::channel();
        let (priority, priority_receiver) = mpsc::channel();
        let worker_console = console.clone();
        let worker_status = last_logged_status.clone();
        thread::Builder::new()
            .name("grbl-transport".into())
            .spawn(move || {
                worker_loop(
                    port,
                    receiver,
                    priority_receiver,
                    worker_console,
                    worker_status,
                )
            })
            .map_err(|error| {
                DriverError::Transport(format!(
                    "GRBL-Transport-Worker starten fehlgeschlagen: {error}"
                ))
            })?;
        Ok(Self {
            requests,
            priority,
            port_name: port_name.to_owned(),
            baud,
            console,
        })
    }

    pub fn console_snapshot(&self) -> Vec<DriverConsoleLine> {
        self.console.snapshot()
    }

    pub fn console_buffer(&self) -> DriverConsoleBuffer {
        self.console.clone()
    }

    pub fn matches(&self, port_name: &str, baud: u32) -> bool {
        self.port_name == port_name && self.baud == baud
    }

    pub fn status(&self) -> Result<GrblStatus, DriverError> {
        let (reply, result) = mpsc::channel();
        self.priority
            .send(PriorityRequest::Status(reply))
            .map_err(|_| worker_unavailable())?;
        result.recv().map_err(|_| worker_unavailable())?
    }

    pub fn send_program(&self, bytes: &[u8]) -> Result<usize, DriverError> {
        let (reply, result) = mpsc::channel();
        self.requests
            .send(TransportRequest::Program(bytes.to_vec(), reply))
            .map_err(|_| worker_unavailable())?;
        result.recv().map_err(|_| worker_unavailable())?
    }

    pub fn command(&self, command: &str) -> Result<(), DriverError> {
        let (reply, result) = mpsc::channel();
        self.requests
            .send(TransportRequest::Command(command.to_owned(), reply))
            .map_err(|_| worker_unavailable())?;
        result.recv().map_err(|_| worker_unavailable())?
    }

    pub fn stop(&self, command: u8) -> Result<(), DriverError> {
        let (reply, result) = mpsc::channel();
        self.priority
            .send(PriorityRequest::Stop(command, reply))
            .map_err(|_| worker_unavailable())?;
        result.recv().map_err(|_| worker_unavailable())?
    }
}

fn handshake_port(
    port: &mut dyn serialport::SerialPort,
    console: &DriverConsoleBuffer,
) -> Result<(), DriverError> {
    // Ein bereits laufender Controller antwortet auf die Leerzeile; ein
    // durch DTR neu gestarteter ESP32 erhält bis zu fünf Sekunden Bootzeit.
    port.write_all(b"\r")
        .map_err(|error| transport_error("Handshake senden", error))?;
    console.push(DriverConsoleDirection::Sent, "<Handshake>");
    port.flush()
        .map_err(|error| transport_error("Handshake senden", error))?;

    let deadline = Instant::now() + HANDSHAKE_TIMEOUT;
    let mut saw_welcome = false;
    while Instant::now() < deadline {
        if let Some(line) = read_line(port, deadline)? {
            console.push(DriverConsoleDirection::Received, &line);
            if matches!(parse_line(&line), Some(GrblLine::Welcome(_))) {
                saw_welcome = true;
                break;
            }
        }
    }

    // `$I` identifiziert auch einen Controller, der beim Öffnen keinen
    // Reset ausführt und daher keine neue Begrüßung sendet.
    port.write_all(b"$I\r")
        .map_err(|error| transport_error("Identitätsabfrage senden", error))?;
    console.push(DriverConsoleDirection::Sent, "$I");
    port.flush()
        .map_err(|error| transport_error("Identitätsabfrage senden", error))?;
    let deadline = Instant::now() + COMMAND_TIMEOUT;
    let mut saw_identity = false;
    while Instant::now() < deadline {
        let Some(line) = read_line(port, deadline)? else {
            continue;
        };
        console.push(DriverConsoleDirection::Received, &line);
        match parse_line(&line) {
            Some(GrblLine::Info(info)) if info.starts_with("[VER:") => {
                saw_identity = true;
            }
            Some(GrblLine::Ack) => break,
            // Ein frisch gestarteter Controller kann Diagnosebefehle im
            // Alarmzustand zunächst mit error:9/ALARM ablehnen. Eine zuvor
            // erkannte echte GRBL-Begrüßung bleibt dennoch ein gültiger
            // Handshake; der Zustand wird separat über `?` sichtbar.
            Some(GrblLine::Error(_)) | Some(GrblLine::Alarm(_)) if saw_welcome => break,
            Some(GrblLine::Error(error)) => return Err(protocol_error("$I", &error.to_string())),
            Some(GrblLine::Alarm(alarm)) => {
                return Err(protocol_error("ALARM", &alarm.to_string()))
            }
            _ => {}
        }
    }
    if saw_welcome || saw_identity {
        Ok(())
    } else {
        Err(DriverError::Transport(
            "Der serielle Port antwortet nicht als GRBL-Controller.".into(),
        ))
    }
}

fn status_on_port(
    port: &mut dyn serialport::SerialPort,
    console: &DriverConsoleBuffer,
    last_logged_status: &Mutex<Option<String>>,
) -> Result<GrblStatus, DriverError> {
    port.write_all(b"?")
        .map_err(|error| transport_error("Statusabfrage senden", error))?;
    port.flush()
        .map_err(|error| transport_error("Statusabfrage senden", error))?;
    let deadline = Instant::now() + COMMAND_TIMEOUT;
    while Instant::now() < deadline {
        let Some(line) = read_line(port, deadline)? else {
            continue;
        };
        match parse_line(&line) {
            Some(GrblLine::Status(status)) => {
                log_status_if_changed(console, last_logged_status, &line);
                return Ok(status);
            }
            Some(GrblLine::Alarm(alarm)) => {
                console.push(DriverConsoleDirection::Received, &line);
                return Err(protocol_error("ALARM", &alarm.to_string()));
            }
            _ => console.push(DriverConsoleDirection::Received, &line),
        }
    }
    Err(DriverError::Transport(
        "Zeitüberschreitung bei der GRBL-Statusabfrage.".into(),
    ))
}

/// Streamt ein bereits kompiliertes G-Code-Programm mit konservativem
/// Zeichenfenster. Mehrere Zeilen dürfen gleichzeitig im garantierten
/// GRBL-RX-Puffer liegen; jedes `ok` gibt exakt die älteste Zeile frei.
fn send_program_on_port(
    port: &mut dyn serialport::SerialPort,
    console: &DriverConsoleBuffer,
    priority: &mpsc::Receiver<PriorityRequest>,
    last_logged_status: &Mutex<Option<String>>,
    bytes: &[u8],
) -> Result<usize, DriverError> {
    let program = std::str::from_utf8(bytes).map_err(|error| {
        DriverError::Transport(format!("G-Code ist kein gültiges UTF-8: {error}"))
    })?;
    let lines = program_lines(program).collect::<Vec<_>>();
    let mut next = 0;
    let mut confirmed = 0;
    let mut buffered_bytes = 0;
    let mut pending = VecDeque::<(&str, usize)>::new();
    let mut status_replies = Vec::new();

    while next < lines.len() || !pending.is_empty() {
        service_stream_priority(port, console, priority, &mut status_replies)?;
        let mut wrote = false;
        while let Some(&line) = lines.get(next) {
            let encoded_len = encoded_line_len(line)?;
            if !window_accepts(buffered_bytes, encoded_len, pending.is_empty()) {
                break;
            }
            port.write_all(line.as_bytes())
                .and_then(|()| port.write_all(b"\r"))
                .map_err(|error| transport_error("G-Code senden", error))?;
            console.push(DriverConsoleDirection::Sent, line);
            pending.push_back((line, encoded_len));
            buffered_bytes += encoded_len;
            next += 1;
            wrote = true;
        }
        if wrote {
            port.flush()
                .map_err(|error| transport_error("G-Code senden", error))?;
        }

        let Some(&(oldest, _)) = pending.front() else {
            continue;
        };
        let timeout = if is_planner_sync_command(oldest) {
            PLANNER_SYNC_TIMEOUT
        } else {
            PROGRAM_ACK_TIMEOUT
        };
        let deadline = Instant::now() + timeout;
        let response = loop {
            service_stream_priority(port, console, priority, &mut status_replies)?;
            let slice_deadline = deadline.min(Instant::now() + IO_TIMEOUT);
            let Some(response) = read_line(port, slice_deadline)? else {
                if Instant::now() >= deadline {
                    break Err(DriverError::Transport(format!(
                        "Keine GRBL-Quittung für „{oldest}“."
                    )));
                }
                continue;
            };
            match parse_line(&response) {
                Some(GrblLine::Ack) => break Ok(()),
                Some(GrblLine::Status(status)) => {
                    log_status_if_changed(console, last_logged_status, &response);
                    for reply in status_replies.drain(..) {
                        let _ = reply.send(Ok(status.clone()));
                    }
                }
                Some(GrblLine::Error(error)) => {
                    console.push(DriverConsoleDirection::Received, &response);
                    break Err(line_protocol_error(oldest, "error", &error.to_string()));
                }
                Some(GrblLine::Alarm(alarm)) => {
                    console.push(DriverConsoleDirection::Received, &response);
                    break Err(line_protocol_error(oldest, "ALARM", &alarm.to_string()));
                }
                _ => console.push(DriverConsoleDirection::Received, &response),
            }
            if Instant::now() >= deadline {
                break Err(DriverError::Transport(format!(
                    "Keine GRBL-Quittung für „{oldest}“."
                )));
            }
        };
        if let Err(error) = response {
            // Soft-Reset ist der sichere Abbruchpfad von GRBL und beendet
            // auch einen möglicherweise noch aktiven Laserzustand.
            if !matches!(&error, DriverError::Transport(message) if message == JOB_ABORTED) {
                let _ = port.write_all(&[0x18]);
                let _ = port.flush();
            }
            return Err(error);
        }
        let (_, encoded_len) = pending.pop_front().expect("Quittung ohne offene Zeile");
        buffered_bytes -= encoded_len;
        confirmed += 1;
    }
    for reply in status_replies {
        let _ = reply.send(Err(DriverError::Transport(
            "Statusantwort blieb beim Ende des GRBL-Streams aus.".into(),
        )));
    }
    Ok(confirmed)
}

fn command_on_port(
    port: &mut dyn serialport::SerialPort,
    console: &DriverConsoleBuffer,
    command: &str,
) -> Result<(), DriverError> {
    console.push(DriverConsoleDirection::Sent, command);
    port.write_all(command.as_bytes())
        .and_then(|()| port.write_all(b"\r"))
        .and_then(|()| port.flush())
        .map_err(|error| transport_error("Konsolenbefehl senden", error))?;
    let deadline = Instant::now() + COMMAND_TIMEOUT;
    while Instant::now() < deadline {
        let Some(response) = read_line(port, deadline)? else {
            continue;
        };
        console.push(DriverConsoleDirection::Received, &response);
        match parse_line(&response) {
            Some(GrblLine::Ack) | Some(GrblLine::Status(_)) => return Ok(()),
            Some(GrblLine::Error(error)) => {
                return Err(protocol_error("error", &error.to_string()));
            }
            Some(GrblLine::Alarm(alarm)) => {
                return Err(protocol_error("ALARM", &alarm.to_string()));
            }
            _ => {}
        }
    }
    Err(DriverError::Transport(format!(
        "Keine GRBL-Quittung für „{command}“."
    )))
}

fn worker_loop(
    mut port: Box<dyn serialport::SerialPort>,
    requests: mpsc::Receiver<TransportRequest>,
    priority: mpsc::Receiver<PriorityRequest>,
    console: DriverConsoleBuffer,
    last_logged_status: Arc<Mutex<Option<String>>>,
) {
    loop {
        while let Ok(request) = priority.try_recv() {
            service_idle_priority(&mut *port, &console, &last_logged_status, request);
        }
        let request = match requests.recv_timeout(Duration::from_millis(20)) {
            Ok(request) => request,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        match request {
            TransportRequest::Command(command, reply) => {
                let _ = reply.send(command_on_port(&mut *port, &console, &command));
            }
            TransportRequest::Program(program, reply) => {
                let _ = reply.send(send_program_on_port(
                    &mut *port,
                    &console,
                    &priority,
                    &last_logged_status,
                    &program,
                ));
            }
        }
    }
}

fn service_idle_priority(
    port: &mut dyn serialport::SerialPort,
    console: &DriverConsoleBuffer,
    last_logged_status: &Mutex<Option<String>>,
    request: PriorityRequest,
) {
    match request {
        PriorityRequest::Status(reply) => {
            let _ = reply.send(status_on_port(port, console, last_logged_status));
        }
        PriorityRequest::Stop(command, reply) => {
            let _ = reply.send(send_stop(port, console, command));
        }
    }
}

fn service_stream_priority(
    port: &mut dyn serialport::SerialPort,
    console: &DriverConsoleBuffer,
    priority: &mpsc::Receiver<PriorityRequest>,
    status_replies: &mut Vec<mpsc::Sender<Result<GrblStatus, DriverError>>>,
) -> Result<(), DriverError> {
    let need_status_request = status_replies.is_empty();
    while let Ok(request) = priority.try_recv() {
        match request {
            PriorityRequest::Status(reply) => status_replies.push(reply),
            PriorityRequest::Stop(command, reply) => {
                let result = send_stop(port, console, command);
                let succeeded = result.is_ok();
                let _ = reply.send(result);
                return Err(DriverError::Transport(if succeeded {
                    JOB_ABORTED.into()
                } else {
                    "GRBL-Jobabbruch fehlgeschlagen.".into()
                }));
            }
        }
    }
    if need_status_request && !status_replies.is_empty() {
        port.write_all(b"?")
            .and_then(|()| port.flush())
            .map_err(|error| transport_error("Statusabfrage senden", error))?;
    }
    Ok(())
}

fn send_stop(
    port: &mut dyn serialport::SerialPort,
    console: &DriverConsoleBuffer,
    command: u8,
) -> Result<(), DriverError> {
    port.write_all(&[command])
        .and_then(|()| port.flush())
        .map_err(|error| transport_error("Soft-Reset senden", error))?;
    let label = if command == 0x19 {
        "<Stop 0x19>"
    } else {
        "<Soft-Reset 0x18>"
    };
    console.push(DriverConsoleDirection::Sent, label);
    Ok(())
}

fn log_status_if_changed(
    console: &DriverConsoleBuffer,
    last_logged_status: &Mutex<Option<String>>,
    line: &str,
) {
    let Ok(mut previous) = last_logged_status.lock() else {
        return;
    };
    if previous.as_deref() != Some(line) {
        console.push(DriverConsoleDirection::Received, line);
        *previous = Some(line.to_owned());
    }
}

fn worker_unavailable() -> DriverError {
    DriverError::Transport("Der GRBL-Transport-Worker ist nicht verfügbar.".into())
}

fn is_planner_sync_command(line: &str) -> bool {
    line.split_ascii_whitespace()
        .next()
        .is_some_and(|word| matches!(word, "M3" | "M4" | "M5"))
}

fn encoded_line_len(line: &str) -> Result<usize, DriverError> {
    let len = line.len() + 1;
    if len > STREAM_RX_WINDOW {
        Err(DriverError::Transport(format!(
            "G-Code-Zeile überschreitet mit {len} Byte das sichere GRBL-Pufferfenster."
        )))
    } else {
        Ok(len)
    }
}

fn window_accepts(buffered: usize, next: usize, empty: bool) -> bool {
    empty || buffered + next <= STREAM_RX_WINDOW
}

fn program_lines(program: &str) -> impl Iterator<Item = &str> {
    program
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with(';') && !line.starts_with('('))
}

fn read_line(
    port: &mut dyn serialport::SerialPort,
    deadline: Instant,
) -> Result<Option<String>, DriverError> {
    let mut bytes = Vec::new();
    let mut byte = [0_u8; 1];
    while Instant::now() < deadline {
        match port.read(&mut byte) {
            Ok(0) => continue,
            Ok(_) if matches!(byte[0], b'\r' | b'\n') => {
                if !bytes.is_empty() {
                    return Ok(Some(String::from_utf8_lossy(&bytes).into_owned()));
                }
            }
            Ok(_) => bytes.push(byte[0]),
            Err(error) if error.kind() == std::io::ErrorKind::TimedOut => continue,
            Err(error) => return Err(transport_error("Serielle Antwort lesen", error)),
        }
    }
    Ok((!bytes.is_empty()).then(|| String::from_utf8_lossy(&bytes).into_owned()))
}

fn transport_error(context: &str, error: impl std::fmt::Display) -> DriverError {
    DriverError::Transport(format!("{context} fehlgeschlagen: {error}"))
}

fn protocol_error(context: &str, detail: &str) -> DriverError {
    DriverError::Transport(format!("GRBL {context}: {detail}"))
}

fn line_protocol_error(line: &str, kind: &str, detail: &str) -> DriverError {
    DriverError::Transport(format!("GRBL {kind} für „{line}“: {detail}"))
}

#[cfg(test)]
mod tests {
    use super::{
        encoded_line_len, is_planner_sync_command, program_lines, window_accepts, STREAM_RX_WINDOW,
    };

    #[test]
    fn streaming_filtert_kommentare_und_leerzeilen() {
        let lines: Vec<_> = program_lines("; Kopf\nG21\n\n(Info)\n M5 \n").collect();
        assert_eq!(lines, ["G21", "M5"]);
    }

    #[test]
    fn nur_spindelbefehle_erhalten_planersynchrones_timeout() {
        assert!(is_planner_sync_command("M5"));
        assert!(is_planner_sync_command("M4 S0"));
        assert!(is_planner_sync_command("M3 S200"));
        assert!(!is_planner_sync_command("G1 X10 Y20 S200"));
        assert!(!is_planner_sync_command("G0 X0 Y0"));
    }

    #[test]
    fn zeichenfenster_puffert_mehrere_zeilen_aber_nie_ueber_grenze() {
        let a = encoded_line_len("G1 X10 Y10 S200").unwrap();
        let b = encoded_line_len("G1 X20 Y20 S200").unwrap();
        assert!(window_accepts(0, a, true));
        assert!(window_accepts(a, b, false));
        assert!(!window_accepts(STREAM_RX_WINDOW - 3, b, false));
    }

    #[test]
    fn ueberlange_einzelzeile_wird_vor_dem_senden_abgewiesen() {
        let line = "X".repeat(STREAM_RX_WINDOW);
        assert!(encoded_line_len(&line).is_err());
    }
}
