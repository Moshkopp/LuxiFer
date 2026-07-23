# MkStudio

> [!WARNING]
> **MkStudio befindet sich in einem frühen, unreifen Entwicklungsstand.**
> Die Software ist nicht für den produktiven Maschinenbetrieb freigegeben.
> Fehler können unerwartete Bewegungen, unkontrollierte Laserleistung,
> Sachschäden und schwere gesundheitliche Schäden verursachen.
>
> **Jede Nutzung der Software sowie jeder Anschluss und Betrieb von Hardware
> erfolgt ausschließlich auf eigene Verantwortung und eigenes Risiko.**
> MkStudio ersetzt weder geeignete Schutztechnik noch Not-Aus, Einhausung,
> Absaugung, Schutzbrille, Aufsicht oder die Sicherheitsvorgaben des
> Maschinen- und Laserherstellers.

MkStudio ist eine freie, native Anwendung für Entwurf, Jobvorbereitung und
Steuerung von Laser- und CNC-nahen Controllern. Das Projekt wird aktiv
entwickelt und legt besonderen Wert auf eine klare Trennung zwischen
Oberfläche, Anwendungslogik, geräteunabhängigem Kern und konkreten
Maschinentreibern.

## Entwicklungsstand

MkStudio ist derzeit experimentell. Es gibt noch keine allgemeine Freigabe für
den Betrieb einer angeschlossenen Maschine oder Laserquelle.

| Bereich | Stand |
|---|---|
| Native Editoroberfläche | aktiv in Entwicklung |
| Ruida | eigener Treiber und eigener Transport, aktiv in Entwicklung |
| grblHAL | Serial, Konsole, gepuffertes Streaming, Live-Status und Stop in Entwicklung |
| Mini-/klassisches GRBL | gemeinsame GRBL-Familie mit kompatibler Stop-Strategie |
| FluidNC | als weitere GRBL-Familienstrategie geplant |
| Ethernet für GRBL-Familie | geplant |

Hardwaretests erfolgen stufenweise und zunächst ohne angeschlossene
Laserquelle. Der aktuelle technische Stand und die dokumentierten
Hardwareprüfungen stehen in der
[GRBLHAL-Roadmap](docs/roadmap/grblhal.md).

## Sicherheit

Vor jedem Hardwaretest müssen mindestens folgende Punkte erfüllt sein:

- Laserquelle abklemmen oder sicher auf `S0` beziehungsweise null Leistung
  begrenzen, solange der jeweilige Test keine Laserleistung erfordert.
- Physischer Not-Aus und geeignete Trennmöglichkeit müssen erreichbar sein.
- Maschine niemals unbeaufsichtigt betreiben.
- Arbeitsbereich freihalten und unerwartete Achsbewegungen einkalkulieren.
- Einhausung, Absaugung sowie geeigneten Augen- und Brandschutz verwenden.
- Konfiguration, Koordinatensystem, Endschalter und Leistungsgrenzen vor dem
  Start unabhängig kontrollieren.

Ein erfolgreicher Test an einem Controller ist keine Freigabe für andere
Firmwarestände, Elektronik, Maschinen oder Laserquellen.

## Bauen und starten

Voraussetzung ist eine aktuelle stabile
[Rust-Toolchain](https://www.rust-lang.org/tools/install).
Die Entwicklung und die bisherigen Hardwaretests erfolgen unter Linux. Andere
Plattformen sind noch nicht als unterstützte Zielsysteme dokumentiert.

```bash
cargo build --workspace
cargo test --workspace
cargo run --release -p studio
```

Ein Release-Build der Anwendung entsteht mit:

```bash
cargo build --release -p studio
```

Je nach Betriebssystem können zusätzliche native Pakete für Fenster,
Grafikbeschleunigung und serielle Geräte erforderlich sein.

## Architektur

```text
studio/native       GUI und Darstellung
        ↓
studio/application  Anwendungsfälle und Gerätelebenszyklus
        ↓
studio/core         geräteunabhängige Modelle und Absichten
        ↓
studio/drivers      Ruida- und GRBL-Treiberfamilie
```

Die GUI erzeugt keine GRBL-, Ruida- oder seriellen Protokollbefehle. Ruida
bleibt ein vollständig eigener Treiber. grblHAL, Mini-/klassisches GRBL und
später FluidNC teilen nur die gemeinsamen Teile ihrer Protokollfamilie und
erhalten getrennte Strategien für tatsächliche Unterschiede.

Die dauerhaften Architekturentscheidungen liegen unter [docs/adr](docs/adr).

## Mitwirken

Fehlerberichte, nachvollziehbare Hardwarebeobachtungen, Dokumentation und
Beiträge sind willkommen. Bei maschinenwirksamen Änderungen bitte immer
Controller, Firmwarestand, Anschlussart und sichere Testbedingungen angeben.
Keine Tests mit realer Laserleistung voraussetzen oder ohne deutliche Warnung
vorschlagen.

## Lizenz

MkStudio ist unter der
[GNU General Public License Version 3](LICENSE), ausschließlich Version 3
(`GPL-3.0-only`), veröffentlicht.

Abhängigkeiten und eingebundene Fremdkomponenten behalten ihre jeweiligen
Lizenzen.
