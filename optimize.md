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

### 🟠 Blocker 1 (herabgestuft) — Design-Canvas auf WebGL

ADR 0008 hat `luxifer/frontend/src/lib/gl/renderer.ts` als *die eine*
GPU-Zeichenschicht etabliert; `PreviewCanvas.svelte` nutzt sie bereits, der
Design-Canvas (Canvas.svelte:343/809) noch nicht. Laut Messung erst bei sehr
großen Importen/Trace ein echter Ruckel-Engpass — **nach** Blocker 2 angehen,
gebündelt mit der Modulzerlegung (Blocker 3). Sofort-Hebel vorab: `Set`-basierte
`selected`-Prüfung pro Shape (siehe Nebenbefund).

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

### 🟠 Blocker 3 — Sammelmodule zu groß

- `frontend/src-tauri/src/lib.rs`: ~1.696 Zeilen
- `Canvas.svelte`: ~1.450 Zeilen
- `App.svelte`: ~1.303 Zeilen
- `core/src/geo_ops.rs`: ~1.057 Zeilen
- `core/src/state.rs` / `core/src/project.rs`: ~920 Zeilen

`Canvas.svelte` mischt Kamera, Raster/Lineale, Rendering, Auswahl, Resize,
Bézier, Node-Edit, Messen, Haltestege, Fillet, Bilder, Tastatur. Änderung an
einem Werkzeug kann ein anderes brechen, weil `draw()` alles gemeinsam anfasst.
Koppelt mit Blocker 1: Die 2D→GL-Umstellung ist ungleich schwerer, solange
Rendering und Werkzeugzustand in einer Datei verklebt sind.

Empfehlung: Beim Angehen von Blocker 1 gleich `canvas/render.ts` +
`canvas/camera.ts` herausziehen; Werkzeuge (`tools/bezier.ts`, `tools/node.ts`)
folgen inkrementell. `lib.rs` nach Command-Gruppen splitten
(`commands/shapes.rs`, `commands/arrange.rs`, …).

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

### Empfohlene Reihenfolge (nach der Messung aktualisiert)

0. ✅ **Messen erledigt** → Blocker 1 herabgestuft; JS-Anteil ruckelt bis
   ~200.000 Punkte nicht (Details oben).
1. **Sofort-Hebel:** `selected.includes(idx)` in `liveTransformPoint` durch eine
   pro-Shape ausgewertete `Set`-Prüfung ersetzen (~9× auf dem heißen Pfad,
   winziger Diff, keine Architekturänderung).
2. **Blocker 2** (Mutex-Kapselung) — klein, risikoarm, verhindert App-weite
   Abstürze. Guter nächster Schritt.
3. **Blocker 1 + 3 zusammen** — Design-Canvas auf `GlRenderer`, dabei
   `render.ts`/`camera.ts` herausziehen. Erst relevant für sehr große
   Importe/Trace; nicht mehr dringend, aber weiterhin die saubere Zielarchitektur.
4. Punkt 4/5 als Aufräumarbeiten danach.

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
