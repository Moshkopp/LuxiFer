//! Gemeinsame Testumgebung: `STUDIO_DATA_DIR` ist prozessglobal, deshalb
//! teilen sich ALLE Tests, die das Datenverzeichnis anfassen (Projekt, Assets,
//! Laser-Registry), einen Lock und dieses Helferlein. Ein zweiter Lock in einem
//! anderen Testmodul würde nicht synchronisieren — Tests liefen dann auf dem
//! Verzeichnis des jeweils anderen.

use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Setzt ein frisches Temp-Datenverzeichnis und gibt den Lock-Guard zurück.
/// Der Guard muss für die Dauer des Tests leben.
pub(crate) fn with_temp_dir(tag: &str) -> std::sync::MutexGuard<'static, ()> {
    let guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("studio_app_test_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    // SAFETY: Zugriff ist über ENV_LOCK serialisiert.
    unsafe {
        std::env::set_var("STUDIO_DATA_DIR", &dir);
    }
    guard
}
