//! System-Font-Auflistung fürs Text-Werkzeug (TTF/OTF). Eigenständig im Frontend,
//! da der Core UI-frei ist; die eigentliche Glyph-Umsetzung macht der Core
//! (`text_to_contours`).
//!
//! Fonts werden zu **Familien** gruppiert (name-Tabelle: Familie + Schnitt),
//! nicht mehr über den Dateinamen-Stem dedupliziert — Bold/Italic sind Schnitte
//! derselben Familie statt eigener (oder verschluckter) Einträge. Der Scan
//! liest dafür jede Font-Datei einmal (nur Header-Parsing via `ttf-parser`);
//! das passiert einmalig beim ersten Öffnen des Text-Dialogs und wird in der
//! App gecacht.

use std::path::PathBuf;

/// Ein Schnitt (Regular/Bold/Italic/…) einer Font-Familie.
pub struct FontFace {
    /// Schnittname aus der name-Tabelle (z. B. "Bold Italic").
    pub style: String,
    pub path: PathBuf,
}

/// Eine Font-Familie mit ihren Schnitten (mindestens einer).
pub struct FontFamily {
    pub name: String,
    pub faces: Vec<FontFace>,
    /// Vom Nutzer importiert (Asset-Katalog / Studio-Fonts-Ordner) — steht in
    /// der Auswahl-Liste vor den System-Fonts.
    pub imported: bool,
}

impl FontFamily {
    /// Index des Standard-Schnitts ("Regular", sonst erster).
    pub fn default_face(&self) -> usize {
        self.faces
            .iter()
            .position(|f| f.style.eq_ignore_ascii_case("Regular"))
            .unwrap_or(0)
    }
}

/// Familie + Schnitt aus der name-Tabelle; typographische Namen (IDs 16/17)
/// haben Vorrang vor den Legacy-Namen (IDs 1/2), weil letztere Schnitte wie
/// "Light" oft in den Familiennamen packen.
fn face_names(data: &[u8]) -> Option<(String, String)> {
    use ttf_parser::name_id;
    let face = ttf_parser::Face::parse(data, 0).ok()?;
    let pick = |id: u16| {
        face.names()
            .into_iter()
            .filter(|n| n.name_id == id)
            .find_map(|n| n.to_string())
    };
    let family = pick(name_id::TYPOGRAPHIC_FAMILY).or_else(|| pick(name_id::FAMILY))?;
    let style = pick(name_id::TYPOGRAPHIC_SUBFAMILY)
        .or_else(|| pick(name_id::SUBFAMILY))
        .unwrap_or_else(|| "Regular".into());
    Some((family, style))
}

/// Sortierschlüssel für Schnitte: gängige Schnitte in gewohnter Reihenfolge
/// vor den exotischen (die alphabetisch folgen).
fn style_rank(style: &str) -> u8 {
    match style.to_ascii_lowercase().as_str() {
        "regular" | "book" | "roman" => 0,
        "italic" | "oblique" => 1,
        "medium" => 2,
        "bold" => 3,
        "bold italic" | "bold oblique" => 4,
        _ => 5,
    }
}

/// Scannt Asset-Katalog und übliche Font-Verzeichnisse nach TTF/OTF und
/// gruppiert nach Familie. Der Katalog kommt zuerst, damit importierte Fonts
/// auch ohne Systeminstallation gewinnen; Duplikate (gleiche Familie+Schnitt)
/// werden verworfen.
pub fn list_font_families() -> Vec<FontFamily> {
    let home = std::env::var("HOME").unwrap_or_default();
    // (Verzeichnis, gilt als importiert): Katalog + Studio-Fonts-Ordner sind
    // Nutzer-Bestand, der Rest System.
    let dirs = [
        (format!("{home}/.local/share/studio/Fonts"), true),
        ("/usr/share/fonts".to_string(), false),
        ("/usr/local/share/fonts".to_string(), false),
        (format!("{home}/.fonts"), false),
        (format!("{home}/.local/share/fonts"), false),
    ];

    let mut paths: Vec<(PathBuf, bool)> = Vec::new();
    if let Ok(assets) = studio_application::AssetService::list_all() {
        for meta in assets
            .into_iter()
            .filter(|meta| meta.kind == studio_core::AssetKind::Font)
        {
            if let Some(path) = studio_application::AssetService::path(&meta.id) {
                paths.push((path, true));
            }
        }
    }
    for (dir, imported) in dirs {
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
                    paths.push((p, imported));
                }
            }
        }
    }

    let mut families: Vec<FontFamily> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (path, imported) in paths {
        let Ok(data) = std::fs::read(&path) else {
            continue;
        };
        // Fallback für Fonts ohne lesbare name-Tabelle: Dateinamen-Stem.
        let (family, style) = face_names(&data).unwrap_or_else(|| {
            let stem = path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string();
            (stem, "Regular".into())
        });
        if !seen.insert((family.to_lowercase(), style.to_lowercase())) {
            continue;
        }
        let face = FontFace { style, path };
        match families.iter_mut().find(|f| f.name == family) {
            Some(fam) => {
                fam.faces.push(face);
                fam.imported |= imported;
            }
            None => families.push(FontFamily {
                name: family,
                faces: vec![face],
                imported,
            }),
        }
    }

    for fam in &mut families {
        fam.faces.sort_by(|a, b| {
            (style_rank(&a.style), &a.style).cmp(&(style_rank(&b.style), &b.style))
        });
    }
    // Importierte zuerst, dann alphabetisch — die Dialog-Liste verlässt sich
    // auf diese Ordnung (eigener Block oben).
    families.sort_by_key(|f| (!f.imported, f.name.to_lowercase()));
    families
}
