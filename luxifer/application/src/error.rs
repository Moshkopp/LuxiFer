use std::error::Error;
use std::fmt;

/// Stabiler, UI-unabhängiger Fehler aus einem Anwendungsfall.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppError {
    code: &'static str,
    message: String,
    details: Option<String>,
}

impl AppError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    /// Kapselt einen technischen Core-/I/O-Fehler (String) unter einem stabilen
    /// Code und einer nutzerlesbaren Meldung; der Originaltext wandert in die
    /// technischen Details.
    pub fn wrap(code: &'static str, message: impl Into<String>, cause: impl Into<String>) -> Self {
        Self::new(code, message).with_details(cause)
    }

    pub fn code(&self) -> &'static str {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn details(&self) -> Option<&str> {
        self.details.as_deref()
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for AppError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fehler_behaelt_stabilen_code_und_technische_details() {
        let error = AppError::new("project_read", "Projekt konnte nicht geöffnet werden")
            .with_details("permission denied");

        assert_eq!(error.code(), "project_read");
        assert_eq!(error.message(), "Projekt konnte nicht geöffnet werden");
        assert_eq!(error.details(), Some("permission denied"));
        assert_eq!(error.to_string(), error.message());
    }
}
