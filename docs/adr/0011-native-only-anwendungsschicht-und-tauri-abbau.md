# ADR 0011: Native-only-Anwendungsschicht und vollständiger Tauri-Abbau

## Status

Akzeptiert — 2026-07-12.

Präzisiert ADR 0010. ADR 0010 entscheidet die Render- und UI-Plattform
(`winit + wgpu + egui`). Dieser ADR entscheidet, wie die vorhandenen
Tauri-Funktionen ohne eine zweite, fehleranfällige Implementierung in die
native Anwendung überführt werden und wann Tauri entfernt werden darf.

## Kontext

Der native Spike hat die zentrale technische Annahme aus ADR 0010 bestätigt:
Der direkte Rust-/wgpu-Pfad ist auch bei großen Szenen sehr schnell. Die
sichtbare native Oberfläche ist jedoch noch kein funktional gleichwertiger
Editor:

- ein großer Teil der sichtbaren Aktionen ist noch nicht oder nur teilweise
  verdrahtet;
- mehrere vorhandene Abläufe verhalten sich fehlerhaft oder anders als in der
  bisherigen Anwendung;
- `native/src/app.rs` bündelt Fensterereignisse, Interaktion, Workflows,
  Rendering-Koordination und Anwendungsaktionen;
- Projekt- und Laserabläufe werden in `native/src/project.rs` und
  `native/src/laser.rs` erneut zusammengesetzt, obwohl entsprechende Abläufe im
  Tauri-Backend bereits existieren;
- das Tauri-Backend enthält nicht nur IPC-Adapter, sondern auch reale
  Anwendungslogik in `frontend/src-tauri/src/commands/`.

Eine Portierung Button für Button würde Tauri durch eine zweite, unvollständige
Implementierung ersetzen. Ein dauerhafter Parallelbetrieb zweier Frontends ist
ebenfalls nicht das Ziel: Die Svelte-/Tauri-Anwendung soll vollständig
entfallen.

`luxifer-core` bleibt die Quelle der Wahrheit für Fachzustand, Geometrie,
Layer, Transformationen, Jobplanung und persistente Modelle. Es soll aber weder
GUI-Zustand noch Betriebssystemdialoge oder konkrete Hardware-Lebenszyklen
übernehmen.

## Entscheidung

LuxiFer wird eine ausschließlich native Desktop-Anwendung. Svelte, WebView,
Tauri-IPC und das Tauri-Backend werden nach abgeschlossener Funktionsmigration
gelöscht.

Zwischen nativer GUI und `luxifer-core` wird eine UI-unabhängige Rust-
Anwendungsschicht eingeführt. Der vorläufige Crate-Name ist
`luxifer-application`:

```text
winit / egui / wgpu
        |
        v
luxifer-application
        |
        +--> luxifer-core
        +--> Projekt-/Asset-Speicher
        +--> Treiber und Geräte-Lebenszyklen
```

Die Anwendungsschicht ist keine IPC-Abstraktion und kein Versuch, Tauri zu
erhalten. Sie ist die testbare Grenze zwischen Darstellung/Interaktion und
vollständigen Anwendungsfällen.

### Verantwortlichkeiten

`luxifer-core` besitzt:

- Editor- und Projektmodelle;
- Geometrie, Auswahl, Transformationen und Layerregeln;
- Undo/Redo-Semantik;
- Import-/Bild-/Textalgorithmen;
- Vorschau- und Jobplanung;
- persistente Formate und fachliche Validierung.

`luxifer-application` besitzt:

- die laufende Editor-/Projekt-Sitzung;
- vollständige Anwendungsfälle wie Öffnen, Speichern, Versionieren, Import,
  Export und Jobstart;
- Koordination von Core, Assets, Projektablage und Treibern;
- ein einheitliches `AppError`-Fehlermodell;
- UI-unabhängige Ergebnis- und Statusmodelle;
- Ressourcen-Lebenszyklen, soweit sie nicht an winit/wgpu/egui gebunden sind.

`luxifer-native` besitzt:

- Fenster, Eventloop, Tastatur- und Mausereignisse;
- Kamera, GPU-Ressourcen, Texturen und Render-Caches;
- egui-Layout und kurzlebigen Präsentationszustand wie offene Dialoge,
  aktives Werkzeug und Drag-Vorschau;
- native Dateiauswahl und die Übersetzung einer Benutzeraktion in genau einen
  Anwendungsfall;
- die Darstellung von Erfolg, Fortschritt und `AppError`.

Nicht zulässig sind:

- Fach- oder Persistenzlogik in egui-Callbacks;
- Tauri-/JSON-ähnliches IPC innerhalb des nativen Prozesses;
- parallele Projekt-, Asset- oder Laserimplementierungen in Native und
  Application;
- dauerhaft duplizierte Geometrie-, Auswahl- oder Transformberechnungen;
- sichtbare, aktiv wirkende Bedienelemente ohne vollständigen Anwendungsfall.

### Migrationsregel

Die Migration erfolgt in vertikalen Funktionsschnitten. Für jeden Schnitt gilt:

1. bestehendes Verhalten und relevante Tauri-Commands inventarisieren;
2. Ablauf und Fehlerfälle als Tests festhalten;
3. Tauri-unabhängige Logik in Core oder Application verschieben;
4. Native vollständig anbinden, inklusive Fehler- und Leerzuständen;
5. den Schnitt gegen die festgelegten Akzeptanzkriterien prüfen;
6. ersetzte Tauri-Logik löschen oder ausdrücklich als noch benötigte Referenz
   markieren.

Ein Funktionsschnitt gilt nicht als migriert, wenn nur ein Button vorhanden ist
oder nur der Erfolgsfall manuell funktioniert.

### Übergangsregel für Tauri

Tauri bleibt während der Migration ausschließlich als lesbare
Referenzimplementierung und temporärer Verhaltensvergleich im Repository. Es
wird nicht mehr als zweite Produktlinie weiterentwickelt. Fehlerkorrekturen
werden grundsätzlich am zukünftigen Core-/Application-Pfad vorgenommen; eine
Tauri-Anpassung ist nur erlaubt, wenn sie zur Verifikation eines noch nicht
migrierten Schnitts nötig ist.

Tauri darf erst vollständig gelöscht werden, wenn die Abnahmekriterien der
Migrations-Taskliste erfüllt sind. Danach werden auch Node-/Svelte-Build,
WebView-Konfiguration, IPC-Datentransferobjekte und Tauri-spezifische
Dokumentation entfernt.

## Konsequenzen

### Positiv

- nur eine produktive GUI und nur ein ausführbarer Anwendungsweg;
- der schnelle native Renderer bleibt direkt mit Rust-Daten verbunden;
- Anwendungsfälle sind ohne Fenster, GPU oder WebView testbar;
- Fehler- und Zustandsübergänge werden konsistent statt pro Panel erfunden;
- Tauri kann am Ende vollständig und nachvollziehbar gelöscht werden.

### Kosten und Risiken

- vor weiterer UI-Breite ist ein struktureller Zwischenschritt nötig;
- die tatsächliche Funktionalität des Tauri-Backends muss vollständig
  inventarisiert werden;
- bestehende Native-Duplizierungen müssen teilweise verworfen oder extrahiert
  werden;
- während der Migration existieren zwei Oberflächen im Repository, obwohl nur
  Native das Ziel ist;
- ein zu großer `Application`-Typ könnte zum neuen Sammelmodul werden. Deshalb
  wird nach Verantwortlichkeiten (`editor`, `project`, `assets`, `laser`,
  `preview`) geschnitten, nicht nach einzelnen Buttons.

## Umsetzung

Die verbindliche Arbeits- und Übergabeliste steht in
[`docs/native_only_migration_tasks.md`](../native_only_migration_tasks.md).
Sie ist während der Umsetzung fortzuschreiben und enthält Status,
Abnahmekriterien, Prüfkommandos und die endgültige Löschliste.

