//! UI-unabhängiger Charon-Verbindungstest (ADR 0012).

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use serde::Deserialize;

use crate::AppError;

const PROTOCOL_VERSION: u32 = 1;
const TIMEOUT: Duration = Duration::from_millis(800);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CharonHandshake {
    pub server: String,
    pub server_version: String,
    pub protocol_version: u32,
    pub instance_id: String,
    pub capabilities: Vec<String>,
}

pub fn test_charon_connection(base_url: &str) -> Result<CharonHandshake, AppError> {
    let endpoint = HttpEndpoint::parse(base_url)?;
    let address = endpoint
        .authority
        .to_socket_addrs()
        .map_err(|error| {
            AppError::wrap(
                "charon_address",
                "Charon-Adresse ist nicht auflösbar.",
                error.to_string(),
            )
        })?
        .next()
        .ok_or_else(|| AppError::new("charon_address", "Charon-Adresse ist nicht auflösbar."))?;
    let mut stream = TcpStream::connect_timeout(&address, TIMEOUT).map_err(|error| {
        AppError::wrap(
            "charon_connect",
            "Charon ist unter der eingestellten Adresse nicht erreichbar.",
            error.to_string(),
        )
    })?;
    stream.set_read_timeout(Some(TIMEOUT)).ok();
    stream.set_write_timeout(Some(TIMEOUT)).ok();
    let request = format!(
        "GET /api/v1/handshake HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        endpoint.authority
    );
    stream.write_all(request.as_bytes()).map_err(|error| {
        AppError::wrap(
            "charon_write",
            "Charon-Anfrage konnte nicht gesendet werden.",
            error.to_string(),
        )
    })?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response).map_err(|error| {
        AppError::wrap(
            "charon_read",
            "Charon-Antwort konnte nicht gelesen werden.",
            error.to_string(),
        )
    })?;
    parse_handshake(&response)
}

struct HttpEndpoint {
    authority: String,
}

impl HttpEndpoint {
    fn parse(raw: &str) -> Result<Self, AppError> {
        let raw = raw.trim().trim_end_matches('/');
        let authority = raw.strip_prefix("http://").ok_or_else(|| {
            AppError::new("charon_url", "Charon-Adresse muss mit http:// beginnen.")
        })?;
        if authority.is_empty() || authority.contains('/') {
            return Err(AppError::new(
                "charon_url",
                "Charon-Adresse muss aus Host und optionalem Port bestehen.",
            ));
        }
        let authority = if authority.contains(':') {
            authority.into()
        } else {
            format!("{authority}:80")
        };
        Ok(Self { authority })
    }
}

fn parse_handshake(response: &[u8]) -> Result<CharonHandshake, AppError> {
    let text = std::str::from_utf8(response).map_err(|error| {
        AppError::wrap(
            "charon_response",
            "Charon hat ungültige Daten geliefert.",
            error.to_string(),
        )
    })?;
    let (headers, body) = text.split_once("\r\n\r\n").ok_or_else(|| {
        AppError::new(
            "charon_response",
            "Charon hat keine gültige HTTP-Antwort geliefert.",
        )
    })?;
    let status = headers.lines().next().unwrap_or_default();
    if !status.contains(" 200 ") {
        return Err(AppError::new(
            "charon_status",
            format!("Charon antwortet mit {status}."),
        ));
    }
    let handshake: CharonHandshake = serde_json::from_str(body).map_err(|error| {
        AppError::wrap(
            "charon_json",
            "Charon-Handshake ist ungültig.",
            error.to_string(),
        )
    })?;
    if handshake.server != "charon" || handshake.protocol_version != PROTOCOL_VERSION {
        return Err(AppError::new(
            "charon_protocol",
            format!(
                "Charon-Protokoll ist nicht kompatibel (erwartet {PROTOCOL_VERSION}, erhalten {}).",
                handshake.protocol_version
            ),
        ));
    }
    Ok(handshake)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_akzeptiert_gueltigen_handshake() {
        let response = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"server\":\"charon\",\"server_version\":\"0.1.0\",\"protocol_version\":1,\"instance_id\":\"local-test\",\"capabilities\":[\"health\",\"handshake\"]}";
        let handshake = parse_handshake(response).unwrap();
        assert_eq!(handshake.server_version, "0.1.0");
        assert!(handshake.capabilities.contains(&"handshake".into()));
    }

    #[test]
    fn url_verlangt_http_und_reines_ziel() {
        assert!(HttpEndpoint::parse("https://localhost:3737").is_err());
        assert!(HttpEndpoint::parse("http://localhost:3737/pfad").is_err());
        assert_eq!(
            HttpEndpoint::parse("http://localhost:3737/")
                .unwrap()
                .authority,
            "localhost:3737"
        );
    }
}
