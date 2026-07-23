# ADR 0024: grblHAL-Transport und Protokollgrenze

Die veränderliche Umsetzungs- und Abnahmeliste liegt in der
[GRBLHAL-Roadmap](../roadmap/grblhal.md). Dieses ADR hält ausschließlich die
dauerhaften Architekturentscheidungen fest.

## Status

In Umsetzung — 2026-07-22.

## Kontext

Der GRBL-Treiber kompiliert den geräteneutralen `JobPlan` zu G-Code und
überträgt ihn über einen eigenen Transport-Worker. Das Laserprofil übergibt
Port, Baudrate und stabile USB-Identität strukturiert bis an den Treiber.

grblHAL kann dasselbe GRBL-Protokoll über verschiedene Streams anbieten. Der
erste produktive Transport ist USB-Serial; Ethernet soll später ergänzt werden,
ohne Parser, Streamingregeln oder GUI-Abläufe zu duplizieren.

Klassisches GRBL, grblHAL und FluidNC gehören zur selben Protokollfamilie,
unterscheiden sich aber in Fähigkeiten und Erweiterungen. Insbesondere verwendet
grblHAL für einen kontrollierten Echtzeit-Stopp `0x19`, während klassisches
GRBL dafür nur den Soft-Reset `0x18` bereitstellt.

## Entscheidung

1. `studio-core` definiert ausschließlich die geräteneutrale
   Verbindungskonfiguration und Status-/Fehlertypen. `MachineDriver::connect`
   erhält die strukturierte `Connection`, nicht einen kodierten Zielstring.
2. `driver-grbl` besitzt GRBL-Protokoll, Parser, Handshake, Flusskontrolle und
   konkrete Transporte. Ein Transport-Worker besitzt den seriellen Port während
   der gesamten Verbindung exklusiv und trennt normale Aufträge von
   priorisierten Echtzeitabsichten.
3. `studio-application` erzeugt den Treiber, koordiniert seinen Lebenszyklus,
   übersetzt Fehler und führt blockierende Abfragen außerhalb des UI-Threads
   aus. Sie kennt keine GRBL-Zeilen und keine serielle Bibliothek.
4. `studio/native` bearbeitet nur Profile und löst Application-Aktionen aus.
   Die GUI öffnet keine Ports, sendet keine GRBL-Kommandos und parst keine
   Controllerantworten.
5. Ein späterer Netzwerktransport implementiert dieselbe interne
   GRBL-Streamgrenze. Er verändert weder Core-Modell noch GUI-Workflow.
6. GRBL-Dialekte werden als Strategien innerhalb der GRBL-Treiberfamilie
   umgesetzt, nicht als kopierte Kompletttreiber. Das Profil wählt die
   Strategie ausdrücklich; eine allgemeine `Grbl 1.1f`-Begrüßung reicht nicht
   zur sicheren automatischen Erkennung.
7. Die geräteneutrale Stop-Absicht wird intern übersetzt: grblHAL verwendet
   `0x19`, Mini-/klassisches GRBL `0x18`. FluidNC kann später eine eigene
   Strategie für Erkennung, Fähigkeiten, Transport und Sonderbefehle ergänzen.
   Ruida bleibt davon vollständig getrennt.

## Sicherheits- und Ablaufregeln

- Verbinden wartet auf eine gültige GRBL-Begrüßung beziehungsweise
  Identitätsantwort; ein lediglich erfolgreich geöffnetes Gerät gilt nicht als
  verbundener Controller.
- Status `?` ist ein Echtzeitkommando und wird nicht wie eine normale
  quittierte G-Code-Zeile behandelt.
- Status und Stop müssen einen laufenden Job über den priorisierten
  Echtzeitkanal erreichen, ohne auf dessen Treiber-Mutex zu warten.
- Normale Befehle werden erst nach `ok` als abgeschlossen betrachtet;
  `error:` und `ALARM:` werden typisiert an die Application gemeldet.
- Verbindungs- und Lesetimeouts sind begrenzt. Kein Geräte-I/O darf den
  Renderthread dauerhaft blockieren.
- Hardwaretests steigern die Wirkung stufenweise: zuerst reine Lesezugriffe,
  danach ausschließlich `S0` und begrenzte virtuelle Bewegung. Reale
  Laserleistung benötigt eine eigene Freigabe.

## Abnahme

- Port und Baudrate erreichen den GRBL-Treiber unverändert.
- Ruida bleibt über dieselbe strukturierte Trait-Grenze funktionsfähig.
- Parser und Streaminglogik sind ohne Hardware deterministisch testbar.
- Ein nackter ESP32-S3 kann dauerhaft verbunden werden; Begrüßung, `$I`,
  Status, gepuffertes `S0`-Streaming und grblHAL-Stop `0x19` sind
  hardwaregeprüft.
- Core und GUI besitzen keine Abhängigkeit auf das Serial-Crate.
