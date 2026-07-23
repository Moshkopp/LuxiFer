# GRBLHAL-Roadmap

- Stand: 2026-07-23
- Architektur: [ADR 0024](../adr/0024-grblhal-transport-und-protokollgrenze.md)
- Ziel: GRBLHAL über Serial und später Ethernet sicher und vollständig betreiben

Dieses Dokument ist die veränderliche Umsetzungs- und Abnahmeliste. Dauerhafte
Architekturentscheidungen gehören in ADR 0024; erledigte Arbeit wird hier beim
zugehörigen Commit nachgeführt.

## Statuslegende

- `[x]` implementiert und automatisiert geprüft
- `[~]` Grundlage vorhanden, noch nicht für den Maschinenbetrieb vollständig
- `[ ]` offen
- **Hardware-geprüft** bedeutet ausdrücklich: am Controller ausgeführt und mit
  dem erwarteten Ergebnis beobachtet

## Unveränderliche Schichtgrenze

| Schicht | Verantwortung |
|---|---|
| Core | Geräteunabhängige Absichten und Datentypen, z. B. Jog, Home, Stop und JobPlan |
| Application | Verbindungs- und Aktionslebenszyklus, Worker, Fehlerabbildung und Fortschritt |
| Treiber | GRBL-Befehle, Zustände, Alarme, G-Code und Protokollsemantik |
| Serial-Infrastruktur | Betriebssystem-Ports und protokollfreier serieller Zugriff |
| GUI | Absichten auslösen und von der Application gelieferte Zustände darstellen |

Kein `$`-, `G`-, `M`- oder GRBL-Echtzeitbefehl darf in Application oder GUI
entstehen. Dieselbe Core-Absicht muss von Ruida und GRBL unabhängig übersetzt
werden können.

## Aktueller Stand

- [x] Gemeinsame serielle Portauflistung mit USB-Metadaten
- [x] Port- und Baudratenauswahl im Laserprofil
- [x] USB-Gerät über VID/PID und Seriennummer trotz geändertem `ttyACM*`-Pfad wiederfinden
- [x] Serielle GRBL-Verbindung und Handshake
- [x] Identifikation über `$I`
- [x] Statuslesen über `?`
- [x] GRBL-Statusparser für Maschinen- und Arbeitsposition
- [x] Verschiebbare Geräte-Konsole mit pausierbarem Statuslesen und Einzelbefehlen
- [x] Unveränderte automatische Statusmeldungen in der Konsole zusammenfassen
- [x] Verbindungsverlust nach zwei aufeinanderfolgenden Kommunikationsfehlern
- [x] Hub-Lease bei bestätigtem Verbindungsverlust freigeben
- [~] Bekannte Alarmcodes werden verständlich erklärt; Hardwareabnahme steht aus
- [x] Controller-Unlock (`$X`) ist geräteneutral angebunden und am nackten ESP32-S3 geprüft
- [~] Homing (`$H`) ist geräteneutral angebunden; Zustandsfolge und Hardwareabnahme fehlen
- [~] G-Code mit konservativem RX-Zeichenfenster senden und Quittungen zuordnen
- [ ] GRBL-Treiber für den Betrieb einer angeschlossenen Maschine freigegeben

Bereits hardwaregeprüft wurde ausschließlich der sichere Leseweg am nackten
ESP32: Öffnen, Handshake, Identifikation und Status. Es wurden dabei keine
Bewegungs- oder Laserbefehle gesendet.

## Phase 1 – Sichere Maschinensteuerung

| Aufgabe | Status | Hauptschicht | Abnahme |
|---|---:|---|---|
| Alarmzustand verständlich darstellen | [x] | Treiber → Application | Bekannte Alarmcodes werden erklärt; unbekannte bleiben mit Rohcode sichtbar |
| Controller entsperren | [x] | Treiber | Geräteunabhängige Absicht erzeugt ausschließlich im GRBL-Treiber `$X` |
| Homing | [~] | Treiber | `home()` erzeugt `$H`, Statusfolge wird erkannt, Fehler werden gemeldet |
| Schritt-Jog | [ ] | Treiber | `jog_axis(... Step)` erzeugt korrektes `$J=...` für X/Y/Z/U |
| Dauer-Jog | [ ] | Treiber | Halten bewegt kontrolliert, Loslassen beendet die Bewegung sicher |
| Jog-Abbruch | [ ] | Treiber | Echtzeit-Abbruch bleibt auch während einer Bewegung erreichbar |
| Feed Hold | [ ] | Treiber | Pause verwendet den GRBL-Echtzeitbefehl und bestätigt den Hold-Status |
| Fortsetzen | [ ] | Core-Vertrag + Treiber | Geräteunabhängige Resume-Absicht ist definiert und GRBL-spezifisch umgesetzt |
| Soft-Reset | [ ] | Treiber | Reset ist erreichbar, Zustand danach wird neu eingelesen |
| Sofort-Stopp | [ ] | Treiber/Transport | Stop ist auch während Streaming jederzeit erreichbar und schaltet sicher ab |

### Sicherheitsabnahme Phase 1

- Jog und Home senden niemals einen Laser-Einschaltbefehl.
- Loslassen, Fokusverlust und Verbindungsfehler beenden einen Dauer-Jog.
- Stop und Reset dürfen nicht hinter einem blockierten Job-Stream warten.
- Jeder gesendete und empfangene Steuerbefehl ist in der Konsole nachvollziehbar.
- Hardwaretests beginnen ohne angeschlossene Laserquelle und mit niedriger
  Geschwindigkeit; die Freigabe jeder Stufe wird hier dokumentiert.

## Phase 2 – Unterbrechbare Jobübertragung

| Aufgabe | Status | Hauptschicht | Abnahme |
|---|---:|---|---|
| Transport-Worker mit Steuerkanal | [~] | GRBL-Treiber | Worker besitzt den Port exklusiv; Status und Stop sind priorisiert, Pause fehlt noch |
| Kontrollierter Startzustand | [x] | Treiber | Start wird in Alarm, Door und unzulässigem Hold abgelehnt |
| Fortschritt | [ ] | Treiber → Application | Bestätigte Befehle und Gesamtzahl werden geräteunabhängig gemeldet |
| Sofortiger Abbruch | [~] | Treiber | grblHAL nutzt priorisiert `0x19`, Mini-GRBL `0x18`; Hardwareabnahme für `0x19` fehlt |
| Sicheres Laser-Aus | [ ] | Treiber | Erfolg, Fehler und Abbruch enden garantiert mit ausgeschalteter Laserleistung |
| Pufferverwaltung | [~] | Treiber | Controllerpuffer wird schneller als Stop-and-wait genutzt, ohne Überlauf |
| Übertragungsende erkennen | [ ] | Treiber | „Alle Zeilen bestätigt“ ist von „Maschine Idle“ getrennt sichtbar |
| Verbindungsabbruch im Job | [ ] | Treiber/Application | Job endet als Fehler, UI und Lease wechseln in sicheren Zustand |
| Wiederverbindung nach Fehler | [ ] | Application | Kein alter Job wird still fortgesetzt; neuer Start ist ausdrücklich nötig |

### Sicherheitsabnahme Phase 2

- Ein physischer Verbindungsabbruch darf keinen Erfolg melden.
- Die GUI bleibt während des vollständigen Jobs bedienbar.
- Pause und Stop haben Vorrang vor weiteren G-Code-Zeilen.
- Ein Fehler enthält die betroffene Zeile und die originale GRBL-Antwort.
- Erst nach bestandenem Trockenlauf ohne Laserquelle folgt ein Test mit realer
  Maschine und sicher begrenzter Leistung.

## Phase 3 – Koordinaten und Bedienworkflow

| Aufgabe | Status | Hauptschicht | Abnahme |
|---|---:|---|---|
| Absolute Startreferenz | [ ] | Core/Application/Treiber | Vorschau und gesendete Maschinenkoordinaten stimmen überein |
| Aktuelle Position | [ ] | Application/Treiber | Relativer Start verwendet einen frisch gelesenen Stand |
| Werkstücknullpunkt | [ ] | Treiber/Application | GRBL-Arbeitskoordinaten werden korrekt gelesen und verwendet |
| Gespeicherte Nullpunkte | [ ] | Application | Bestehendes geräteneutrales Nullpunktmodell funktioniert unverändert mit GRBL |
| Ursprung anfahren | [ ] | Treiber | Geräteunabhängige GoOrigin-Absicht wird sicher übersetzt |
| Rahmenfahrt ohne Laser | [ ] | Treiber | Jobgrenze wird abgefahren; Laserleistung bleibt nachweislich aus |
| Bettgrenzen | [ ] | Core/Application | Jobs und Zielpunkte außerhalb des Profils werden vor dem Senden abgelehnt |
| GRBL-Soft-Limits | [ ] | Treiber | Controllerzustand und Studio-Prüfung widersprechen sich nicht still |

## Phase 4 – GRBL-Konfiguration

| Aufgabe | Status | Hauptschicht | Abnahme |
|---|---:|---|---|
| Einstellungen mit `$$` lesen | [ ] | Treiber | Parameter werden strukturiert statt als GUI-geparster Text geliefert |
| Parameterbeschreibungen | [ ] | Treiber | Bekannte Schlüssel haben Namen, Einheit und sicheren Wertebereich |
| Einstellungen schreiben | [ ] | Treiber | Nur explizit bestätigte Änderungen werden geschrieben und gegengelesen |
| Achsenauflösung | [ ] | Treiber | `$100` ff. erscheinen im bestehenden Controller-Workflow |
| Geschwindigkeit/Beschleunigung | [ ] | Treiber | `$110`/`$120` ff. werden korrekt zugeordnet |
| Laser Mode | [ ] | Treiber | `$32` wird gelesen, verständlich angezeigt und vor Jobs validiert |
| Maximalleistung | [ ] | Treiber | `$30` wird beim Mapping der Jobleistung berücksichtigt |
| Firmwareinformationen | [~] | Treiber | `$I` wird bereits gelesen; strukturierte Anzeige und `$G` fehlen |

Schreibzugriffe auf Controllerparameter erhalten vor der Hardwarefreigabe
separate Parser-, Wertebereichs- und Roundtrip-Tests.

## Phase 5 – Ethernet

| Aufgabe | Status | Hauptschicht | Abnahme |
|---|---:|---|---|
| Transportart im Profil | [ ] | Core/Application/GUI | Serial und Ethernet sind explizit auswählbar |
| TCP-/WebSocket-Transport | [ ] | gemeinsame Infrastruktur/Treiber | Verbindungsweg ist austauschbar, GRBL-Parser bleibt derselbe |
| Verbindungsverlust | [ ] | Application | Dieselbe Lifecycle-Semantik wie bei Serial |
| Funktionsparität | [ ] | Treiber | Jog, Stop, Status und Job verhalten sich über beide Transporte gleich |

## Definition of Done pro Aufgabe

Eine Aufgabe wird erst `[x]`, wenn alle zutreffenden Punkte erfüllt sind:

1. Die Schichtgrenze aus ADR 0024 ist eingehalten.
2. Parser oder Fachverhalten besitzen automatisierte Tests.
3. Fehler- und Abbruchpfade sind getestet, nicht nur der Erfolgsfall.
4. Workspace-Tests, Formatierung und Clippy sind sauber.
5. Bei Hardwarewirkung ist der sichere Hardwaretest dokumentiert.
6. Diese Roadmap wurde im selben Commit aktualisiert.

## Hardwareprotokoll

- 2026-07-23, nackter ESP32-S3 ohne angeschlossene Peripherie:
  automatisches Statuslesen in der MkStudio-Konsole pausiert, die für diesen
  Firmware-Build gültige Reset-Invertierung mit `$14=1` gesetzt und anschließend
  über die geräteneutrale Unlock-Aktion `$X` gesendet. Der Controller wechselte
  von `Alarm` nach `Idle`. Es wurden weder Homing noch Bewegung oder
  Laserkommandos ausgeführt.
- 2026-07-23, derselbe nackte ESP32-S3: gepuffertes Streaming mit einem
  konservativen 116-Byte-Zeichenfenster hardwaregeprüft. Das Testprogramm
  verwendete ausschließlich `S0`, eine virtuelle Bewegung von maximal 2 mm und
  endete bestätigt im Zustand `Idle`. Es wurde keine Laserleistung angefordert.

## Nächster Arbeitsblock

1. Das Zeichenfenster in einen priorisierten Transport-Worker für Pause und Stop überführen.
2. Homing erst mit angeschlossenen Endschaltern sicher hardwareprüfen und die Statusfolge erfassen.
3. Schritt-Jog und sicheren Jog-Abbruch implementieren.
