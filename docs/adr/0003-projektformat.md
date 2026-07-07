# ADR 0003: Projektformat, Speichern/Laden & Asset-Bibliothek

## Status
Akzeptiert — 2026-07-07

## Kontext

Zum Testen und Arbeiten muss man Projekte **speichern und wieder laden** können.
Das Core-Fundament steht (`luxifer/core/src/project.rs`: `ProjectFile`,
`save_to_dir`, `load`, `list_projects`), aber es fehlen Tauri-Commands und
Frontend. Zugleich soll das Format so gebaut sein, dass **Charon** es später pro
Arbeitsplatz **synchronisieren** kann (Invariante: Charon ist optional, steuert nie
eine Maschine, ist nie Voraussetzung für lokale Arbeit).

Zwei Erkenntnisse prägen die Entscheidung:

- **Identität über Umbenennen hinweg.** Der bisherige `ProjectFile` nutzt den
  Ordnernamen als Identität. Für Sync ist das fragil (Umbenennen = neues Projekt,
  zwei Rechner kollidieren). Es braucht eine stabile ID + Zeitstempel.
- **Assets gehören nicht ins Projekt.** Bilder/Fonts/DXF/SVG mehrfach pro Projekt
  zu kopieren war ThorBurns Import-Fehler. Sie gehören in eine **zentrale,
  projektübergreifende Bibliothek**; Projekte verlinken nur per Referenz.

## Entscheidung

### 1. Projektformat (versionierte JSON-Hülle mit stabiler Identität)

`ProjectFile` bekommt (alle neuen Felder `#[serde(default)]`, damit alte Dateien
weiter laden):

- `id: String` — stabile ID, bei Erstellung erzeugt, unveränderlich über
  Umbenennen. Erzeugt durch eigene `gen_id()` (Zeit + Zufall), **kein Fremd-Crate**.
- `created_at`, `modified_at` — ISO-8601 (UTC), über `std::time`.
- `description: String`, `tags: Vec<String>` (`tags` existiert bereits).
- `asset_refs: Vec<String>` — Liste von Asset-IDs, **vorerst leer** (vorbereitet).
- `versions: Vec<VersionInfo>` — Historie fester Stände.
- Aktueller Arbeitsstand (`bed`, `layers`, `shapes`) wie bisher.

`VersionInfo { id, created_at, note }`. Thumbnails liegen als **Datei** neben dem
Snapshot (nicht im JSON), damit `projekt.luxi` schlank bleibt.

### 2. Ordnerstruktur auf Platte

```
<data_root>/
  Projekte/
    <Name>/
      projekt.luxi        aktueller Stand + Metadaten (id, Zeitstempel,
                          Beschreibung, tags, asset_refs [], versions [])
      versions/
        <version-id>.luxi Geometrie-Snapshot der festgehaltenen Version
        <version-id>.png  Thumbnail dieser Version
  Assets/                 (später, mit Import) zentrale Bibliothek,
                          projektübergreifend, per ID/Content-Hash
```

`asset_refs` verweist auf `Assets/`, kopiert nie. Der Store selbst kommt mit dem
Import (eigene ADR); hier nur das Format-Feld.

### 3. Speicher-Workflows (GUI)

- **Neues Projekt** — ausgelöst über **Strg+N** oder den **„Neu"-Button** im
  Projekt-Reiter. Leert die Zeichenfläche und setzt den Projektkontext zurück
  (namenlos, `dirty = false`). Bei ungesicherten Änderungen greift zuvor der
  Unsaved-Guard (siehe unten).
- **Strg+S**: namenloses Projekt → Projekt-Reiter öffnet sich (Name/Beschreibung/
  Tags ausfüllen, speichern). Benanntes Projekt → still speichern (überschreibt
  Arbeitsstand), Toast „Gespeichert ✓ · Shift+Strg+S legt eine Version an".
- **Shift+Strg+S**: neue **Version** (bewusster Snapshot mit eigenem Thumbnail).
- **Datenschutz**: Neu/Öffnen bei ungesicherten Änderungen (`AppState::dirty`) →
  Nachfrage „Verwerfen / Speichern / Abbrechen". Gilt **auch für ein namenloses
  Projekt** (verklickter „Neu"-Button darf keinen Entwurf verlieren). Ist das
  Projekt noch namenlos, heißt die Speichern-Option „Speichern unter…" und öffnet
  den Projekt-Reiter zum Benennen (statt still zu überschreiben).
- **Start**: App startet leer im Designer (wie „Neu", aber ohne Guard); Toast
  „Zuletzt: ‹Name›" (Öffnen/Dismiss), Anker `last_project_id` in den GUI-Settings
  (ADR 0002).

### 4. Projekt-Reiter als Browser (volle Body-Fläche)

Links Suchfeld + Liste (Name, Tags, „geändert"), rechts Detail-Panel: Thumbnail,
erstellt/geändert, Tags, Beschreibung, Versionsliste (je Thumbnail + laden),
Assets-Bereich („keine"), **Charon-Status** (ehrlich „offline — nicht verbunden",
bis Charon existiert). Aktionen oben: **Neu**, Speichern. Am gewählten Projekt:
Laden, Umbenennen, Löschen, Export.

### 5. Thumbnail im Frontend

Das Thumbnail wird im **Frontend** aus der vorhandenen Canvas-Zeichenlogik in ein
Offscreen-Canvas gerendert und als PNG an den Core gereicht. Reine Darstellung,
kein Wahrheits-Zustand → konform mit „Frontend zeichnet nur" (CLAUDE.md Regel 2).
Der Core speichert nur die gelieferten Bytes.

## Invarianten

1. **Identität = `id`, nicht der Ordnername.** Umbenennen ändert nie die `id`.
2. **Assets werden referenziert, nie ins Projekt kopiert.** Der zentrale Store ist
   die einzige Ablage für Bilder/Fonts/DXF/SVG.
3. **Charon ist nie Voraussetzung.** Speichern/Laden funktioniert vollständig
   offline; der Charon-Status ist reine Anzeige.
4. **Format ist vorwärts-tolerant.** Neue Felder mit serde-`default`; alte Dateien
   laden ohne Migration.
5. Die **Fachlogik (Format, Versionen, Speichern) liegt im Core** und ist ohne UI
   testbar (CLAUDE.md Regel 1).

## Konsequenzen

- Charon kann später per `id` + `modified_at` Projekte abgleichen und geteilte
  Assets nur einmal übertragen.
- Der `last_project_id`-Anker erweitert die GUI-Settings (ADR 0002).
- Thumbnails kosten je Version eine kleine PNG-Datei — bewusst, für die visuelle
  Versionsliste.

## Nicht Teil dieser Entscheidung

- **Import** (Bilder/Fonts/DXF/SVG) und der **eigentliche Asset-Store** — eigene ADR.
- **Charon-Netzwerkprotokoll** — Charon bleibt vorerst leer.
- **Auto-Save** des Arbeitsstands (nur vorgemerkt, nicht jetzt).
