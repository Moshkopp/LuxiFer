# ADR 0002: Panel-System (freie Positionierung, Reiter, Theming, Arbeitsplatz-Settings)

## Status
Akzeptiert — 2026-07-06

## Kontext

Die GUI ist das Produkt. Bisher sind die Panele (Werkzeuge, Ebenen, Farbpalette,
Laser) mit **festen Pixel-Positionen** im Fenster verankert. Das erzeugt ein
konkretes Problem:

Der Arbeitsplatz hat **zwei Monitore mit unterschiedlicher Auflösung**
(FullHD 1920×1080 und WQHD 2560×1440). Pixel-verankerte Panele landen beim
Wechsel der Auflösung/des Monitors an der falschen Stelle oder außerhalb des
sichtbaren Bereichs — das „Positionier-Problem", das sich durch alle bisherigen
Versionen zog.

Diese ADR legt ein **auflösungsunabhängiges Panel-System** fest (Panele frei
relativ positioniert), dazu wählbares „Frosted Depth"-Glas-Theming (Akzent- +
Button-Farbe), reiterbasierte Sichtbarkeit und arbeitsplatzbezogene, lokal
gespeicherte GUI-Settings (später über Charon synchronisierbar).

## Entscheidung

### 1. Freie relative Positionierung (löst das Auflösungsproblem)

Panele werden **nicht in Pixeln**, sondern in **Bruchteilen des Fensters**
positioniert. Jedes Panel merkt sich `{ x, y, w, h }` als Anteil 0…1 der
Fensterbreite/-höhe plus ein `z` für die Stapel-Reihenfolge.

- **Auflösungsunabhängig:** Dieselben Bruchteile sitzen auf FullHD und WQHD an
  der *gleichen relativen* Stelle — nie mehr außerhalb. Das war der Kern gegen
  das Positionier-Problem.
- **Frei, ohne Raster/Snap.** Panele lassen sich **stufenlos** verschieben und
  in der Größe ziehen. Ein festes Zell-Raster mit Snap wurde verworfen: es war
  in der Praxis entweder zu grob oder zu fummelig. Freie Positionierung trifft
  genau die gewünschte Stelle.
- **Panele dürfen sich überlappen (frei schwebend).** Es gibt **keine
  Verdrängungs- oder Kollisionslogik** — die z-Reihenfolge entscheidet, was oben
  liegt. Kollisionen vermeidet der Nutzer selbst beim Anordnen. Das hält den
  Umbau schlank und passt dazu, dass Panele ohnehin über dem Canvas schweben.
- **Größe passt Inhalt an:** Panele sind responsiv; der Inhalt fließt in die
  gegebene Fläche (Toolbar wird bei genügend Breite **1- oder 2-spaltig**,
  Listen scrollen bei Bedarf). Die Breite steuert also unmittelbar das
  Erscheinungsbild.

Panele „schweben" über dem Canvas (Canvas ist die vollflächige Grundebene); die
Bruchteile bestimmen nur ihre Position/Größe, nicht ob sie den Canvas
verdrängen.

### 2. Reiter: Design / Laser / Monitor

Oben gibt es Reiter **Design**, **Laser**, **Monitor** (Monitor kommt später).
**Jeder Reiter hat sein eigenes, gespeichertes Layout** (welche Panele sichtbar
sind und wo sie sitzen). Reiter wechseln = das ganze Panel-Set + Layout
wechselt.

- Pro Reiter: Menge sichtbarer Panele + deren `{x,y,w,h,z}` (Bruchteile).
- Panele lassen sich pro Reiter ein-/ausblenden.
- Beispiel: Design zeigt Werkzeuge/Ebenen/Farbpalette; Laser zeigt Ebenen +
  Laser-Control; Monitor (später) Job-Fortschritt/Status.
- **Standard-Layout je Reiter:** Jeder Reiter hat ein eingebautes, sinnvolles
  Default-Layout. Im Editier-Modus setzt ein „Zurücksetzen" den *aktuellen*
  Reiter auf sein Standard zurück (die anderen Reiter bleiben unberührt). Beim
  allerersten Start bzw. ohne gespeicherte Settings gilt das Standard.

### 3. Theming: „Frosted Depth"-Glas + zwei einstellbare Farben

- **Frosted-Depth-Glas:** Panele sind mehrschichtiges Frostglas mit echter
  Tiefe — starker Hintergrund-Blur, diagonaler Licht-Sheen, akzentgetönter
  Verlauf, Außenschatten + Akzent-Halo + innere Licht-Kante oben + Dunkelschatten
  unten. Der Canvas scheint gedämpft durch. Bedienelemente (Buttons, Kacheln)
  sind selbst durchscheinend statt harte Flächen; aktive Elemente glühen im
  Akzent. Das **Rauchglas selbst ist mit dem Akzent getönt** (dezent), nicht nur
  der Rand.
- **Layer-Kacheln:** Jede Ebene ist eine eigene Glaskachel. Die Layer-Farbe
  färbt linke/rechte Kante und zieht sich als dezenter Waschgang durch die
  Fläche; die Kachel trägt Modus, Speed, Min–Max-Leistung und drei Schalter
  (Air Assist, Aktiv=brennen, Zeigen=Objekte anzeigen).
- **Zwei wählbare Farben:**
  - **Akzentfarbe** — aktive Werkzeuge, Auswahl, Handles, Hervorhebungen.
  - **Button-Farbe** — Grundfläche der Buttons, getrennt einstellbar, damit man
    die **Sichtbarkeit/den Kontrast** der Bedienelemente an die Umgebung
    (heller/dunkler Arbeitsplatz, Glass-Look) anpassen kann.
- **Kräftigkeit je Farbe über einen Intensitäts-Regler:** Man wählt je einen
  **Farbton** (Akzent, Button) und stellt mit einem Slider „Kräftigkeit"
  (Sättigung/Helligkeit) ein, wie dezent oder knallig er wirkt. So trifft man
  „ruhig" bis „kräftig" ohne Farbtheorie. Das beantwortet die offene Frage „wie
  kräftig" durch einen eigenen Regler statt eines festen Werts.
- **Der Regler ist auf einen lesbaren Bereich geklemmt** (Entscheidung): Der
  Slider deckt **nicht** die vollen 0–100 % ab, sondern nur einen Korridor, in
  dem Text/Icons auf dem Glass-Hintergrund lesbar und die Buttons erkennbar
  bleiben (Richtwert Sättigung ~30–90 %, Helligkeit nicht bis an die Extreme).
  So kann man sich nicht in Unlesbarkeit oder unsichtbare Buttons regeln. Die
  genauen Grenzen werden beim Umbau am echten Glass-Look feinjustiert.
- **Umsetzung** über CSS-Variablen, die aus den Settings gesetzt werden — eine
  Quelle, überall wirksam. Neben den fertigen Farben (`--accent`, `--btn`) werden
  auch die H/S/L-Kanäle einzeln gesetzt (`--accent-h/-s/-l`, `--btn-h/-s/-l`),
  damit das Glas-Design beliebige Transparenzen aus der Theme-Farbe bauen kann.
- **Bedient wird das nicht über ein Settings-Menü, sondern über ein Flyout im
  Editier-Modus** (siehe §5): Farben/Intensität ändert man direkt, während man
  die Oberfläche umbaut, mit sofortiger Vorschau.

### 4. Arbeitsplatz + Settings-Persistenz (Charon-ready)

- In den Settings wird ein **Arbeitsplatzname** verankert (z. B. „Werkstatt-PC",
  „Laptop").
- **GUI-Settings** werden **lokal als JSON** gespeichert — pro Arbeitsplatz.
  Kein Charon nötig, um zu arbeiten (offline-first). Enthalten:
  - Reiter-Layouts (sichtbare Panele + Position/Größe **als Bruchteile**,
    siehe §1, je Reiter),
  - Akzentfarbe + ihre Intensität, Button-Farbe + ihre Intensität,
  - Arbeitsplatzname.
  Der Editier-Modus selbst ist **flüchtig** (wird nicht gespeichert).
- Die JSON-Struktur wird so gehalten, dass **Charon sie später übernehmen und
  pro Arbeitsplatz synchronisieren** kann. Charon speichert dann GUI-Settings
  nach Arbeitsplatzname; die lokale Datei bleibt der Fallback.

### 5. Editier-Modus (verstecktes Schloss)

Das Anpassen der Oberfläche (Panele verschieben/größen, ein-/ausblenden, Theming)
läuft über einen expliziten **Editier-Modus**, damit der Normalbetrieb ruhig und
unverrückbar bleibt.

- **Aktivierung über ein verstecktes Schloss-Symbol unten links.** Es ist im
  Normalbetrieb **unsichtbar** und erscheint erst, wenn die Maus in die Ecke
  fährt (Hover) — ein bewusst zurückhaltendes „Easter Egg", das die Oberfläche
  nicht unnötig unruhig macht. Klick schaltet den Editier-Modus um (offen ↔
  gesperrt).
- **Im Editier-Modus:**
  - Panele bekommen eine **Bounding-Box** wie eine selektierte Shape im Canvas
    (Akzent-Rahmen + Greifpunkte). Fläche ziehen verschiebt, Griffe skalieren —
    **frei, ohne Raster-Snap** (§1).
  - Panele lassen sich **ein-/ausblenden** (pro Reiter).
  - Ein **Theming-Flyout** erlaubt Akzent-/Button-Farbe + Intensität (§3) mit
    Live-Vorschau.
  - Ein **„Zurücksetzen"** stellt das Standard-Layout des aktuellen Reiters her
    (§2).
- **Außerhalb des Editier-Modus** sind Panele fixiert; kein versehentliches
  Verschieben.
- Änderungen werden beim Verlassen des Editier-Modus (oder laufend) in die
  GUI-Settings persistiert (§4).

## Invarianten

1. **Panel-Positionen sind relativ (Bruchteile des Fensters), nie feste Pixel.**
   Das ist der Kern gegen das Auflösungsproblem; frei positioniert, ohne Raster.
2. Jeder Reiter hält sein eigenes Layout; Umschalten verändert kein anderes.
3. Akzent- und Button-Farbe (samt Intensität) kommen aus den Settings (eine
   Quelle, CSS-Variablen), nicht hartkodiert.
4. GUI-Settings sind offline lokal persistent; Charon ist optionaler Sync,
   nie Voraussetzung.
5. Panele sind nur im Editier-Modus veränderbar; im Normalbetrieb fixiert.

## Konsequenzen

- Das frühere Frontend (feste `position:absolute`-Panele) wurde auf freie,
  relative Positionierung umgebaut: ein `PanelHost` positioniert Panele aus
  Bruchteil-Rects, im Editier-Modus mit Bounding-Box und freiem Drag/Resize.
- Ein **Settings-Modell** (`luxifer-core::ui_settings`, JSON) + Lade-/Speicherweg
  entstand (lokale Datei über Tauri-Commands; Struktur Charon-kompatibel).
- Theming ist über CSS-Variablen zentralisiert; „Frosted Depth"-Glas als
  Panel-Grundstil, Bedienelemente aus den Theme-Farben abgeleitet.
- Die Reiter Design/Laser existieren funktional; **Monitor** ist als Reiter
  angelegt, aber inhaltlich später gefüllt.

## Offen / nicht Teil dieser Entscheidung

- Feinschliff der Drag-Interaktion (z. B. Tastatur-Nudging) — später.
- Charon-Sync-Protokoll (eigener späterer Schritt); hier nur die JSON-Struktur
- Charon-Sync-Protokoll (eigener späterer Schritt); hier nur die JSON-Struktur
  vorbereiten.
- Monitor-Reiter-Inhalte.
