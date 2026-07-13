//! System-Font-Auflistung fürs Text-Werkzeug (TTF/OTF). Eigenständig im Frontend,
//! da der Core UI-frei ist; die eigentliche Glyph-Umsetzung macht der Core
//! (`text_to_contours`).

use std::path::PathBuf;

pub struct FontEntry {
    pub name: String,
    pub path: PathBuf,
}

/// Scannt die üblichen Font-Verzeichnisse nach TTF/OTF (nicht rekursiv tief,
/// aber inkl. Unterordner). Dedupliziert nach Dateiname.
pub fn list_fonts() -> Vec<FontEntry> {
    let home = std::env::var("HOME").unwrap_or_default();
    let dirs = [
        format!("{home}/.local/share/luxifer/Fonts"),
        "/usr/share/fonts".to_string(),
        "/usr/local/share/fonts".to_string(),
        format!("{home}/.fonts"),
        format!("{home}/.local/share/fonts"),
    ];
    let mut out: Vec<FontEntry> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    if let Ok(assets) = luxifer_core::list_assets(&luxifer_core::assets_dir()) {
        for meta in assets
            .into_iter()
            .filter(|meta| meta.kind == luxifer_core::AssetKind::Font)
        {
            if let Some(path) = luxifer_core::asset_path(&luxifer_core::assets_dir(), &meta.id) {
                let name = std::path::Path::new(&meta.original_name)
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or(&meta.original_name)
                    .to_string();
                if seen.insert(name.clone()) {
                    out.push(FontEntry { name, path });
                }
            }
        }
    }
    for dir in dirs {
        let mut stack = vec![PathBuf::from(dir)];
        while let Some(d) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else {
                continue;
            };
            for entry in rd.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    stack.push(p);
                    continue;
                }
                let ext = p
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if ext == "ttf" || ext == "otf" {
                    let name = p
                        .file_stem()
                        .and_then(|n| n.to_str())
                        .unwrap_or("?")
                        .to_string();
                    if seen.insert(name.clone()) {
                        out.push(FontEntry { name, path: p });
                    }
                }
            }
        }
    }
    out.sort_by_key(|e| e.name.to_lowercase());
    out
}
