# Native-Bedienung: Mängelliste & TODO

Stand: 2026-07-12. Vom Nutzer am laufenden Fenster gesammelt. Diese Liste wird
analysiert, priorisiert und abgearbeitet. Klassifizierung:

- **REG** = Regression: ging in der Tauri-App, durch die native Migration
  verloren/kaputt.
- **FEHLT** = Feature war/ist noch nicht nativ umgesetzt (bekannte Lücke).
- **UX** = vorhanden, aber unbrauchbar/unklar/hässlich.
- **BUG** = klar falsches Verhalten.

Priorität: P1 = blockiert normales Arbeiten, P2 = wichtig, P3 = Politur.

---

## A. Canvas / Zeichnen / Auswahl

| ID | Klasse | Prio | Beschreibung |
|----|--------|------|--------------|
| A1 | ERLEDIGT | P1 | Auswahl-Werkzeug zeichnet einen bildschirmkonstant gestrichelten Marquee-Rahmen. |
| A2 | ERLEDIGT | P1 | Bézier-Feder: Drücken setzt Anker, Ziehen erzeugt symmetrische Tangenten; Enter schließt den Entwurf ab. |
| A3 | ERLEDIGT | P2 | Spline/Polyline/Bézier rasten nahe dem Startknoten ein; Klick oder Enter schließt den Pfad, der Startknoten signalisiert die Fangzone farbig. |
| A4 | ERLEDIGT | P1 | Strg+Z = Undo, Strg+Shift+Z und Strg+Y = Redo. |
| A5 | ERLEDIGT | P3 | Undo/Redo als Icon-Buttons im Header; Vektor-/Bild-Import zu einem Icon-Button zusammengelegt. |

## B. Geometrie-Operationen

| ID | Klasse | Prio | Beschreibung |
|----|--------|------|--------------|
| B1 | ERLEDIGT | P1 | Geschlossene konvexe Linienkonturen behalten beim Offset harte Miter-Ecken statt verrundeter Übergänge. |
| B2 | ERLEDIGT | P2 | Muster-Füllung mit Parameterdialog (Linien/Kreise/Langlöcher/Waben, Abstände, Winkel, Elementgröße); Füllung landet auf eigenem Layer, ein Undo-Schritt. |
| B3 | FEHLT | P2 | Haltesteg ist nur Stub. |
| B4 | ERLEDIGT | P1 | Muster-Füllung blieb beim Verschieben der Quellform stehen (Muster-Konturen sind eigenständige Shapes): Muster und Quellformen gruppieren sich jetzt automatisch — die Gruppenauswahl nimmt die Füllung beim Move mit. Der normale Scanline-Fill folgte nachweislich korrekt (frame-genauer Gesten-Test); falls dort weiter etwas hakt, bitte Schritte notieren. |

## C. Bilder

| ID | Klasse | Prio | Beschreibung |
|----|--------|------|--------------|
| C1 | ERLEDIGT | P1 | Bildtexturen werden nicht mehr von Scanlines der pinken Bild-Layer-Kennfarbe überzeichnet. |
| C2 | UX | P2 | Bild-Doppelklick-Dialog hat keine Live-Vorschau der Einstellungen. |
| C3 | FERTIG | P2 | Bildparameter, Crop und Trace besitzen getrennte kompakte Arbeitsbereiche mit Live-Vorschau. Crop erzeugt ein abgeleitetes Asset und bleibt per Undo reversibel. |

## D. Fills / Vorschau

| ID | Klasse | Prio | Beschreibung |
|----|--------|------|--------------|
| D1 | ENTSCHIEDEN | — | Scanlines bleiben bewusst im Design-Tab: direkte Kontrolle des Fill-Ergebnisses; der native Vertex-Cache zeigt aktuell keinen spürbaren Performance-Einbruch. |
| D2 | ERLEDIGT | P1 | Vorschau zeigt Cut/Fill/Travel, **verarbeitete** Bild-Rasterungen (dieselbe Rasterung wie der echte Job) und eine Legende mit Kennzahlen (Arbeitsweg, Leerfahrt, Job-Fläche). Simulation/Scrubber bleibt offen. |
| D3 | ERLEDIGT | P1 | Vorschau ist eine Material-Bühne: rechts Vorlage wählen (Holz hell, Holz dunkel, Schiefer). Untergrund = Materialfarbe; Brennwege und Rasterbilder in Brennfarbe (weiß auf Schiefer, dunkel auf Holz). Leerfahrten per Schalter (Standard aus), zählen aber immer in die Kennzahlen. |

## E. Panels / Layout / Views

| ID | Klasse | Prio | Beschreibung |
|----|--------|------|--------------|
| E1 | ERLEDIGT | P1 | Rechtes Panel ist mit 340 px sinnvoll vorbelegt und zwischen 300–460 px responsiv verstellbar. |
| E2 | ERLEDIGT | P2 | Layer erscheinen als lesbare Inspector-Karten mit Name, Modus, Objektzahl und ausgeschriebenen Zuständen. |
| E3 | ERLEDIGT | P2 | Laser-Tab erzwingt Auswahl, sperrt Zeichnen/Löschen und gibt Layer nur temporär für Verschieben/Skalieren/Drehen frei. |
| E4 | ERLEDIGT | P1 | Projektbrowser ist Master-Detail: Liste links, rechts Metadaten, Vektor-Miniatur, Umbenennen, Export, zweistufiges Löschen und Versionsliste (Laden/Löschen). PNG-Thumbnails pro Version bleiben offen. |
| E5 | ERLEDIGT | P1 | Laser-Tab: Panel lief über den rechten Rand hinaus (Profilzeile zu breit), die Ebenenliste fehlte, und die Treiber-Rückmeldung stand unsichtbar ganz unten. Jetzt: Ebenenliste + Positionsfreigabe in eigenem linken Panel (resizierbar, scrollt), Laser-Bedienpanel rechts, Rückmeldung bei den Job-Kacheln. |
| E6 | ERLEDIGT | P1 | Job-Buttons schlugen IMMER fehl („Laser-Aktion fehlgeschlagen [laser_action]"): Der LaserService rief nie `connect()` auf — jede Geräteaktion lief in `NotConnected`. Jetzt verbindet er vor verbindungsbedürftigen Aktionen (Export weiterhin ohne Gerät); das Fehlerbanner zeigt zusätzlich die technische Ursache. HW-verifiziert: Absolut fährt korrekt. |
| E7 | ERLEDIGT | P1 | Startmodus „Aktuelle Position"/„Benutzerursprung" fuhr trotzdem absolut (an HW beobachtet): Dem Ruida-Job fehlten F-Block + zweiter BBox-Satz — ohne diese Register ignoriert der Controller das Startmodus-Byte der Preamble. **HW-verifiziert: Start fährt jetzt relativ korrekt.** |
| E8 | ERLEDIGT | P1 | Rahmen/Gummiband ignorierten den Startmodus (fuhren immer die absolute Job-BBox ab, an HW beobachtet) und nullten die Leistung nicht. Jetzt Referenzlogik: Ankerpunkt der Rahmen-BBox landet auf Kopfposition bzw. Benutzerursprung; Leistungsregister werden im Rahmen-Paket genullt. **HW-verifiziert.** |
| E9 | ERLEDIGT | P1 | Startmarker im Laser-Canvas fehlte: grünes Fadenkreuz am gewählten Job-Nullpunkt-Anker der Job-BBox (nur bei relativem Startmodus, wie in der Tauri-App). HW-/Sichtabnahme durch den Nutzer bestätigt. |

## F. Header / Werkzeug-Zugänge

| ID | Klasse | Prio | Beschreibung |
|----|--------|------|--------------|
| F1 | UX | P2 | Vektor- und Bildladen sollen über einen gemeinsamen Import-Button laufen. |
| F2 | UX | P3 | „Aztec laden" und „Text einfügen" gehören in den Header. |

## G. Text

| ID | Klasse | Prio | Beschreibung |
|----|--------|------|--------------|
| G1 | FEHLT | P2 | Text-Tool: keine Vorschau. |
| G2 | FEHLT | P2 | Text-Tool: kein Upload eigener Fonts. |

## H. Canvas-Grid / Lineale

| ID | Klasse | Prio | Beschreibung |
|----|--------|------|--------------|
| H1 | ERLEDIGT | P2 | Grid füllt den sichtbaren Ausschnitt bis zum Fensterrand; der Arbeitsbereich ist Fläche+Rahmen. Zoom-adaptiv gegen Moiré: Feinlinien blenden unter 14 px Bildschirm-Abstand weich aus, ab 7 px vergröbert der Schritt ×5 (Settings-Raster bleibt beim Arbeitszoom unverändert). |
| H2 | ERLEDIGT | P2 | Lineale oben/links in mm, Tick-Intervall zoomabhängig (1/2/5er-Reihe), Cursor-Marker in Akzentfarbe. |

---

## Analyse-Notizen (wird ergänzt)

- A1 (erledigt): `Drag::Marquee` wird als gestricheltes, jeden Frame neu
  aufgebautes Overlay gezeichnet; der Geometrie-Cache bleibt auswahlfrei.
- A4 (erledigt): `Strg+Shift+Z` wird vor `Strg+Z` ausgewertet und löst Redo
  aus. `Strg+Y` bleibt als Alias erhalten; Fokus-/Modal-Gate bleibt wirksam.
- A2 (erledigt): Der Canvas hält während des Zeichnens echte `BezierNode`-Drafts.
  Beim Drag entstehen `h_in`/`h_out`, das Overlay zeigt Kurve, Tangenten und
  Anker live; die Application übernimmt den fertigen Pfad als einen Undo-Schritt.
- A3 (erledigt): Eine bildschirmkonstante 10-px-Fangzone schließt Pfade ab drei
  Knoten. Overlay-Gummiband und Startmarker zeigen das Einrasten; Application
  erzeugt für Klick und Enter echte geschlossene Polyline-/Spline-/Bézier-Pfade.
- B4 (erledigt): Für den normalen Fill wurde der komplette Pfad geprüft und
  per Test abgesichert (echte Move-Geste über `CanvasState::on_mouse`/
  `on_cursor_move`, Render-Revision und Szenen-Vertices pro Frame — die
  Scanlines folgen). Pattern-Fill war strukturell betroffen: Die Muster-
  Polylinien sind eigene Shapes auf dem „Muster"-Layer; ohne Verknüpfung
  bleiben sie beim Verschieben der Quelle stehen. `pattern_fill_selected`
  vergibt jetzt eine gemeinsame `group_id` für Quellen + Muster (bzw. tritt
  einer bestehenden Quell-Gruppe bei); `expand_selection_to_groups` nimmt
  das Muster damit bei jeder Auswahl mit. End-zu-End getestet (Fill →
  Klick auf Quelle → Move → Muster-BBox wandert mit).
- B2 (erledigt): `EditorSession::pattern_fill` validiert Abstände/Größe/Winkel
  und macht die stille Core-No-Op (keine geschlossene Kontur in der Auswahl)
  als stabilen Fehler sichtbar. Der Dialog erweitert den bestehenden
  GeoOp-Parameterdialog (vierte Variante); die Elementgröße ist bei „Linien"
  deaktiviert, weil sie dort keine Bedeutung hat.
- C3/Trace (erledigt): `EditorSession::trace_image` lädt das Asset, wendet die
  Tonwert-LUT des Bildes an (Helligkeit/Kontrast/Gamma wirken vor der
  Schwelle) und tract über den Core; die Konturen landen skaliert in mm auf
  dem aktiven Zeichen-Layer (ein Core-Undo über `add_polylines`). Die UI ist
  eine „Vektorisieren"-Sektion im Bild-Dialog (Schwelle 0–255, Invertieren);
  der Dialog bleibt nach dem Trace offen, damit man die Schwelle nachziehen
  und erneut tracen kann. Fehlerpfade (kein Bild, fehlendes Asset, keine
  Konturen) sind stabile `AppError`s ohne Mutation. Crop validiert die Schnittkanten und passt Asset, Position und Bildbox in einem Undo-Schritt an.
- B1 (erledigt): `cavalier_contours` erzeugte standardmäßig runde Außen-Joins.
  Geschlossene konvexe Linienkonturen nutzen nun im Core Schnittpunkte
  benachbarter Parallelkanten (Miter); kollabierte Innenoffsets bleiben leer.
  Konkave und offene Konturen behalten die robuste Selbstschnittbehandlung.
- C1 (erledigt): RGBA-Textur und Shader waren korrekt. Der nachfolgende native
  Vektor-Fill-Pass behandelte jedoch `LayerMode::Image` als Füllkontur und malte
  die rechteckige Bildfläche in der Layer-Kennfarbe über. Zusätzlich lag die
  Textur vor dem opaken Bett. Die Reihenfolge ist nun Bett/Gitter → Bildtexturen
  → Vektorgeometrie → Overlay; Image-Layer erzeugen keine Fill-Scanlines mehr.
- D1 (bewusst beibehalten): Anders als zunächst geplant bleiben Fill-Scanlines
  im Design-Tab sichtbar. Sie liefern sofortige visuelle Kontrolle über das
  tatsächliche Fill-Ergebnis; dank gecachtem Vertexpuffer ist derzeit kein
  wahrnehmbarer Performanceverlust vorhanden. Nur bei belegbarer Regression
  erneut aufgreifen.
- D3 (erledigt): `PreviewMaterial` (canvas/scene.rs) definiert Untergrund-,
  Brenn- und Leerfahrt-Farbe je Vorlage; die Layerfarben-Ansicht entfällt in
  der Vorschau bewusst — sie zeigt das Werkstück, die Farben hat der
  Design-Tab. Wichtig fürs Rendering: Der Framebuffer ist sRGB, die
  Vertex-Farben laufen daher durch `srgb_to_linear`, sonst erschiene
  Schiefer mittelgrau statt fast schwarz (das egui-Panel nutzt dieselben
  sRGB-Werte direkt — Karte und Canvas zeigen exakt denselben Ton).
  Materialwechsel/Travel-Schalter invalidieren den Preview-Cache.
- D2 (erledigt): Der read-only Preview-Reiter zeichnet Cut-, Fill- und
  Travel-Bewegungen aus `EditorSession::job_preview`/`JobPlan`; Editor-
  Shortcuts, Gesten und Overlay sind gesperrt, Navigation per Mittelmaus/
  Mausrad. Bild-Layer zeigen jetzt die **verarbeitete Rastertextur** (Pixel
  255 = gebrannt) statt der Design-Textur; der Asset-Resolver
  (`application::assets::resolve_luma`) ist derselbe wie im echten Job.
  Asset-Dekodierung und Rasterung laufen auf einem Session-Snapshot in einem
  Worker statt im Renderthread; der Cache-Schlüssel umfasst Renderrevision,
  Auswahlmodus, Material und Travel-Anzeige. Während der Berechnung bleibt die
  UI navigierbar und zeigt die Materialbühne bzw. den letzten fertigen Stand.
  Dabei wurde eine gefährliche Lücke geschlossen: `LaserService::plan` plante
  zuvor OHNE Assets — Bild-Layer wären beim echten Brennen/Export
  stillschweigend übersprungen worden, obwohl die Vorschau sie zeigt. Eine
  Legende (schwebendes Fenster) erklärt die Farben (Schnitt je Layer,
  Füllung, Bild-Gravur, Leerfahrt) und zeigt Arbeitsweg/Leerfahrt/Job-Fläche.
  Nebenbei: `import_path` importiert jetzt auch Bilddateien (CLI-Argument;
  Vorarbeit für F1). Offen bleibt die Simulation (Scrubber/Abspielen).
- E4 (erledigt): Der Browser zeigt links die wählbare Projektliste (Doppelklick
  öffnet), rechts den Detailbereich aus `ProjectService::detail`: Metadaten,
  eine live gezeichnete Vektor-Miniatur (`peek_state`, beim offenen Projekt die
  Session), Umbenennen-Entwurf, Export und zweistufiges Löschen. Die
  Versionsliste lädt/löscht Versionen des offenen Projekts; das Löschen der
  aktuellen Version ersetzt den Canvas durch die vom Core beförderte Version
  (Service-Bug behoben: der beförderte Zustand wurde zuvor verworfen) und läuft
  wie Version-Laden über den Dirty-Guard. Statt gespeicherter PNG-Thumbnails
  gibt es die Live-Miniatur; PNG-Thumbnails pro Version (Speicherpfad ist im
  Core vorhanden) bleiben als Feinarbeit offen.
- E5 (erledigt): Der Überlauf kam aus der Profilzeile des Laserpanels — die
  ComboBox reservierte nur 34 px für den „Verwalten"-Knopf; die Zeile drückte
  alle folgenden `available_width()`-Berechnungen über den Panelrand. Jetzt
  liegt der Knopf rechtsbündig (right-to-left) und die Combo füllt exakt den
  Rest. Der Inspector-Inhalt (Design und Laser) steckt in einer vertikalen
  ScrollArea (`auto_shrink false`), damit kleine Fenster bedienbar bleiben.
  Im Laser-Tab liegt die volle Ebenenliste (Job an/aus, Parameterdialog,
  Reihenfolge — Brennvorbereitung) plus Positions-Freigabeliste in einem
  EIGENEN linken Panel (260–420 px, resizierbar): rechts mit dem Bedienpanel
  zusammengequetscht wäre sie bei zehn Ebenen unbrauchbar; links ersetzt sie
  die im Laser-Tab ohnehin gesperrte Werkzeugleiste.
  Die Start/Stopp/Rahmen-Verdrahtung war bereits
  vollständig (`UiAction::LaserRun` → `LaserService::run_action`, hardwarelos
  getestet); nur der Modulkommentar behauptete noch „loggen vorerst". Die
  Treiber-Rückmeldung erscheint jetzt direkt unter den Job-Kacheln.
- E6 (erledigt): Die Migration hatte Tauris `needs_connection`/`connect_active`
  verloren — `driver_for` baute nur das Treiberobjekt, verband aber nie; der
  Ruida-Treiber liefert dann bei jeder Geräteaktion `NotConnected`, und das
  Banner verschluckte die Ursache (AppError-`details` wurden nie angezeigt).
  Jetzt: `with_driver(connect, …)` verbindet vor SendJob/Frame/Gummiband/
  Pause/Stopp/Home/Ursprung/Jog (idempotent im Treiber, Ziel aus dem Profil:
  IP bzw. serieller Port); Export kompiliert weiterhin ohne Gerät. Ohne
  erreichbares Gerät kommt „Keine Verbindung zum Laser (IP)" mit technischer
  Ursache im Banner (Ruida-Ping: 300 ms Timeout). Getestet: Klassifikation
  der Verbindungspflicht + Fehlerpfad gegen 127.0.0.1. Der synchrone
  Verbindungsaufbau blockiert die UI kurz (~300 ms) — der asynchrone
  Geräteablauf bleibt die bekannte offene Architekturfrage. **Abnahme an
  echter Hardware steht aus.**
- E7 (erledigt): Der native Ruida-Job bestand aus Preamble → Layer-Config →
  Geometrie → Trailer; die funktionierende Referenz baut Preamble →
  Layer-Config → **F-Block + zweiter BBox-Satz** → Geometrie → Trailer. In
  den fehlenden F1/F2- und E7-13/17/23/37-Registern steht die (bei relativem
  Start verschobene) Job-BBox samt Breite/Höhe — offenbar leitet der
  Controller daraus die Platzierung ab; ohne sie fällt er auf absolut
  zurück. Zusätzlich angeglichen: Die Layer-Anzeige-BBox (E7 52/53/61/62)
  bleibt wie in der Referenz in Tischkoordinaten, verschoben werden nur
  Geometrie und Job-BBox. Tests prüfen die Job-Struktur und dass der
  relative Modus die Job-BBox (F2 03 = −Anker), nicht aber die Layer-BBox
  verschiebt. **Bitte an der Maschine gegenprüfen: „Aktuelle Position" und
  „Benutzerursprung" mit Anker Mitte/Ecken.**
- E8/E9 (erledigt): `frame`/`rubber_frame` fuhren die Job-BBox bzw. Hülle in
  absoluten Tischkoordinaten ab — Startmodus und Anker wurden ignoriert, und
  die Leistungsregister blieben ungenullt. Beide laufen jetzt über einen
  gemeinsamen `drive_frame` nach der Referenzlogik: Referenzpunkt je Modus
  lesen (Kopfposition bzw. Benutzerursprung), Ankerpunkt der Rahmen-BBox
  dorthin verschieben (`shift_frame_points`, getestet), Sequenz nullt vorher
  MIN/MAX-Leistung und kehrt zur Ausgangsposition zurück — alles in einem
  Paket. Der `MachineDriver`-Trait gibt `frame`/`rubber_frame` dafür die
  `JobParams` mit. Der Startmarker (E9) kommt aus
  `EditorSession::job_start_marker` (Anker auf der Job-BBox aus denselben
  rotierten Konturpunkten wie der Plan, ohne Fill-/Raster-Rechnung) und wird
  im Laser-Tab als bildschirmkonstantes Overlay gezeichnet.
- E1/E2 (erledigt): Der Inspector ist breiter und resizbar. Layer-Karten trennen
  Identität (Farbe/Name/Modus/Objektzahl), Zustände (Sichtbar/Job/Gesperrt/Luft)
  und Reihenfolge klar; der Name öffnet den Parameterdialog direkt.
- E3 (erledigt): Der Laser-Tab setzt automatisch das Auswahlwerkzeug und hält
  alle Layer zunächst nur in der UI gesperrt. Einzelne Layer lassen sich unter
  „Position bearbeiten“ temporär für Move/Resize/Rotate freigeben; beim
  Tabwechsel verfallen die Freigaben. Core-Locks, Dirty-State und Undo bleiben
  vom reinen Ansichtswechsel unberührt.
