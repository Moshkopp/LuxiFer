# luxifer-native — nativer Editor (winit + wgpu + egui)

Der Umbau aus **ADR 0010**: die Zeichen-/UI-Schicht von LuxiFer nativ, ohne
WebView, ohne IPC. `luxifer-core` bleibt die einzige Quelle der Wahrheit und wird
direkt gelinkt. Läuft **neben** der weiter funktionierenden Tauri-App
(`luxifer/frontend/`), damit man vergleichen kann.

## Warum

Direkter Release-Vergleich mit ThorBurn (Qt) zeigte LuxiFer auf allen Achsen
träge. Ein wgpu-Spike bewies: derselbe Core + dieselbe Aztec-Geometrie laufen
nativ mit 144 fps, während die Tauri-App kriecht — der Engpass ist die
IPC-Brücke (73k Segmente Rust→JSON→JS) plus WebKitGTK-Overhead, nicht der Core
und nicht die GPU. Details: `docs/adr/0010-nativer-renderer-wgpu.md`.

## Starten

```bash
# aus der Repo-Wurzel
GDK_BACKEND=x11 cargo run -p luxifer-native --release

# mit direkt geladener Datei (erstes Argument):
GDK_BACKEND=x11 cargo run -p luxifer-native --release -- /pfad/zu/datei.svg
```

`GDK_BACKEND=x11` aus demselben Grund wie in `dev.sh` (Wayland-Present-Latenz).

### Test-Umgebungsvariablen
- `LUXI_FILL=1` — beim Auto-Import gleich alle Layer auf Fill stellen.
- `LUXI_TAB=laser` — rechten Reiter direkt auf Laser starten.

## Was läuft (Stand: Umbau-Branch)

- **Canvas** (wgpu, LineList): Shapes aus dem echten `AppState`, Tisch-Rahmen,
  Auswahl-Hervorhebung, Auswahl-BBox. Pan (mittlere Maus / Leertaste+links),
  Zoom (Mausrad, auf den Cursor).
- **Flächen-Fill** über `scanline::fill_segments` (derselbe Even-Odd-Fill wie die
  Laser-Vorschau), als horizontale Linien pro fillbarem Layer.
- **Vertex-Cache**: Geometrie wird nur bei Szenen-Änderung neu gebaut
  (Fingerprint), Pan/Zoom projiziert allein der Shader. Aztec (1810 Shapes,
  ~73k Fill-Segmente) → **146 fps**.
- **Import**: SVG/DXF über `import_vector` + nativer Datei-Dialog (rfd).
- **Interaktion** über den Core: Rechteck/Ellipse/Polygon zeichnen, Auswahl +
  Hit-Test, Verschieben, Marquee-Auswahl, Farbe/Layer, Undo/Redo, Löschen.
- **Panels** (egui): Werkzeuge links; rechts per Tab Design (Ebenen + Palette
  mit aktiver-Farbe-Markierung) oder **Laser** (Ampel-Grid Start/Pause/Stopp/
  Ursprung/Rahmen/Gummiband, Nullpunkt-Anker, Jog-Kreuz, Schritt/Speed-Slider).
- Tastatur: V/R/E/P Werkzeuge, Z/Y Undo/Redo, Entf löschen, Esc abbrechen,
  Enter Polygon schließen.

## Was noch fehlt (nächste Schritte)

1. **Laser-Treiber anbinden** — das Laserpanel loggt Aktionen nur; die echte
   Treiber-/Job-Logik (Ruida etc.) muss verdrahtet werden.
2. **Transform-Handles** (Resize/Rotate der Auswahl) — der Core kann es
   (`interact::resize_bbox`, `scale_selection_to`, `rotate_selection`), das UI
   fehlt.
3. **Text, Bild-Import-Vorschau, Laser-Vorschau-Reiter, Projekt-Browser** — alles
   im Core vorhanden, UI muss nativ nachgebaut werden.
4. **Fill-Darstellung** verfeinern (aktuell Hatch-Linien; ggf. echte Flächen via
   Stencil, s. ADR 0009 §1).
5. **Konturen mit Dicke** (aktuell 1px-LineList) + Anti-Aliasing (MSAA).

## Architektur (Module)

- `main.rs` — winit-Loop.
- `app.rs` — hält `AppState` + Kamera + Tool/Tab-Zustand, verbindet Eingaben mit
  Core-Aufrufen, rendert Canvas + egui in einen Frame. Vertex-Cache.
- `gpu.rs` — wgpu-Setup, Canvas-Pipeline (LineList), Kamera-Uniforms.
- `camera.rs` — Welt(mm)↔Bildschirm(px), Pan/Zoom/Fit.
- `scene_geo.rs` — `AppState` → Vertices (Konturen + Scanline-Fill).
- `ui.rs` — egui-Panels (Werkzeuge, Ebenen, Palette, Tabs).
- `laserpanel.rs` — Laser-Bedienpanel (egui).
- `tools.rs` — Werkzeug-/Tab-/Laser-UI-Zustand.

**Invariante bleibt gewahrt:** keine Fachlogik hier — alles Mutierende geht durch
`luxifer-core`.
