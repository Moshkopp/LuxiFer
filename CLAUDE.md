# CLAUDE.md — Arbeitsrichtlinien für Studio (Rust / Tauri / Svelte)

Verbindlich. Regeln mit **MUSS** / **DARF NICHT** sind Architektur-Invarianten;
Abweichung nur mit ausdrücklicher Zustimmung des Nutzers. Antworten und
Code-Kommentare auf Deutsch.

## Projekt in einem Satz

Studio ist eine offline-first Desktop-Anwendung zur Laser-Steuerung.
**Die GUI ist das Produkt.** Hub (Rust) ist ein optionaler Koordinations-
Server und niemals Voraussetzung für lokale Arbeit.

## Stack (Native-only seit ADR 0011)

- **studio-core** (Rust): Fachmodell, Geometrie, Layer/Farbe, Undo,
  Projektformat und Job-Kompilierung. **Einzige fachliche Quelle der Wahrheit.**
- **studio-application** (Rust): UI-unabhängige Anwendungsfälle, I/O-
  Koordination, Fehlergrenze und Geräte-Lebenszyklen.
- **studio** (Rust): winit + wgpu + egui. Fenster, Eingaben, Rendering
  und kurzlebiger Präsentationszustand.
- **Hub** (Rust): optionaler Koordinationsserver; niemals Voraussetzung für
  lokale Arbeit und niemals Maschinensteuerung.

## Verzeichnisse

| Pfad | Inhalt |
|------|--------|
| `studio/core/` | Rust-Core (UI-frei, testbar) |
| `studio/application/` | UI-unabhängige Anwendungsfälle und Fehlergrenze |
| `studio/native/` | Produktive native Desktop-GUI |
| `studio/drivers/` | Maschinenprotokolle hinter Application-Schnittstellen |
| `hub/` | Optionaler Koordinationsserver |
| `docs/referenz/` | ThorBurn-Analyse + Funktions-Worksheet (Bauplan) |
| `nur zur Referenu/` | Altes ThorBurn-Projekt — **nur Referenz, gitignored** |

## Architektur-Invarianten

1. **Fachlogik gehört in `studio-core`** (Rust), nicht in die native UI.
   Geometrie, Hit-Test, Bounds, Skalierung, Layer/Farbe, Undo, Job sind im Core
   und dort testbar. Faustregel: Was ohne UI testbar sein sollte, gehört in den
   Core. **Keine Canvas-Fachlogik doppelt im Frontend** (das war ThorBurns
   Fehler).
2. Native zeichnet und übersetzt Eingaben in Application-Aufrufe. Vollständige
   Anwendungsfälle, Persistenz und Geräte-Lebenszyklen gehören nach
   `studio-application`; Native hält keinen zweiten fachlichen Wahrheitszustand.
3. **Farbe = Layer = Parametersatz, automatisch verwaltet.** Der Nutzer legt NIE
   manuell einen Layer an. Farbe klicken → `AppState::activate_color` (bei
   Auswahl Shape in Farb-Layer verschieben, sonst `pending_color` merken); leere
   Layer werden über `remove_empty_layers` automatisch entfernt. Siehe
   docs/referenz/01-thorburn-analyse.md §1.5.
4. **Undo ist Snapshot-basiert** (`push_undo` vor jeder mutierenden Aktion),
   nicht Command-basiert.
5. **Hub steuert niemals eine Maschine** und ist nie Voraussetzung für lokale
   Arbeit.

## Referenz (ThorBurn)

6. Aus `nur zur Referenu/` und den Referenz-Dokumenten wird **kein Code
   kopiert.** Nur analysieren, wie eine Funktion gebaut war, und im aktuellen
   Stil sauber neu implementieren. Der Bauplan (Reihenfolge M1–M7) steht in
   `docs/referenz/02-funktions-worksheet.md`.

## Build, Test, Format

```bash
# Rust (aus Repo-Wurzel)
cargo build
cargo test        # müssen grün sein; neue Core-Logik bekommt Tests
cargo clippy
cargo fmt

# Native Anwendung starten
cargo run -p studio
```

7. **Vor jedem Commit:** `cargo build` + `cargo test` grün, `cargo clippy` ohne
   Warnungen, `cargo fmt`. Neue Core-Fachlogik bekommt Tests.

## Native GUI-Styling (egui 0.35)

8. Neues und geändertes Widget-Styling **MUSS** den dokumentierten APIs von
   egui 0.35 folgen. Globale Gestaltung gehört in `egui::Style`/`Visuals`,
   lokale Abweichungen in einen begrenzten `Ui`-Style-Scope. Wiederkehrende
   Widget-Varianten verwenden `widget_style` und `Classes`, statt ihre Werte an
   jeder Aufrufstelle neu zusammenzustellen.
9. Zusammengesetzte Widget-Inhalte **MÜSSEN** nach Möglichkeit die egui-Atom-API
   (`IntoAtoms`, Tupel, `left_text`, `right_text`, `gap`) verwenden. Gemischte
   Schriften oder Icons dürfen nicht über manuell aufgebaute `LayoutJob`s
   ausgerichtet werden, wenn die Atom-API denselben Inhalt abbilden kann.
10. Größen, Abstände und Ausrichtung werden über `Spacing`, `interact_size`,
    Layout-Ausrichtung und `override_text_valign` festgelegt. Manuelles
    Pixel-Painting für normale Bedienelemente ist nur zulässig, wenn egui 0.35
    dafür keine dokumentierte Widget- oder Style-API anbietet; Canvas- und
    Overlay-Rendering bleibt davon unberührt.
11. Icons stammen aus dem eingebundenen, egui-0.35-kompatiblen Icon-Satz oder
    aus garantiert unterstützten egui-Glyphen. Ungeprüfte Unicode-Zeichen sind
    als Bedienelement-Icons nicht zulässig.

## Commits

- Sprache: english. Betreff im Imperativ, knapp; Body erklärt das *Warum*.
- Ein Commit = eine logische Änderung.
- Nur committen/pushen, wenn der Nutzer es verlangt.
- Footer: empty
