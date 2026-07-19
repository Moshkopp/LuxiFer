# ADR 0013: egui- und wgpu-Stack auf aktuellen Stand anheben

## Status

Umgesetzt — 2026-07-13. Automatisierte Abnahme, nativer Start-Smoke-Test und
visuelle Prüfung erfolgreich.

Ergänzt [ADR 0010](0010-nativer-renderer-wgpu.md) und
[ADR 0011](0011-native-only-anwendungsschicht-und-tauri-abbau.md). Die dort
entschiedene native Architektur bleibt unverändert; aktualisiert wird ihr
technisches UI- und Renderfundament.

## Kontext

Der native Studio-Renderer wurde am 2026-07-12 mit diesem Versionssatz
begonnen:

```toml
winit = "0.30"
wgpu = "22"
egui = "0.29"
egui-wgpu = "0.29"
egui-winit = "0.29"
```

Der Lockfile-Stand ist `egui 0.29.1`, `wgpu 22.1.0` und `winit 0.30.13`.
`egui 0.29.1` stammt aus Oktober 2024. Beim Aufbau des nativen Grundgerüsts
wurde dieser Stand ohne dokumentierte Kompatibilitäts- oder Produktanforderung
übernommen.

Zum Zeitpunkt dieser Entscheidung ist `egui 0.35.0` der aktuelle Stand.
`egui-wgpu 0.35.0` verwendet `wgpu 29` und bleibt mit `winit 0.30` kompatibel.
Ein isoliertes Update nur des UI-Crates ist daher nicht möglich: Studio
verwendet dieselben wgpu-Typen sowohl im eigenen Canvas-Renderer als auch an
der egui-Rendergrenze. Unterschiedliche wgpu-Hauptversionen würden getrennte,
nicht austauschbare Rust-Typen erzeugen.

Der native UI-Code ist bereits über zahlreiche Panels und Dialoge verteilt.
Mit jedem weiteren Funktionsschnitt wächst die Menge an Code, die später gegen
geänderte egui-APIs migriert werden müsste. Insbesondere neuere egui-Versionen
führen eine stärker `Ui`-orientierte Wurzel- und Panel-API ein; die bisher
verwendeten `Context`-, `SidePanel`- und `TopBottomPanel`-Einstiegspunkte sind
teilweise abgelöst oder als veraltet markiert.

Die Architektur aus ADR 0010/0011 begrenzt das Risiko: `studio-core` und
`studio-application` kennen weder egui noch winit oder wgpu. Der
Versionssprung betrifft damit die native Präsentations- und Rendergrenze, nicht
Projektformat, Fachlogik, Hub-Protokoll oder Maschinensteuerung.

## Entscheidung

Studio hebt den nativen UI- und Renderstack jetzt koordiniert auf folgenden
Zielstand an:

```toml
winit = "0.30"
wgpu = "29"
egui = "0.35"
egui-wgpu = "0.35"
egui-winit = "0.35"
```

Der Sprung erfolgt als eigener technischer Migrationsschnitt, bevor weitere
größere native UI-Funktionen hinzukommen. Es wird kein dauerhafter
Kompatibilitätslayer für egui 0.29 eingeführt.

Für die Migration gelten diese Grenzen:

1. `egui`, `egui-wgpu` und `egui-winit` werden immer gemeinsam auf derselben
   Version gehalten.
2. Die direkte `wgpu`-Abhängigkeit folgt der von `egui-wgpu` verwendeten
   Hauptversion, damit Canvas und UI denselben Device-, Queue-, Texture- und
   Render-Pass-Typ verwenden.
3. Veraltete egui-Einstiegspunkte werden auf die aktuelle API umgestellt, statt
   Warnungen oder Übergangsadapter dauerhaft zu übernehmen.
4. Anpassungen bleiben in `studio`. Eine durch das Upgrade ausgelöste
   Verschiebung von UI- oder Renderdetails in `studio-core` oder
   `studio-application` ist nicht zulässig.
5. Fachliche Änderungen, neue UI-Funktionen und visuelle Neugestaltung gehören
   nicht in diesen Migrationsschnitt. Sichtbare Abweichungen werden nur
   korrigiert, soweit sie durch den Versionssprung entstehen.
6. Der Rust-Mindeststand und alle transitive Plattformanforderungen des neuen
   Stacks werden vor Abschluss gegen Entwicklungs- und Zielsystem geprüft.

## Abnahmekriterien

Der Versionssprung gilt als abgeschlossen, wenn:

- der Workspace mit dem neuen Stack ohne Compilerfehler und ohne neue
  Warnungen baut;
- nicht gleichzeitig alte und neue Hauptversionen von egui oder wgpu im
  Abhängigkeitsbaum verbleiben;
- alle automatisierten Workspace-Tests erfolgreich sind;
- Start, Fenster-Resize, Skalierungsfaktor und egui-Repaint funktionieren;
- Maus- und Tastaturereignisse weiterhin korrekt zwischen Panels, Dialogen und
  Canvas getrennt werden;
- Canvas, Bilder, Auswahl-Overlay und egui in derselben Frame-Reihenfolge
  korrekt gerendert werden;
- Texturen und Asset-Thumbnails nach Erzeugung und Freigabe korrekt angezeigt
  werden;
- die vorhandenen Projekt-, Editor-, Laser- und Einstellungsoberflächen
  manuell auf Layout- oder Interaktionsregressionen geprüft wurden;
- die Aztec-Referenzdatei weiterhin flüssig gerendert und bewegt werden kann.

## Konsequenzen

### Positiv

- Der frisch entstandene native Renderer baut nicht weiter auf einem bereits
  beim Start deutlich veralteten UI-Stack auf.
- Spätere UI-Arbeit verwendet die aktuelle egui-API und muss nicht erneut über
  mehrere Hauptversionen migriert werden.
- Neuere Fehlerkorrekturen und Verbesserungen in egui, wgpu und deren
  Plattformintegration stehen Studio zur Verfügung.
- Der Versionssatz und seine Kopplung sind als bewusste Architekturentscheidung
  dokumentiert.

### Kosten und Risiken

- `wgpu 22` auf `wgpu 29` kann Änderungen an Surface-Konfiguration,
  Pipelines, Render-Pässen, Textur-Uploads und Ressourcen-Lebenszyklen
  erfordern.
- Die egui-Integration in Eventloop und Frame-Aufbau sowie bestehende Panels
  können API-Anpassungen benötigen.
- Kompiliererfolg allein deckt visuelle und interaktive Regressionen nicht ab;
  deshalb ist eine manuelle UI- und Renderprüfung Teil der Abnahme.
- Der neue Stack kann einen höheren Rust-Mindeststand oder aktualisierte
  Plattformabhängigkeiten verlangen.

## Folgeentscheidungen

Künftige Hauptversionssprünge werden bewusst und zeitnah bewertet. Ein neuer
Release wird nicht automatisch übernommen, aber auch nicht ohne dokumentierten
Grund über mehrere Hauptversionen aufgeschoben. Kleine, kompatible Updates
dürfen über den Lockfile-Updateprozess erfolgen; Änderungen an der Kopplung von
egui, wgpu und winit erfordern erneut eine explizite technische Prüfung.

## Umsetzungsstand

- `studio` verwendet `egui`, `egui-wgpu` und `egui-winit` jeweils in
  Version `0.35.0`, `wgpu` in Version `29.0.4` und weiterhin `winit 0.30.13`.
- Die egui-Wurzel wurde von `Context::run` auf `Context::run_ui` und die
  UI-Komposition auf die neue `Ui`- und `Panel`-API umgestellt.
- Entfernte Präsentations-APIs wie `Rounding`, `SelectableLabel` und die alten
  Paneltypen wurden ohne Kompatibilitätsadapter auf ihre aktuellen
  Entsprechungen migriert.
- Canvas-, Bild- und egui-Pipelines teilen sich dieselbe wgpu-29-Device-,
  Queue-, Surface- und Render-Pass-Grenze.
- Die neue Surface-Status-API behandelt Erfolg, suboptimale Frames,
  Rekonfiguration und vorübergehend nicht darstellbare Frames explizit.
- Rust `1.96.1` erfüllt den vom aktuellen egui-Stack verlangten Mindeststand.
- `cargo tree -p studio --depth 2` zeigt nur `egui 0.35.0` und
  `wgpu 29.0.4`; alte Hauptversionen verbleiben nicht im nativen Baum.
- `cargo test --workspace` ist vollständig erfolgreich: 352 Tests, keine
  Fehler.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ist
  ohne Warnungen erfolgreich.
- Ein nativer Start-Smoke-Test erreicht erfolgreich Fenster-, GPU- und erste
  Renderphase. Die anschließende visuelle Prüfung bestätigt geglätteter
  wirkende Canvas-Linien und angenehmere Slider. Einzelne Bedienelemente wirken
  gegenüber dem vorherigen Stand relativ größer; das Gesamtlayout ist
  akzeptiert, die Größenabweichungen bleiben als Detailbeobachtung für spätere
  UI-Feinabstimmung dokumentiert.
