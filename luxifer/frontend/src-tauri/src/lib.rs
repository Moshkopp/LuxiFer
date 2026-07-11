//! LuxiFer Tauri-Backend. Hält den `AppState` des Cores und stellt Commands
//! bereit. Das Frontend zeichnet nur — die gesamte Fachlogik bleibt im Core.

use std::sync::Mutex;

use luxifer_core::preview::JobPreview;
use luxifer_core::{
    assets_dir, import_image, rendered_png, AppState, Geo, ImageParams, LaserRegistry, PolyShape, ShapeInfo, UiSettings,
};
use tauri::{Manager, State};

mod commands;
mod shared;
use commands::laser::*;
use commands::project::*;
use commands::shapes::*;
use shared::{
    base64_encode, plan_with_assets, scene, scene_with,
    ActiveDriver, AppData, CurrentProject, PreviewDto, Scene,
};


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

/// Importiert ein Bild (ADR 0004): legt die (Graustufen-)Kopie im Asset-Store ab
/// und fügt ein Bild-Objekt auf einem eigenen Image-Layer ein. `bytes` sind die
/// rohen Bytes der vom Nutzer gewählten Datei (das Frontend liest sie über einen
/// `<input type=file>` — kein Tauri-Dialog nötig), `name` der Anzeigename.
///
/// Die Zielgröße in mm ergibt sich aus den Pixelmaßen bei 96 DPI, begrenzt auf
/// 80 % der Bettgröße (ein 4K-Bild soll nicht riesig platziert werden), und wird
/// mittig aufs Bett gesetzt. Seitenverhältnis bleibt erhalten.
#[tauri::command]
fn import_image_file(data: State<AppData>, bytes: Vec<u8>, name: String) -> Result<Scene, String> {
    let meta = import_image(&assets_dir(), &bytes, &name).map_err(|e| e.to_string())?;

    let mut s = data.state.lock().unwrap();
    // px → mm bei 96 DPI.
    const PX_TO_MM: f64 = 25.4 / 96.0;
    let mut w = meta.width as f64 * PX_TO_MM;
    let mut h = meta.height as f64 * PX_TO_MM;
    // Auf 80 % der Bettgröße begrenzen, Seitenverhältnis wahren.
    let max_w = s.bed_w_mm * 0.8;
    let max_h = s.bed_h_mm * 0.8;
    if w > max_w || h > max_h {
        let scale = (max_w / w).min(max_h / h);
        w *= scale;
        h *= scale;
    }
    // Mittig aufs Bett.
    let x = (s.bed_w_mm - w) / 2.0;
    let y = (s.bed_h_mm - h) / 2.0;
    s.add_image(meta.id, x, y, w, h);
    Ok(scene_with(&s, &data))
}

/// Rendert ein Asset mit den gegebenen Parametern und gibt es als PNG-Data-URL
/// zurück (Canvas-Darstellung bzw. Editor-Vorschau). `invert` = Editor- oder
/// Laser-Invert (der Aufrufer wählt); für die Canvas-Anzeige `invert_editor`.
#[tauri::command]
fn image_render(asset: String, params: ImageParams, invert: bool) -> Option<String> {
    let png = rendered_png(&assets_dir(), &asset, &params, invert).ok()?;
    Some(format!("data:image/png;base64,{}", base64_encode(&png)))
}

/// Setzt die Bild-Parameter eines Bild-Shapes (Editor). `index` ist der
/// Shape-Index; nicht-Bild-Shapes werden ignoriert.
#[tauri::command]
fn set_image_params(data: State<AppData>, index: usize, params: ImageParams) -> Scene {
    let mut s = data.state.lock().unwrap();
    if let Some(shape) = s.shapes.get_mut(index) {
        if let Geo::Image { params: p, .. } = &mut shape.geo {
            *p = params;
        }
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
    // Gruppen sind eine Einheit: Auswahl auf ganze Gruppen erweitern.
    s.expand_selection_to_groups();
    scene_with(&s, &data)
}

/// Marquee-Auswahl: alle Shapes, deren BBox vollständig im Rechteck liegt.
#[tauri::command]
fn select_rect(data: State<AppData>, x1: f64, y1: f64, x2: f64, y2: f64) -> Scene {
    let mut s = data.state.lock().unwrap();
    s.select_in_rect(x1, y1, x2, y2);
    s.expand_selection_to_groups();
    scene_with(&s, &data)
}

/// Gruppiert die Auswahl (Shapes verhalten sich danach als Einheit).
#[tauri::command]
fn group_op(data: State<AppData>) -> Scene {
    let mut s = data.state.lock().unwrap();
    s.group_selected();
    scene_with(&s, &data)
}

/// Löst die Gruppierung der Auswahl.
#[tauri::command]
fn ungroup_op(data: State<AppData>) -> Scene {
    let mut s = data.state.lock().unwrap();
    s.ungroup_selected();
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
        "center" => Align::Center,
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
        "space-h" => Distribute::SpaceHorizontal,
        "space-v" => Distribute::SpaceVertical,
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
    #[serde(default = "default_bidirectional")]
    bidirectional: bool,
}

fn default_bidirectional() -> bool {
    true
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
            "Image" => LayerMode::Image,
            _ => LayerMode::Cut,
        };
        l.speed_mm_s = p.speed_mm_s;
        l.power_pct = p.power_pct;
        l.min_power_pct = p.min_power_pct;
        l.passes = p.passes;
        l.air_assist = p.air_assist;
        l.line_step_mm = p.line_step_mm;
        l.dpi = p.dpi;
        l.bidirectional = p.bidirectional;
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

/// Verschiebt einen Layer in der Brenn-Reihenfolge (ADR 0005 §0). `from`/`to`
/// sind Layer-Indizes; der Core remappt dabei alle `shape.layer_id`. Ein
/// Undo-Punkt entsteht nur bei tatsächlicher Bewegung.
#[tauri::command]
fn move_layer(data: State<AppData>, from: usize, to: usize) -> Scene {
    let mut s = data.state.lock().unwrap();
    s.move_layer(from, to);
    scene_with(&s, &data)
}


/// Leitet aus dem aktuellen Zustand die Laser-Vorschau ab (ADR 0005): die zu
/// fahrenden Segmente in Ausführungsreihenfolge inkl. Verfahrwege. Reine
/// Ableitung des `JobPlan` — kein Undo, keine Mutation.
#[tauri::command]
fn job_preview(data: State<AppData>) -> PreviewDto {
    let s = data.state.lock().unwrap();
    let plan = plan_with_assets(&s.shapes, &s.layers);
    let preview = JobPreview::from_plan(&plan);
    PreviewDto::from_preview(&preview)
}


/// Lädt die GUI-Settings (Theming, Arbeitsplatz) — ADR 0002.
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
                lasers: Mutex::new(LaserRegistry::load()),
                active: Mutex::new(ActiveDriver::default()),
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
            group_op,
            ungroup_op,
            move_selected,
            scale_selected,
            align,
            distribute,
            mirror,
            boolean_op,
            trace_image,
            list_fonts,
            import_vector_file,
            add_text,
            update_text,
            text_preview,
            pattern_fill_op,
            add_spline,
            add_bezier,
            add_bezier_nodes,
            drag_node,
            hit_bezier_segment,
            split_node,
            delete_node,
            toggle_node_smooth,
            upload_font,
            offset_op,
            fillet_op,
            fillet_corners_op,
            bridge_op,
            nest_op,
            nest_fill_op,
            insert_coasters,
            set_layer_params,
            toggle_layer,
            move_layer,
            job_preview,
            laser_list,
            laser_save,
            laser_delete,
            laser_set_active,
            laser_actions,
            laser_run_action,
            laser_export,
            laser_jog,
            laser_home,
            laser_position,
            laser_ping,
            clear_selection,
            delete_selected,
            get_ui_settings,
            save_ui_settings,
            new_project,
            save_project,
            save_version,
            import_image_file,
            image_render,
            set_image_params,
            open_project,
            open_version,
            delete_version,
            project_list,
            project_detail,
            project_assets,
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
