//! UDP-Transport für den Ruida RDC6445G. Echte I/O.
//!
//! Ports: senden → 50200, empfangen ← 40200 (lokal binden). Nach der
//! HW-verifizierten ThorBurn-Referenz. Der Job muss bereits als fertiges Paket
//! (geswizzelt + Checksum) übergeben werden — das erzeugt `RuidaDriver::compile`.
//!
//! Ohne angeschlossene Maschine nicht real testbar; die Logik ist bewusst
//! schlank und der Kodier-/Paketteil ist im [`crate::protocol`] getestet.

use std::net::UdpSocket;
use std::time::Duration;

use crate::protocol::{unswizzle_byte, ACK, MAGIC, NAK};

const SEND_PORT: u16 = 50200;
const RECV_PORT: u16 = 40200;
const TIMEOUT: Duration = Duration::from_secs(4);
const MAX_RETRIES: usize = 3;

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

    /// Sendet ein bereits fertiges Paket (geswizzelt + Checksum) in ≤1024-Byte-
    /// Chunks; jeder Chunk wird per ACK bestätigt.
    pub fn send(&self, packet: &[u8]) -> Result<(), TransportError> {
        const CHUNK: usize = 1024;
        for chunk in packet.chunks(CHUNK) {
            self.send_chunk(chunk)?;
        }
        Ok(())
    }

    fn send_chunk(&self, chunk: &[u8]) -> Result<(), TransportError> {
        for attempt in 0..MAX_RETRIES {
            self.socket.send_to(chunk, &self.target)?;
            match self.recv_ack() {
                Ok(()) => return Ok(()),
                Err(TransportError::Nak) if attempt + 1 < MAX_RETRIES => continue,
                Err(e) => return Err(e),
            }
        }
        Err(TransportError::Nak)
    }

    fn recv_ack(&self) -> Result<(), TransportError> {
        let mut buf = [0u8; 64];
        loop {
            self.socket
                .recv_from(&mut buf)
                .map_err(|_| TransportError::Timeout)?;
            match unswizzle_byte(buf[0], MAGIC) {
                ACK => return Ok(()),
                NAK => return Err(TransportError::Nak),
                _ => continue, // verwaiste Antwort überlesen
            }
        }
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
