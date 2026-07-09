//! UDP-Transport für den Ruida RDC6445G. Echte I/O.
//!
//! Ports: senden → 50200, empfangen ← 40200 (lokal binden). Nach der
//! HW-verifizierten ThorBurn-Referenz. Der Job muss bereits als fertiges Paket
//! (geswizzelt + Checksum) übergeben werden — das erzeugt `RuidaDriver::compile`.
//!
//! Ohne angeschlossene Maschine nicht real testbar; die Logik ist bewusst
//! schlank und der Kodier-/Paketteil ist im [`crate::protocol`] getestet.

use std::net::UdpSocket;
use std::time::{Duration, Instant};

use crate::protocol::{unswizzle_byte, ACK, ERR, MAGIC, NAK};

const SEND_PORT: u16 = 50200;
const RECV_PORT: u16 = 40200;
const TIMEOUT: Duration = Duration::from_secs(4);
const MAX_RETRIES: usize = 3;

/// Ergebnis des Wartens auf ein Chunk-ACK.
enum AckResult {
    /// Chunk bestätigt.
    Ack,
    /// NAK/ERR — Chunk neu senden.
    Resend,
    /// Keine Antwort bis zur Deadline — Chunk neu senden.
    Timeout,
}

#[derive(Debug)]
pub enum TransportError {
    Io(String),
    Nak,
    Timeout,
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::Io(e) => write!(f, "E/A-Fehler: {e}"),
            TransportError::Nak => write!(f, "Maschine hat abgelehnt (NAK)"),
            TransportError::Timeout => write!(f, "Keine Antwort (Timeout)"),
        }
    }
}

impl From<std::io::Error> for TransportError {
    fn from(e: std::io::Error) -> Self {
        TransportError::Io(e.to_string())
    }
}

/// Verbindung zu einer Ruida-Maschine über UDP.
#[derive(Debug)]
pub struct RuidaTransport {
    socket: UdpSocket,
    target: String,
}

impl RuidaTransport {
    /// Verbindet zur Maschine. UDP ist verbindungslos — erst per [`ping`] prüfen,
    /// ob der Controller antwortet, sonst ist „verbunden" bedeutungslos.
    pub fn connect(ip: &str) -> Result<Self, TransportError> {
        if !Self::ping(ip) {
            return Err(TransportError::Timeout);
        }
        let socket = UdpSocket::bind(format!("0.0.0.0:{RECV_PORT}"))?;
        socket.set_read_timeout(Some(TIMEOUT))?;
        Ok(Self {
            socket,
            target: format!("{ip}:{SEND_PORT}"),
        })
    }

    /// Die IP-Adresse des verbundenen Ziels (ohne Port).
    pub fn target_ip(&self) -> &str {
        self.target.split(':').next().unwrap_or(&self.target)
    }

    /// Sendet einen ROHEN Payload (ungeswizzelt, ohne Checksum) in ≤1024-Byte-
    /// Chunks. **Jeder Chunk wird einzeln geswizzelt + mit eigener Checksum
    /// paketiert** (`build_packet`) und per ACK bestätigt — der Controller
    /// erwartet in jedem UDP-Paket `[2-Byte-Checksum][geswizzelte Nutzdaten]`.
    pub fn send(&self, payload: &[u8]) -> Result<(), TransportError> {
        const CHUNK: usize = 1024;
        if payload.is_empty() {
            return Ok(());
        }
        for chunk in payload.chunks(CHUNK) {
            self.send_chunk(chunk)?;
        }
        Ok(())
    }

    fn send_chunk(&self, chunk: &[u8]) -> Result<(), TransportError> {
        let packet = crate::protocol::build_packet(chunk, MAGIC);
        for _ in 0..MAX_RETRIES {
            self.socket.send_to(&packet, &self.target)?;
            match self.await_ack() {
                AckResult::Ack => return Ok(()),
                // NAK/ERR/Timeout → neuer Sendeversuch.
                AckResult::Resend => continue,
                AckResult::Timeout => continue,
            }
        }
        Err(TransportError::Timeout)
    }

    /// Wartet bis zu `TIMEOUT` (Gesamt-Deadline) auf das echte ACK dieses Chunks.
    /// Verwaiste/verspätete Pakete (z. B. ein Statuspaket aus einem vorigen Query)
    /// werden überlesen — NICHT als „kein ACK" gewertet und NICHT neu gesendet,
    /// sonst ginge der Job-Stream doppelt raus und der Controller verwürfe ihn
    /// (HW-verifizierte Referenz-Logik). Nur NAK/ERR lösen einen Resend aus.
    fn await_ack(&self) -> AckResult {
        let deadline = Instant::now() + TIMEOUT;
        let mut buf = [0u8; 64];
        loop {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return AckResult::Timeout;
            };
            if self
                .socket
                .set_read_timeout(Some(remaining.max(Duration::from_millis(1))))
                .is_err()
            {
                return AckResult::Timeout;
            }
            match self.socket.recv_from(&mut buf) {
                Ok((len, _)) if len > 0 => match unswizzle_byte(buf[0], MAGIC) {
                    ACK => return AckResult::Ack,
                    NAK | ERR => return AckResult::Resend,
                    _ => continue, // verwaiste Antwort überlesen, weiter warten
                },
                Ok(_) => continue,
                Err(_) => return AckResult::Timeout,
            }
        }
    }

    /// Register-Abfrage senden und die passende Antwort lesen. `payload` ist die
    /// rohe (ungeswizzelte) `DA 00 …`-Anfrage; das Ergebnis ist die entswizzelte
    /// `DA 01 …`-Antwort. Versetzte Pakete werden übersprungen.
    pub fn query(&self, payload: &[u8]) -> Result<Vec<u8>, TransportError> {
        let expected_hi = payload.get(2).copied().unwrap_or(0xFF);
        let expected_lo = payload.get(3).copied().unwrap_or(0xFF);
        self.drain();
        self.send(payload)?; // send() paketiert selbst (swizzle + Checksum)

        let mut buf = [0u8; 1024];
        for _ in 0..8 {
            let (len, _) = self
                .socket
                .recv_from(&mut buf)
                .map_err(|_| TransportError::Timeout)?;
            let resp = crate::protocol::unswizzle(&buf[..len], MAGIC);
            if resp.len() >= 4
                && resp[0] == 0xDA
                && resp[1] == 0x01
                && resp[2] == expected_hi
                && resp[3] == expected_lo
            {
                return Ok(resp);
            }
        }
        Err(TransportError::Timeout)
    }

    /// Empfangspuffer leeren (verhindert, dass versetzte Antworten spätere Lesungen
    /// verfälschen).
    pub fn drain(&self) {
        let mut buf = [0u8; 1024];
        let _ = self
            .socket
            .set_read_timeout(Some(Duration::from_millis(50)));
        while self.socket.recv_from(&mut buf).is_ok() {}
        let _ = self.socket.set_read_timeout(Some(TIMEOUT));
    }

    /// Schneller Erreichbarkeits-Ping (300 ms). WICHTIG: Der Controller antwortet
    /// fest an Port 40200 — daher lokal auf 40200 binden (HW-verifiziert).
    pub fn ping(ip: &str) -> bool {
        let Ok(socket) = UdpSocket::bind(format!("0.0.0.0:{RECV_PORT}")) else {
            return false;
        };
        if socket
            .set_read_timeout(Some(Duration::from_millis(300)))
            .is_err()
        {
            return false;
        }
        // Status-Register lesen als Ping-Nutzlast (DA 00 <addr>), als Paket.
        let payload = vec![0xDA, 0x00, 0x04, 0x00];
        let pkt = crate::protocol::build_packet(&payload, MAGIC);
        if socket.send_to(&pkt, format!("{ip}:{SEND_PORT}")).is_err() {
            return false;
        }
        let mut buf = [0u8; 1024];
        match socket.recv_from(&mut buf) {
            Ok((len, from)) => len > 0 && from.ip().to_string() == ip,
            Err(_) => false,
        }
    }
}
