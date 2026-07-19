use std::path::PathBuf;

fn main() {
    // env::var statt env!: Der Pfad darf nicht zur Compile-Zeit ins
    // Build-Skript eingebrannt werden, sonst bricht ein verschobenes Repo.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR fehlt");
    let path = PathBuf::from(manifest_dir).join("../../branding.conf");
    println!("cargo:rerun-if-changed={}", path.display());
    let content = std::fs::read_to_string(&path).expect("branding.conf muss lesbar sein");
    for line in content.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .expect("branding.conf erwartet SCHLUESSEL=WERT");
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .unwrap_or(value);
        match key {
            "PRODUCT_NAME" | "STUDIO_NAME" | "HUB_NAME" | "HUB_PROTOCOL_ID" | "APP_ID"
            | "DATA_DIR_NAME" => {
                println!("cargo:rustc-env={key}={value}");
            }
            _ => panic!("Unbekannter Branding-Schlüssel: {key}"),
        }
    }
}
