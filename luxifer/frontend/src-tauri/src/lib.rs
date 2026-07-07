//! LuxiFer Tauri-Backend. Hält den `AppState` des Cores und stellt Commands
//! bereit. Das Frontend zeichnet nur — die gesamte Fachlogik bleibt im Core.

use std::sync::Mutex;

use luxifer_core::{
    delete_project, list_projects, projects_dir, rename_project, AppState, Geo, Layer, ProjectFile,
    ProjectInfo, PolyShape, Shape, ShapeInfo, Tab, UiSettings, VersionInfo,
};
use serde::Serialize;
use tauri::{Manager, State};

/// Das aktuell geöffnete Projekt (Metadaten, ohne die Geometrie — die lebt im
/// `AppState`). `None`, solange das Projekt noch namenlos ist.
#[derive(Default)]
struct CurrentProject {
    file: Option<ProjectFile>,
}

/// Geteilter Zustand über alle Commands.
struct AppData {
    state: Mutex<AppState>,
    current: Mutex<CurrentProject>,
}

/// Metadaten des offenen Projekts fürs Frontend (Kopf im Designer/Toast).
#[derive(Serialize, Clone)]
struct ProjectMeta {
    name: String,
    description: String,
    tags: Vec<String>,
}

/// Schlanke Sicht auf den Zustand fürs Frontend (ohne Undo-Stacks).
#[derive(Serialize)]
struct Scene {
    layers: Vec<Layer>,
    shapes: Vec<Shape>,
    selected: Vec<usize>,
    bed_w_mm: f64,
    bed_h_mm: f64,
    /// Ungespeicherte Änderungen? Steuert den Unsaved-Guard im Frontend.
    dirty: bool,
    /// Offenes Projekt (Name/Beschreibung/Tags) oder `None`, wenn namenlos.
    project: Option<ProjectMeta>,
}

impl Scene {
    fn build(s: &AppState, cur: &CurrentProject) -> Self {
        Scene {
            layers: s.layers.clone(),
            shapes: s.shapes.clone(),
            selected: s.selected.clone(),
            bed_w_mm: s.bed_w_mm,
            bed_h_mm: s.bed_h_mm,
            dirty: s.dirty,
            project: cur.file.as_ref().map(|f| ProjectMeta {
                name: f.name.clone(),
                description: f.description.clone(),
                tags: f.tags.clone(),
            }),
        }
    }
}

// Sperrt beide Zustände und baut die Scene (der übliche Rückgabewert).
fn scene(data: &State<AppData>) -> Scene {
    let s = data.state.lock().unwrap();
    let cur = data.current.lock().unwrap();
    Scene::build(&s, &cur)
}

// Baut die Scene aus einem bereits gelockten AppState + dem Projektkontext.
// Ersetzt das frühere `Scene::from_state(&s)` in den einzelnen Commands.
fn scene_with(s: &AppState, data: &State<AppData>) -> Scene {
    let cur = data.current.lock().unwrap();
    Scene::build(s, &cur)
}

#[tauri::command]
fn get_scene(data: State<AppData>) -> Scene {
    scene(&data)
}

#[tauri::command]
fn swatch_colors() -> Vec<[u8; 3]> {
    luxifer_core::SWATCH_COLORS.to_vec()
}

#[tauri::command]
fn add_rect(data: State<AppData>, x: f64, y: f64, w: f64, h: f64) -> Scene {
    let mut s = data.state.lock().unwrap();
    s.add_shape(Geo::Rect { x, y, w, h });
    scene_with(&s, &data)
}

#[tauri::command]
fn add_ellipse(data: State<AppData>, cx: f64, cy: f64, rx: f64, ry: f64) -> Scene {
    let mut s = data.state.lock().unwrap();
    s.add_shape(Geo::Ellipse { cx, cy, rx, ry });
    scene_with(&s, &data)
}

/// Fügt eine offene 2-Punkt-Linie als Polyline hinzu.
#[tauri::command]
fn add_line(data: State<AppData>, x1: f64, y1: f64, x2: f64, y2: f64) -> Scene {
    let mut s = data.state.lock().unwrap();
    s.add_shape(Geo::Polyline {
        pts: vec![(x1, y1), (x2, y2)],
        closed: false,
    });
    scene_with(&s, &data)
}

/// Fügt eine Polylinie aus den gelieferten Punkten hinzu. `closed` schließt die
/// Kontur (letzter → erster Punkt). Wird ignoriert, wenn < 2 Punkte kommen.
#[tauri::command]
fn add_polyline(data: State<AppData>, pts: Vec<(f64, f64)>, closed: bool) -> Scene {
    let mut s = data.state.lock().unwrap();
    if pts.len() >= 2 {
        s.add_shape(Geo::Polyline { pts, closed });
    }
    scene_with(&s, &data)
}

/// Katalog der parametrischen Formen für die Galerie im Werkzeug-Panel.
/// Datengetrieben: eine neue Form im Core erscheint hier automatisch.
#[tauri::command]
fn shape_catalog() -> Vec<ShapeInfo> {
    PolyShape::catalog()
}

/// Fügt eine parametrische Form als geschlossene Polylinie hinzu.
/// `shape` = stabiler Bezeichner aus dem Katalog (z. B. "hex"); unbekannte
/// Bezeichner werden ignoriert (Zustand bleibt unverändert).
#[tauri::command]
fn add_polygon(data: State<AppData>, shape: String, cx: f64, cy: f64, r: f64, rot: f64) -> Scene {
    let mut s = data.state.lock().unwrap();
    if let Some(kind) = PolyShape::from_id(&shape) {
        let pts = kind.points(cx, cy, r, rot);
        if pts.len() >= 3 {
            s.add_shape(Geo::Polyline { pts, closed: true });
        }
    }
    scene_with(&s, &data)
}

#[tauri::command]
fn activate_color(data: State<AppData>, color: [u8; 3]) -> Scene {
    let mut s = data.state.lock().unwrap();
    s.activate_color(color);
    scene_with(&s, &data)
}

#[tauri::command]
fn select_at(data: State<AppData>, x: f64, y: f64, tol: f64, additive: bool) -> Scene {
    let mut s = data.state.lock().unwrap();
    match s.hit_test(x, y, tol) {
        Some(idx) => {
            if additive {
                // Toggle: enthalten → raus, sonst rein.
                if let Some(pos) = s.selected.iter().position(|&i| i == idx) {
                    s.selected.remove(pos);
                } else {
                    s.selected.push(idx);
                }
            } else if !s.selected.contains(&idx) {
                s.selected = vec![idx];
            }
        }
        None => {
            if !additive {
                s.selected.clear();
            }
        }
    }
    scene_with(&s, &data)
}

/// Marquee-Auswahl: alle Shapes, deren BBox vollständig im Rechteck liegt.
#[tauri::command]
fn select_rect(data: State<AppData>, x1: f64, y1: f64, x2: f64, y2: f64) -> Scene {
    let mut s = data.state.lock().unwrap();
    s.select_in_rect(x1, y1, x2, y2);
    scene_with(&s, &data)
}

/// Verschiebt die Auswahl um ein Gesamt-Delta (ein Undo-Punkt pro Geste).
#[tauri::command]
fn move_selected(data: State<AppData>, dx: f64, dy: f64) -> Scene {
    let mut s = data.state.lock().unwrap();
    if dx != 0.0 || dy != 0.0 {
        s.push_undo();
        s.translate_selected(dx, dy);
    }
    scene_with(&s, &data)
}

/// Skaliert die Auswahl von der Start-Gruppenbox auf die Zielbox (ein Undo-Punkt).
#[allow(clippy::too_many_arguments)]
#[tauri::command]
fn scale_selected(
    data: State<AppData>,
    sx: f64,
    sy: f64,
    sw: f64,
    sh: f64,
    tx: f64,
    ty: f64,
    tw: f64,
    th: f64,
) -> Scene {
    use luxifer_core::BBox;
    let mut s = data.state.lock().unwrap();
    s.push_undo();
    s.scale_selection_to(BBox::new(sx, sy, sw, sh), BBox::new(tx, ty, tw, th));
    scene_with(&s, &data)
}

#[tauri::command]
fn align(data: State<AppData>, kind: String) -> Scene {
    use luxifer_core::Align;
    let mut s = data.state.lock().unwrap();
    let k = match kind.as_str() {
        "left" => Align::Left,
        "hcenter" => Align::HCenter,
        "right" => Align::Right,
        "top" => Align::Top,
        "vcenter" => Align::VCenter,
        "bottom" => Align::Bottom,
        _ => return scene_with(&s, &data),
    };
    s.align_selection(k);
    scene_with(&s, &data)
}

#[tauri::command]
fn distribute(data: State<AppData>, kind: String) -> Scene {
    use luxifer_core::Distribute;
    let mut s = data.state.lock().unwrap();
    let k = match kind.as_str() {
        "h" => Distribute::Horizontal,
        "v" => Distribute::Vertical,
        _ => return scene_with(&s, &data),
    };
    s.distribute_selection(k);
    scene_with(&s, &data)
}

/// Spiegelt die Auswahl an der Mittelachse ihrer gemeinsamen BBox.
/// `axis`: "h" = horizontal spiegeln (links↔rechts, vertikale Achse),
/// "v" = vertikal spiegeln (oben↔unten, horizontale Achse).
#[tauri::command]
fn mirror(data: State<AppData>, axis: String) -> Scene {
    use luxifer_core::Axis;
    let mut s = data.state.lock().unwrap();
    let a = match axis.as_str() {
        "h" => Axis::Vertical,
        "v" => Axis::Horizontal,
        _ => return scene_with(&s, &data),
    };
    s.mirror_selection(a);
    scene_with(&s, &data)
}

#[tauri::command]
fn clear_selection(data: State<AppData>) -> Scene {
    let mut s = data.state.lock().unwrap();
    s.selected.clear();
    scene_with(&s, &data)
}

#[tauri::command]
fn delete_selected(data: State<AppData>) -> Scene {
    let mut s = data.state.lock().unwrap();
    s.delete_selected();
    scene_with(&s, &data)
}

/// Vom Frontend gelieferte Layer-Parameter (Doppelklick-Dialog).
#[derive(serde::Deserialize)]
struct LayerParams {
    name: String,
    mode: String,
    speed_mm_s: f64,
    power_pct: f64,
    min_power_pct: f64,
    passes: u32,
    air_assist: bool,
    line_step_mm: f64,
    dpi: f64,
}

/// Setzt die Parameter eines Layers (ein Undo-Punkt).
#[tauri::command]
fn set_layer_params(data: State<AppData>, index: usize, p: LayerParams) -> Scene {
    use luxifer_core::LayerMode;
    let mut s = data.state.lock().unwrap();
    if index < s.layers.len() {
        s.push_undo();
        let l = &mut s.layers[index];
        l.name = p.name;
        l.mode = match p.mode.as_str() {
            "Fill" => LayerMode::Fill,
            "Raster" => LayerMode::Raster,
            _ => LayerMode::Cut,
        };
        l.speed_mm_s = p.speed_mm_s;
        l.power_pct = p.power_pct;
        l.min_power_pct = p.min_power_pct;
        l.passes = p.passes;
        l.air_assist = p.air_assist;
        l.line_step_mm = p.line_step_mm;
        l.dpi = p.dpi;
    }
    scene_with(&s, &data)
}

/// Schalter eines Layers umschalten (Anzeige, Brennen, Luft, Sperre).
#[tauri::command]
fn toggle_layer(data: State<AppData>, index: usize, field: String) -> Scene {
    let mut s = data.state.lock().unwrap();
    if let Some(l) = s.layers.get_mut(index) {
        match field.as_str() {
            "visible" => l.visible = !l.visible,          // Objekte anzeigen
            "enabled" => l.enabled = !l.enabled,          // im Job mitbrennen
            "air_assist" => l.air_assist = !l.air_assist, // Luftunterstützung
            "locked" => l.locked = !l.locked,             // Editiersperre
            _ => {}
        }
    }
    scene_with(&s, &data)
}

/// Erzeugt aus dem aktuellen Zustand einen G-Code-Job (GRBL-Treiber).
/// Gibt den G-Code als Text zurück (oder einen Fehler bei leerem Job).
#[tauri::command]
fn generate_gcode(data: State<AppData>) -> Result<String, String> {
    use luxifer_core::{JobPlan, MachineDriver};
    use luxifer_driver_grbl::GrblDriver;
    let s = data.state.lock().unwrap();
    let plan = JobPlan::from_shapes(&s.shapes, &s.layers);
    let bytes = GrblDriver::default().compile(&plan, &s.layers)?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

/// Prüft per UDP-Ping, ob eine Ruida-Maschine unter `ip` antwortet.
#[tauri::command]
fn ruida_ping(ip: String) -> bool {
    luxifer_driver_ruida::RuidaTransport::ping(&ip)
}

/// Kompiliert den aktuellen Zustand als Ruida-Job und sendet ihn per UDP.
#[tauri::command]
fn ruida_send(data: State<AppData>, ip: String) -> Result<String, String> {
    use luxifer_core::{JobPlan, MachineDriver};
    use luxifer_driver_ruida::{RuidaDriver, RuidaTransport};
    let plan = {
        let s = data.state.lock().unwrap();
        JobPlan::from_shapes(&s.shapes, &s.layers)
    };
    let packet = RuidaDriver.compile(&plan, &[])?;
    let t = RuidaTransport::connect(&ip).map_err(|e| e.to_string())?;
    t.send(&packet).map_err(|e| e.to_string())?;
    Ok(format!("Job gesendet ({} Byte).", packet.len()))
}

/// Lädt die GUI-Settings (Panel-Layouts, Theming, Arbeitsplatz) — ADR 0002.
/// Fehlt die Datei, kommt der Default zurück; die GUI startet immer.
#[tauri::command]
fn get_ui_settings() -> UiSettings {
    UiSettings::load()
}

/// Speichert die vom Frontend gelieferten GUI-Settings lokal als JSON.
/// Werte werden vor dem Schreiben geklemmt/aufgeräumt (sanitize).
#[tauri::command]
fn save_ui_settings(mut settings: UiSettings) -> Result<UiSettings, String> {
    settings.sanitize();
    settings.save()?;
    Ok(settings)
}

/// Setzt einen Reiter auf sein Standard-Layout zurück (ADR §2), speichert und
/// gibt die aktualisierten Settings zurück. Andere Reiter bleiben unberührt.
#[tauri::command]
fn reset_ui_tab(tab: Tab) -> Result<UiSettings, String> {
    let mut settings = UiSettings::load();
    settings.reset_tab(tab);
    settings.save()?;
    Ok(settings)
}

// ---- Projektverwaltung (ADR 0003) -----------------------------------------

/// Volle Detailansicht eines Projekts (rechte Seite im Browser).
#[derive(Serialize)]
struct ProjectDetail {
    name: String,
    description: String,
    tags: Vec<String>,
    created_at: String,
    modified_at: String,
    versions: Vec<VersionInfo>,
    asset_refs: Vec<String>,
}

/// Neues, leeres Projekt: Zeichenfläche leeren, Projektkontext zurücksetzen.
#[tauri::command]
fn new_project(data: State<AppData>) -> Scene {
    {
        let mut s = data.state.lock().unwrap();
        *s = AppState::new();
    }
    {
        let mut cur = data.current.lock().unwrap();
        cur.file = None;
    }
    scene(&data)
}

/// Speichert das Projekt. Ist noch keins offen (namenlos), wird mit den
/// gelieferten Metadaten ein neues angelegt; sonst wird der Arbeitsstand des
/// offenen Projekts überschrieben. `thumb_png` sind fertige PNG-Bytes (Frontend).
#[tauri::command]
fn save_project(
    data: State<AppData>,
    name: String,
    description: String,
    tags: Vec<String>,
    thumb_png: Vec<u8>,
) -> Result<Scene, String> {
    let dir = projects_dir();
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Projektname darf nicht leer sein.".into());
    }
    let mut s = data.state.lock().unwrap();
    let mut cur = data.current.lock().unwrap();

    // Bestehendes Projekt aktualisieren oder neues anlegen.
    let pf = match cur.file.take() {
        Some(mut existing) => {
            existing.name = name.clone();
            existing.description = description;
            existing.tags = tags;
            existing.update_from_state(&s);
            existing
        }
        None => {
            let mut pf = ProjectFile::from_state(&s, &name, tags);
            pf.description = description;
            pf
        }
    };
    pf.save_to_dir(&dir)?;
    // Thumbnail des aktuellen Stands neben die Projektdatei legen (für die Liste).
    if !thumb_png.is_empty() {
        let _ = std::fs::write(dir.join(&name).join("thumbnail.png"), &thumb_png);
    }
    s.mark_saved();
    cur.file = Some(pf);
    remember_last_project(&name);
    Ok(Scene::build(&s, &cur))
}

/// Hält den aktuellen Stand als **neue Version** fest (Shift+Strg+S). Verlangt
/// ein bereits gespeichertes (benanntes) Projekt.
#[tauri::command]
fn save_version(data: State<AppData>, note: String, thumb_png: Vec<u8>) -> Result<Scene, String> {
    let dir = projects_dir();
    let mut s = data.state.lock().unwrap();
    let mut cur = data.current.lock().unwrap();
    let Some(pf) = cur.file.as_mut() else {
        return Err("Bitte zuerst das Projekt speichern (Strg+S).".into());
    };
    // Arbeitsstand ins ProjectFile übernehmen, dann Version anlegen.
    pf.update_from_state(&s);
    let name = pf.name.clone();
    pf.add_version(&dir, note, &thumb_png)?;
    // Hauptvorschau (Arbeitsstand) mit aktualisieren: „Hauptversion = letzter
    // Stand", egal ob per Strg+S oder Shift+Strg+S gespeichert.
    if !thumb_png.is_empty() {
        let _ = std::fs::write(dir.join(&name).join("thumbnail.png"), &thumb_png);
    }
    s.mark_saved();
    Ok(Scene::build(&s, &cur))
}

/// Öffnet ein Projekt (lädt den aktuellen Arbeitsstand in den AppState).
#[tauri::command]
fn open_project(data: State<AppData>, name: String) -> Result<Scene, String> {
    let dir = projects_dir();
    let pf = ProjectFile::load_by_name(&dir, &name)?;
    {
        let mut s = data.state.lock().unwrap();
        *s = pf.clone().into_state();
    }
    {
        let mut cur = data.current.lock().unwrap();
        cur.file = Some(pf);
    }
    remember_last_project(&name);
    Ok(scene(&data))
}

/// Lädt einen Versions-Snapshot und **befördert ihn zum aktuellen Stand**:
/// Die Version wird der neue Arbeitsstand UND der neue gespeicherte Hauptstand
/// (`projekt.luxi` + `thumbnail.png`). So bleiben Vorschau (= letzter Speicher-
/// stand) und tatsächlich geladener Stand immer synchron (ADR 0003 Regel:
/// „Hauptversion = letzter gespeicherter Stand"). Die Versionshistorie bleibt.
#[tauri::command]
fn open_version(data: State<AppData>, name: String, version_id: String) -> Result<Scene, String> {
    let dir = projects_dir();
    let snap = ProjectFile::load_version(&dir, &name, &version_id)?;
    // Aktuelle Metadaten (Name/Beschreibung/Tags/Historie) behalten, nur die
    // Geometrie durch den Snapshot ersetzen.
    let mut current = ProjectFile::load_by_name(&dir, &name)?;
    current.bed_w_mm = snap.bed_w_mm;
    current.bed_h_mm = snap.bed_h_mm;
    current.layers = snap.layers.clone();
    current.shapes = snap.shapes.clone();
    current.modified_at = luxifer_core::project::now_iso8601();
    // Als neuen Hauptstand schreiben und das Versions-Thumbnail als Hauptvorschau
    // übernehmen, damit die grosse Vorschau exakt diesen Stand zeigt.
    current.save_to_dir(&dir)?;
    if let Some(vpng) = luxifer_core::version_thumb_path(&dir, &name, &version_id) {
        let _ = std::fs::copy(&vpng, dir.join(&name).join("thumbnail.png"));
    }
    {
        let mut s = data.state.lock().unwrap();
        *s = snap.into_state();
    }
    {
        let mut cur = data.current.lock().unwrap();
        cur.file = Some(current);
    }
    remember_last_project(&name);
    Ok(scene(&data))
}

/// Liste aller Projekte (linke Seite im Browser).
#[tauri::command]
fn project_list() -> Vec<ProjectInfo> {
    list_projects(&projects_dir())
}

/// Volle Details eines Projekts (rechte Seite im Browser).
#[tauri::command]
fn project_detail(name: String) -> Result<ProjectDetail, String> {
    let pf = ProjectFile::load_by_name(&projects_dir(), &name)?;
    Ok(ProjectDetail {
        name: pf.name,
        description: pf.description,
        tags: pf.tags,
        created_at: pf.created_at,
        modified_at: pf.modified_at,
        versions: pf.versions,
        asset_refs: pf.asset_refs,
    })
}

/// Liefert das Thumbnail des Projekts (aktueller Stand, `thumbnail.png`) als
/// Data-URL, oder `None`, wenn keins existiert.
#[tauri::command]
fn project_thumb(name: String) -> Option<String> {
    let path = projects_dir().join(&name).join("thumbnail.png");
    read_png_data_url(&path)
}

/// Liefert das Thumbnail einer bestimmten Version als Data-URL (oder `None`).
#[tauri::command]
fn version_thumb(name: String, version_id: String) -> Option<String> {
    let p = luxifer_core::version_thumb_path(&projects_dir(), &name, &version_id)?;
    read_png_data_url(&p)
}

/// Liest eine PNG-Datei und kodiert sie als `data:image/png;base64,…`-URL.
fn read_png_data_url(path: &std::path::Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(format!("data:image/png;base64,{}", base64_encode(&bytes)))
}

/// Minimale Base64-Kodierung (Standard-Alphabet, mit Padding), ohne Fremd-Crate.
fn base64_encode(data: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | (b[2] as u32);
        out.push(A[((n >> 18) & 63) as usize] as char);
        out.push(A[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            A[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            A[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// Löscht ein Projekt samt Versionen. War es das offene Projekt, wird der
/// Projektkontext zurückgesetzt (der Arbeitsstand bleibt zum Weiterarbeiten).
#[tauri::command]
fn project_delete(data: State<AppData>, name: String) -> Result<(), String> {
    delete_project(&projects_dir(), &name)?;
    let mut cur = data.current.lock().unwrap();
    if cur.file.as_ref().is_some_and(|f| f.name == name) {
        cur.file = None;
    }
    Ok(())
}

/// Benennt ein Projekt um (Identität/`id` bleibt). Aktualisiert den offenen
/// Projektkontext, falls es das offene Projekt war.
#[tauri::command]
fn project_rename(data: State<AppData>, old_name: String, new_name: String) -> Result<(), String> {
    let dir = projects_dir();
    rename_project(&dir, &old_name, &new_name)?;
    let mut cur = data.current.lock().unwrap();
    if let Some(f) = cur.file.as_mut() {
        if f.name == old_name {
            f.name = new_name.clone();
            remember_last_project(&new_name);
        }
    }
    Ok(())
}

/// Exportiert die Projektdatei nach `ziel` (einfacher Datei-Export der
/// `projekt.luxi`). Ordner-/ZIP-Export kann später folgen.
#[tauri::command]
fn project_export(name: String, ziel: String) -> Result<(), String> {
    let src = projects_dir().join(&name).join("projekt.luxi");
    std::fs::copy(&src, &ziel).map_err(|e| e.to_string())?;
    Ok(())
}

/// Merkt sich das zuletzt geöffnete/gespeicherte Projekt in den GUI-Settings
/// (für den Start-Toast). Fehler werden geschluckt — rein kosmetisch.
fn remember_last_project(name: &str) {
    let mut settings = UiSettings::load();
    if settings.last_project != name {
        settings.last_project = name.to_string();
        let _ = settings.save();
    }
}

#[tauri::command]
fn undo(data: State<AppData>) -> Scene {
    let mut s = data.state.lock().unwrap();
    s.undo();
    scene_with(&s, &data)
}

#[tauri::command]
fn redo(data: State<AppData>) -> Scene {
    let mut s = data.state.lock().unwrap();
    s.redo();
    scene_with(&s, &data)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(AppData {
                state: Mutex::new(AppState::new()),
                current: Mutex::new(CurrentProject::default()),
            });
            // Fenster-/Taskleisten-Icon zur Laufzeit setzen (greift auch im
            // Dev-Modus, wo das gebündelte Icon sonst nicht verwendet wird).
            if let (Some(win), Some(icon)) =
                (app.get_webview_window("main"), app.default_window_icon())
            {
                let _ = win.set_icon(icon.clone());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_scene,
            swatch_colors,
            add_rect,
            add_ellipse,
            add_line,
            add_polyline,
            shape_catalog,
            add_polygon,
            activate_color,
            select_at,
            select_rect,
            move_selected,
            scale_selected,
            align,
            distribute,
            mirror,
            set_layer_params,
            toggle_layer,
            generate_gcode,
            ruida_ping,
            ruida_send,
            clear_selection,
            delete_selected,
            get_ui_settings,
            save_ui_settings,
            reset_ui_tab,
            new_project,
            save_project,
            save_version,
            open_project,
            open_version,
            project_list,
            project_detail,
            project_thumb,
            version_thumb,
            project_delete,
            project_rename,
            project_export,
            undo,
            redo,
        ])
        .run(tauri::generate_context!())
        .expect("Fehler beim Starten der LuxiFer-App");
}
