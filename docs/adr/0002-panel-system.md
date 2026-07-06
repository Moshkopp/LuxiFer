# ADR 0002: Panel-System (Raster-Layout, Reiter, Theming, Arbeitsplatz-Settings)

## Status
Vorschlag — 2026-07-06

## Kontext

Die GUI ist das Produkt. Bisher sind die Panele (Werkzeuge, Ebenen, Farbpalette,
Laser) mit **festen Pixel-Positionen** im Fenster verankert. Das erzeugt ein
konkretes Problem:

Der Arbeitsplatz hat **zwei Monitore mit unterschiedlicher Auflösung**
(FullHD 1920×1080 und WQHD 2560×1440). Pixel-verankerte Panele landen beim
Wechsel der Auflösung/des Monitors an der falschen Stelle oder außerhalb des
sichtbaren Bereichs — das „Positionier-Problem", das sich durch alle bisherigen
Versionen zog.

Diese ADR legt ein **auflösungsunabhängiges Panel-System** fest, dazu wählbares
Theming (Glassmorphism + Akzentfarbe), reiterbasierte Sichtbarkeit und
arbeitsplatzbezogene, lokal gespeicherte GUI-Settings (später über Charon
synchronisierbar).

## Entscheidung

### 1. Relatives Raster-Layout (löst das Auflösungsproblem)

Panele werden **nicht in Pixeln**, sondern in **Rasterzellen** positioniert. Das
Fenster ist ein Raster aus **Spalten × Zeilen** (z. B. 12 Spalten × 8 Zeilen).
Ein Panel belegt einen Zellbereich `{ col, row, colSpan, rowSpan }`.

- **Auflösungsunabhängig:** Die Zellengröße ergibt sich aus der aktuellen
  Fenstergröße (`Zellbreite = Fensterbreite / Spaltenzahl`). Dieselbe
  `{col,row,span}` sitzt auf FullHD und WQHD an der *gleichen relativen*
  Stelle — nie mehr außerhalb.
- **Snap:** Beim Verschieben/Größenändern rastet ein Panel auf ganze Zellen ein.
  Es gibt keine „krummen" Pixel-Positionen.
- **Größe passt Inhalt an:** Panele sind responsiv; der Inhalt fließt in die
  gegebene Zellfläche (Toolbar wird bei genügend Breite **1- oder 2-spaltig**,
  Listen scrollen bei Bedarf). `colSpan` steuert also unmittelbar das
  Erscheinungsbild.
- **Rastermaße (Spalten × Zeilen) sind ein Settings-Wert.** Standard 12 × 8,
  aber in den Settings frei anpassbar, damit man experimentieren kann. Die
  Spaltenzahl bestimmt die Feinheit — z. B. mehr Spalten = Toolbar kann 1 oder 2
  davon breit sein. Ändert man das Raster, bleiben die Panel-Zuordnungen
  relativ gültig (ggf. mit Clamping auf die neuen Grenzen).

Panele „schweben" weiter über dem Canvas (Canvas ist die vollflächige
Grundebene); das Raster bestimmt nur ihre Position/Größe, nicht ob sie den
Canvas verdrängen.

### 2. Reiter: Design / Laser / Monitor

Oben gibt es Reiter **Design**, **Laser**, **Monitor** (Monitor kommt später).
**Jeder Reiter hat sein eigenes, gespeichertes Layout** (welche Panele sichtbar
sind und wo sie im Raster sitzen). Reiter wechseln = das ganze Panel-Set +
Layout wechselt.

- Pro Reiter: Menge sichtbarer Panele + deren `{col,row,span}`.
- Panele lassen sich pro Reiter ein-/ausblenden.
- Beispiel: Design zeigt Werkzeuge/Ebenen/Farbpalette; Laser zeigt Ebenen +
  Laser-Control; Monitor (später) Job-Fortschritt/Status.
- **Standard-Layout je Reiter:** Jeder Reiter hat ein eingebautes, sinnvolles
  Default-Layout. Im Editier-Modus setzt ein „Zurücksetzen" den *aktuellen*
  Reiter auf sein Standard zurück (die anderen Reiter bleiben unberührt). Beim
  allerersten Start bzw. ohne gespeicherte Settings gilt das Standard.

### 3. Theming: Glassmorphism + zwei einstellbare Farben

- **Glassmorphism:** Panele mit Milchglas-Effekt (Hintergrund-Blur,
  Halbtransparenz, feiner heller Rand, weicher Schatten). Der Canvas scheint
  gedämpft durch.
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
- **Umsetzung** über CSS-Variablen (`--accent`, `--btn`), die aus den Settings
  gesetzt werden — eine Quelle, überall wirksam.
- **Bedient wird das nicht über ein Settings-Menü, sondern über ein Flyout im
  Editier-Modus** (siehe §5): Farben/Intensität ändert man direkt, während man
  die Oberfläche umbaut, mit sofortiger Vorschau.

### 4. Arbeitsplatz + Settings-Persistenz (Charon-ready)

- In den Settings wird ein **Arbeitsplatzname** verankert (z. B. „Werkstatt-PC",
  „Laptop").
- **GUI-Settings** werden **lokal als JSON** gespeichert — pro Arbeitsplatz.
  Kein Charon nötig, um zu arbeiten (offline-first). Enthalten:
  - Reiter-Layouts (sichtbare Panele + `{col,row,span}` je Reiter),
  - Akzentfarbe + ihre Intensität, Button-Farbe + ihre Intensität,
  - Arbeitsplatzname,
  - Rastermaße (Spalten × Zeilen).
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
  - Panele sind **verschiebbar und in der Größe änderbar** (Snap aufs Raster).
  - Panele lassen sich **ein-/ausblenden** (pro Reiter).
  - Ein **Theming-Flyout** erlaubt Akzent-/Button-Farbe + Intensität (§3) mit
    Live-Vorschau.
  - Ein **„Zurücksetzen"** stellt das Standard-Layout des aktuellen Reiters her
    (§2).
  - Optional die **Rastermaße** (Spalten × Zeilen, §1) zum Experimentieren.
- **Außerhalb des Editier-Modus** sind Panele fixiert; kein versehentliches
  Verschieben.
- Änderungen werden beim Verlassen des Editier-Modus (oder laufend) in die
  GUI-Settings persistiert (§4).

## Invarianten

1. **Panel-Positionen sind relativ (Rasterzellen), nie feste Pixel.** Das ist
   der Kern gegen das Auflösungsproblem.
2. Jeder Reiter hält sein eigenes Layout; Umschalten verändert kein anderes.
3. Akzent- und Button-Farbe (samt Intensität) kommen aus den Settings (eine
   Quelle, CSS-Variablen), nicht hartkodiert.
4. GUI-Settings sind offline lokal persistent; Charon ist optionaler Sync,
   nie Voraussetzung.
5. Panele sind nur im Editier-Modus veränderbar; im Normalbetrieb fixiert.

## Konsequenzen

- Das aktuelle Frontend (feste `position:absolute`-Panele) wird auf ein
  Grid-Layout-System umgebaut. Panele werden zu Grid-Kindern mit
  `{col,row,span}` und Drag/Resize-Snap.
- Ein **Settings-Modell** (JSON) + Lade-/Speicherweg entsteht (zunächst lokale
  Datei über einen Tauri-Command; Struktur Charon-kompatibel).
- Theming wird über CSS-Variablen zentralisiert; Glassmorphism als
  Panel-Grundstil.
- Die Reiter Design/Laser existieren funktional; **Monitor** wird als Reiter
  angelegt, aber inhaltlich später gefüllt.

## Offen / nicht Teil dieser Entscheidung

- Der Standard-Startwert der Rastermaße (12×8 als Vorgabe; anpassbar in den
  Settings — das ist bereits entschieden, siehe §1).
- Genaue Interaktion beim Verschieben (Drag-Handles, Kollision zweier Panele in
  derselben Zelle) — wird beim Umbau ausgearbeitet.
- Charon-Sync-Protokoll (eigener späterer Schritt); hier nur die JSON-Struktur
  vorbereiten.
- Monitor-Reiter-Inhalte.
