# Optimierungs- und Architektur-Analyse

Fortgeschrieben. Der obere Teil ist der **aktuelle** Stand (11.07.2026, zweite
Analyse). Der untere Teil (§ „Historie") dokumentiert die erste Analyse und
belegt, was daraus bereits erledigt ist.

---

## Aktueller Stand (Analyse 2)

### Gesamturteil

Die erste Analyse (siehe Historie) ist weitgehend abgearbeitet: zentrale
Shape-Transformationen, Bézier-Synchronisation, Arrange auf Gruppen-Einheiten,
rotationskorrekte Welt-Bounding-Box, kanonische `selection_bbox` in der Scene,
feste Text-Transformregeln, Bézier-Hit-Test im Core, einheitliche
`EditorError`-Fehlerbehandlung und das wiederhergestellte Clippy-Gate sind
committet. Geprüfter Stand: 192 Core-Tests.

Der Design-Canvas rendert weiterhin auf dem CPU-2D-Context, während das
GPU-Fundament (ADR 0008) bereits existiert und im Preview läuft. Die Messung
(unten) hat die Dringlichkeit dieser Weiche allerdings **deutlich relativiert**
und stattdessen zwei kleinere, konkretere Hebel freigelegt.

### Messergebnis (Blocker 1 quantifiziert, 11.07.2026)

Vor der Umstellung wurde gemessen statt angenommen (vgl.
[[messen-vor-architektur-weichen]]). Zwei Klärungen ändern das Bild:

1. **Hatch/Fill belastet den Design-Canvas NICHT.** `fill_segments`/`scanline`
   kommen in `state.rs` nicht vor — Füll-Linien entstehen erst im Job/Preview,
   nicht als Design-Shapes. Eine gefüllte Form zeichnet der Design-Canvas als
   *eine* `ctx.fill()`-Fläche (Canvas.svelte:857), nicht als tausende Linien.
   Der ursprünglich vermutete Haupt-Stress trifft den Design-Canvas also gar
   nicht.
2. **Der reale Punktdruck** kommt nur aus importierten Vektoren (SVG/DXF,
   adaptiv geflattet auf ~10–50 Pkt/Kurve, bezier.rs:135) und Trace-Ergebnissen.

Micro-Benchmark der reinen JS-CPU-Arbeit pro Frame (die im WebView identisch
anfällt; der GPU-Rasterizer-Teil von `ctx.stroke` ist darin NICHT enthalten):

| Szene            | Punkte  | 2D idle | 2D drag | WebGL   |
|------------------|---------|---------|---------|---------|
| mittel (100)     |   4.000 | 0,06 ms | 0,08 ms | ~0 ms   |
| Import-Logo (500)|  20.000 | 0,25 ms | 0,33 ms | ~0 ms   |
| großer Import    |  80.000 | 0,98 ms | 1,30 ms | ~0 ms   |
| sehr groß        | 200.000 | 2,29 ms | 3,07 ms | ~0 ms   |
| extrem           | 800.000 | 9,53 ms | 12,3 ms | ~0 ms   |

Fazit: Die **JS-Geometrie-Vorbereitung** bleibt bis ~200.000 Punkte unter der
16,6-ms-Grenze (60 fps) — sie ruckelt heute allein noch nicht. Erst jenseits
realistischer Projektgrößen wird sie kritisch. WebGL bleibt für sehr große
Importe/Trace das richtige Ziel, ist aber **kein akuter Blocker**.

Nebenbefund (billiger Sofort-Hebel): `liveTransformPoint` ruft
`scene.selected.includes(idx)` **pro Punkt** (Canvas.svelte:787/790). Isoliert
kostet das bei 200.000 Punkten ~2,8 ms/Frame — praktisch der gesamte
JS-Aufwand. Auswertung **pro Shape** über ein `Set` drückt es auf ~0,3 ms (≈9×),
ohne die Render-Architektur anzufassen.

### ✅ Blocker 1 — Design-Canvas auf WebGL (ERLEDIGT 2026-07-11)

Umgesetzt auf Branch `webgl-design-canvas` (Commit „Design-Canvas auf
WebGL-Hybrid umstellen"). Der Design-Canvas rendert Geometrie (Konturen, Grid,
Bett) jetzt über `GlRenderer` wie der Preview; Overlays (Lineale, Handles, Node-/
Mess-/Fillet-Griffe, Draft-Vorschauen), Flächen-Füllungen und Bilder bleiben auf
einer transparenten 2D-Ebene darüber, die auch alle Pointer-Handler trägt.
Live-Drag baut nur die selektierten Konturen pro Frame neu (`liveXf`). Neuer
Batch-Builder `gl/design-render.ts` (UI-frei, Segment-/Rotationslogik gegen den
alten 2D-Pfad geprüft). Verifiziert: svelte-check + Release-Build grün, Grid/Bett/
Shape-Kontur via WebGL im Release sichtbar.

Hinweis: Der WebGL-Umbau war **nicht** die Ursache der gefühlten Latenz (siehe
unten) — er ist die saubere Zielarchitektur, kein Performance-Notfall.

### ✅ Eingabe-Latenz („Cursor klebt") — GELÖST auf Umgebungsebene (2026-07-11)

Getrennte Baustelle, beim WebGL-Test aufgefallen. Symptom: Ziehen/Bewegen wirkte
träge, Form hinkte dem Cursor nach. Diagnose per rohem Cursor-Kreuz (nur
Event→Pixel, keine App-Logik): Das Kreuz hinkte **ebenfalls** hinterher →
Latenz lag **unter JS**, in der WebKitGTK/Wayland-Present-Pipeline, nicht im Code.
Bestätigt: kein Unterschied synchron vs. rAF; unverändert unter nativem Wayland
UND XWayland — bis der Flag `WEBKIT_DISABLE_COMPOSITING_MODE` (Wayland-Blank-
Window-Workaround, erzwingt Software-Present) entfiel.

Fix: `dev.sh` startet über `GDK_BACKEND=x11` (XWayland), volles HW-Compositing
ohne die WebKit-Flags → Versatz deutlich geringer, Ziehen direkt (Rest-Latenz =
normale Compositor/VSync, „nah an flüssig"). Zusätzlich: synchrones Zeichnen
während Gesten + aktuellste Position aus `getCoalescedEvents`. Siehe Memory
`tauri-wayland-start`.

Empfehlung: Basis-Geometrie (Shapes, Grid, Bed) auf `GlRenderer` umstellen wie im
Preview; Overlays (Lineale, Handles, Text-Labels) dürfen ein transparenter
2D-Layer darüber bleiben. **Vorher im Release messen** (Test-Szene mit vielen
Shapes), um die Dringlichkeit zu belegen.

### 🔴 Blocker 2 — `Mutex::lock().unwrap()` 73× im Tauri-Backend

`luxifer/frontend/src-tauri/src/lib.rs` enthält 73 `lock().unwrap()`. Ein
einziger Panic während gehaltenem Lock **vergiftet den Mutex**; danach schlägt
jeder weitere Command fehl → App gefühlt eingefroren, obwohl nur eine Operation
buggy war. Mit wachsender Command-Zahl steigt die Panic-Wahrscheinlichkeit.

Empfehlung: Zentrale Helper-Funktion `with_state(|s| …) -> Result<_, EditorError>`,
die `lock()` kapselt und einen vergifteten Mutex kontrolliert als `EditorError`
zurückgibt statt zu panicken. Da `EditorError` bereits existiert, ist das eine
mechanische, risikoarme Umstellung.

### 🟠 Blocker 3 — Sammelmodule zu groß (weitgehend ERLEDIGT 2026-07-11)

Prinzip: **nach Verantwortlichkeit** schneiden (nicht „ein Typ = eine Datei" —
das würde jedes Feature über viele Dateien zerreißen; ein Feature wie „Shape
zeichnen" geht über alle Typen, ein Typ über alle Features). Geometrie-*Primitive*
dagegen zentral als eine Funktion.

Erledigt:
- **`lib.rs` 1.696 → 185 Zeilen.** Zerlegt in `shared.rs` (geteilte Infra) +
  `commands/{shapes,project,edit,laser,image}.rs`. Wurzel enthält nur noch
  Kern-Commands + `run()`/`generate_handler`. (Tauri-Kniff: `pub`-Commands in
  Submodulen vermeiden die `generate_handler`-Makrokollision E0255.)
- **Geometrie-Primitive** (`lib/geometry.ts`): `ellipsePoints`/`rectPoints`/
  `rotateAroundBBoxCenter`/`boundsOf` als eine Quelle — sammelte die 96-vs-64-
  Duplizierung ein (siehe Punkt 4/5 unten, damit teilweise miterledigt).
- **`Canvas.svelte` 1.562 → 1.490:** zustandsfreie Helfer nach `canvas/handles.ts`
  (Auswahl-Griffe) und `canvas/bezier-draft.ts` (Kurven-Flatten) ausgelagert.

Offen (bewusst als größerer, separater Schritt vertagt):
- `Canvas.svelte` ist noch groß, weil der Rest kamera-/gestengebundenen Zustand
  teilt (`zoom`/`pan`/`drag`/`toScreen`). Saubere Weiterzerlegung braucht einen
  Camera-Store + Tool-Module — höheres Regressionsrisiko, eigener Schritt.
- `App.svelte` (~1.300), `core/src/geo_ops.rs` (~1.057), `state.rs`/`project.rs`
  (~920) noch nicht angefasst.

### 🟡 Punkt 4 — Geometrie-Duplizierung Frontend ↔ Core (mittel)

`shapeBBox()` (core.ts:174) reimplementiert die rotationskorrekte Bounding-Box
(inkl. 64-Punkt-Ellipse-Sampling) in TypeScript, obwohl der Core das jetzt
kanonisch kann und `selection_bbox` bereits in der Scene liefert. Solange beide
existieren, driften sie bei neuen Formtypen auseinander (ThorBurn-Fehler,
CLAUDE.md §1). Kein akuter Bug, aber schleichende Wahrheits-Duplizierung.

Empfehlung: Prüfen, ob `shapeBBox` im Frontend nach Einführung von
`selection_bbox` noch gebraucht wird; wenn ja, über ein read-only Core-Command
lösen.

### 🟡 Punkt 5 — Statische Punktdichte bei Ellipsen/N-Ecken (niedrig-mittel)

Ellipsen/Polygone werden mit fester Auflösung als Polyline gespeichert
(shapes.rs:133, 64 Punkte). Beim starken Hochskalieren wird ein Kreis sichtbar
kantig; die Punktzahl wächst zudem linear mit der Objektzahl in der
Scene-Serialisierung. Aktuell unkritisch, relevant sobald „präzise Kurven" oder
zoom-abhängiges Tessellieren gefordert wird. Notieren, nicht sofort handeln.

### Empfohlene Reihenfolge (Stand 2026-07-11 spät)

0. ✅ **Messen erledigt** → Blocker 1 quantifiziert.
1. ✅ **Blocker 1 (WebGL-Hybrid) erledigt** (auf `main` gemergt).
2. ✅ **Eingabe-Latenz gelöst** (GDK_BACKEND=x11, siehe oben).
3. ✅ **Blocker 3 (Modulzerlegung) weitgehend erledigt** — lib.rs → shared.rs +
   commands/*, geometry.ts, Canvas-Helfer ausgelagert. Rest (Camera-Store/
   Tool-Module, App.svelte, Core-Module) offen.
4. **Offen — Blocker 2** (Mutex-Kapselung): `lock().unwrap()` (jetzt über die
   `commands/*`-Module verteilt) durch einen `with_state()`-Helper mit
   `EditorError` ersetzen. Klein, risikoarm — guter nächster Schritt.
5. **Punkt 4** (Frontend↔Core-BBox-Duplizierung) — `shapeBBox` in core.ts nutzt
   jetzt die zentralen Primitive (geometry.ts), driftet aber weiter vom Core.
   Prüfen, ob es ganz durch `selection_bbox` ersetzbar ist. **Punkt 5** (statische
   Ellipsen-Auflösung) jetzt an EINER Stelle (`ELLIPSE_SEGS` in geometry.ts).

---

## Historie (Analyse 1) — überwiegend erledigt

Erste Analyse (11.07.2026, früher am Tag). Die als kritisch/hoch markierten
Punkte wurden anschließend umgesetzt und committet.

### Was daraus erledigt ist

- Zentrale Shape-Transformationen (Verschieben, Skalieren, Spiegeln) halten
  editierbare Bézier-Metadaten synchron; Regressionstests sichern das ab.
- Bézier-Anker/Tangenten werden bei Verschieben, Skalieren, Spiegeln, Arrange und
  Nesting gemeinsam mit der Kontur transformiert.
- Arrange bildet über `group_id` echte Einheiten; Gruppen und Textblöcke werden
  beim Ausrichten/Verteilen nicht mehr auseinandergerissen.
- Verteilung nach Objektmitten plus gleichmäßige horizontale/vertikale
  Zwischenräume.
- Rotationskorrekte Welt-Bounding-Box im Core; Auswahl, Arrange, Hit-Test und
  Transformanzeige nutzen sie bei gedrehten Shapes.
- Kanonische Auswahl-Bounding-Box wird im Core berechnet und als Teil der `Scene`
  ausgeliefert (`selection_bbox`).
- Transform-Leiste mit X/Y, Breite/Höhe, Seitenverhältnis-Sperre, 3×3-Anker;
  Null-Breite/-Höhe abgefangen (kein Infinity/NaN mehr).
- Feste Text-Transformregel: proportionale Skalierung aktualisiert `size_mm`;
  nichtproportional/Spiegelung entfernt nicht reproduzierbare Textparameter.
- Beide offenen Clippy-Warnungen behoben; Clippy-Gate wieder belastbar.
- Bézier-Segment-Hit-Test im Rust-Core (Frontend übergibt nur Weltposition +
  zoomabhängige Toleranz).
- Alle Tauri-Aufrufe laufen durch eine gemeinsame Invoke-Grenze; Fehler als
  `EditorError` normalisiert und zentral angezeigt.

Relevante Commits u. a.: `ba52247`, `97823ea`, `1492957`, `8c787cd`, `3260d5f`,
`74b3e78`, `e5e2c03`.

### Was aus Analyse 1 offen blieb → in Analyse 2 fortgeführt

- Modulzerlegung der großen Sammelmodule (jetzt Blocker 3).
- Einheitliche Fehlerbehandlung im Backend war nur teilweise erledigt: der
  `Mutex::lock().unwrap()`-Umgang ist noch offen (jetzt Blocker 2).
- Die Render-Architektur (CPU-Canvas im Design) war in Analyse 1 noch nicht als
  eigener Punkt erfasst — jetzt Blocker 1 und wichtigster nächster Schritt.
