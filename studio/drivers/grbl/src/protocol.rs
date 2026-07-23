//! Reiner GRBL-Zeilenparser. Kein I/O, keine GUI und keine Application-Typen.

#[derive(Debug, Clone, PartialEq)]
pub struct GrblStatus {
    pub state: String,
    pub machine_position: Option<[f64; 3]>,
    pub work_position: Option<[f64; 3]>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GrblLine {
    Welcome(String),
    Ack,
    Error(GrblError),
    Alarm(GrblAlarm),
    Status(GrblStatus),
    Info(String),
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrblError {
    pub code: String,
    pub explanation: &'static str,
}

impl std::fmt::Display for GrblError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.code, self.explanation)
    }
}

fn error(code: &str) -> GrblError {
    let explanation = match code {
        "5" => "Homing ist deaktiviert",
        "8" => "Befehl ist im aktuellen Maschinenzustand nicht erlaubt",
        "9" => "Befehl ist im Alarmzustand gesperrt",
        "10" => "Soft-Limits erfordern zuvor ausgeführtes Homing",
        "13" => "Sicherheitstür ist geöffnet",
        "15" => "Jog-Ziel überschreitet die Maschinengrenzen",
        "16" => "Ungültiger Jog-Befehl",
        "17" => "Lasermodus erfordert einen PWM-fähigen Ausgang",
        "18" => "Reset-/Not-Aus-Eingang ist aktiv",
        "52" => "Einstellungswert liegt außerhalb des erlaubten Bereichs",
        "53" => "Einstellung ist in dieser Firmwarekonfiguration deaktiviert",
        _ => "unbekannter Fehlercode",
    };
    GrblError {
        code: code.to_owned(),
        explanation,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrblAlarm {
    pub code: String,
    pub explanation: &'static str,
}

impl std::fmt::Display for GrblAlarm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.code, self.explanation)
    }
}

fn alarm(code: &str) -> GrblAlarm {
    let explanation = match code {
        "1" => "Hard-Limit ausgelöst; Position vermutlich verloren",
        "2" => "Soft-Limit überschritten",
        "3" => "Reset während einer Bewegung",
        "4" => "Antasten fehlgeschlagen: Sonde war bereits ausgelöst",
        "5" => "Antasten fehlgeschlagen: Sonde wurde nicht ausgelöst",
        "6" => "Homing durch Reset abgebrochen",
        "7" => "Homing fehlgeschlagen: Sicherheitstür geöffnet",
        "8" => "Homing fehlgeschlagen: Endschalter beim Freifahren nicht gelöst",
        "9" => "Homing fehlgeschlagen: Endschalter nicht gefunden",
        "10" => "Not-Aus ausgelöst",
        "11" => "Homing erforderlich",
        _ => "unbekannter Alarmcode",
    };
    GrblAlarm {
        code: code.to_owned(),
        explanation,
    }
}

pub fn parse_line(raw: &str) -> Option<GrblLine> {
    let line = raw.trim_matches(['\r', '\n', ' ']);
    if line.is_empty() {
        return None;
    }
    if line.starts_with("Grbl ") {
        return Some(GrblLine::Welcome(line.to_owned()));
    }
    if line == "ok" {
        return Some(GrblLine::Ack);
    }
    if let Some(code) = line.strip_prefix("error:") {
        return Some(GrblLine::Error(error(code.trim())));
    }
    if let Some(code) = line.strip_prefix("ALARM:") {
        return Some(GrblLine::Alarm(alarm(code.trim())));
    }
    if line.starts_with('<') && line.ends_with('>') {
        return parse_status(line).map(GrblLine::Status);
    }
    if line.starts_with('[') && line.ends_with(']') {
        return Some(GrblLine::Info(line.to_owned()));
    }
    Some(GrblLine::Other(line.to_owned()))
}

fn parse_status(line: &str) -> Option<GrblStatus> {
    let body = line.strip_prefix('<')?.strip_suffix('>')?;
    let mut fields = body.split('|');
    let state = fields.next()?.to_owned();
    let mut status = GrblStatus {
        state,
        machine_position: None,
        work_position: None,
    };
    for field in fields {
        if let Some(value) = field.strip_prefix("MPos:") {
            status.machine_position = parse_xyz(value);
        } else if let Some(value) = field.strip_prefix("WPos:") {
            status.work_position = parse_xyz(value);
        }
    }
    Some(status)
}

fn parse_xyz(value: &str) -> Option<[f64; 3]> {
    let mut values = value.split(',').map(str::parse::<f64>);
    let xyz = [
        values.next()?.ok()?,
        values.next()?.ok()?,
        values.next()?.ok()?,
    ];
    xyz.iter().all(|value| value.is_finite()).then_some(xyz)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erkennt_handshake_quittung_und_fehler() {
        assert!(matches!(
            parse_line("Grbl 1.1f ['$' for help]"),
            Some(GrblLine::Welcome(_))
        ));
        assert_eq!(parse_line("ok\r\n"), Some(GrblLine::Ack));
        assert_eq!(
            parse_line("error:18"),
            Some(GrblLine::Error(GrblError {
                code: "18".into(),
                explanation: "Reset-/Not-Aus-Eingang ist aktiv",
            }))
        );
        assert_eq!(
            parse_line("ALARM:1"),
            Some(GrblLine::Alarm(GrblAlarm {
                code: "1".into(),
                explanation: "Hard-Limit ausgelöst; Position vermutlich verloren",
            }))
        );
        let Some(GrblLine::Alarm(unknown)) = parse_line("ALARM:4711") else {
            panic!("Alarm erwartet");
        };
        assert_eq!(unknown.code, "4711");
        assert_eq!(unknown.explanation, "unbekannter Alarmcode");
    }

    #[test]
    fn status_parser_behaelt_maschinen_und_arbeitsposition() {
        let Some(GrblLine::Status(status)) =
            parse_line("<Idle|MPos:1.250,-2.000,0.000|WPos:0.250,3.000,0.000|FS:0,0>")
        else {
            panic!("Status erwartet");
        };
        assert_eq!(status.state, "Idle");
        assert_eq!(status.machine_position, Some([1.25, -2.0, 0.0]));
        assert_eq!(status.work_position, Some([0.25, 3.0, 0.0]));
    }
}
