# ADR 0012: Charon als optionaler lokaler Koordinationsdienst

## Status

Akzeptiert — 2026-07-13.

## Kontext

Charon ist bisher nur ein leeres Workspace-Binary. Die Projekt- und
Asset-ADRs sehen ihn langfristig für Synchronisation und Koordination vor,
fordern aber zugleich, dass LuxiFer vollständig offline und ohne Server
arbeitsfähig bleibt.

Der erste Entwicklungsschritt soll auf demselben Rechner wie LuxiFer laufen.
Damit können Protokoll, Fehlergrenzen und Bedienung stabilisiert werden, bevor
Deployment, Authentifizierung oder ein Proxmox-Betrieb hinzukommen.

## Entscheidung

Charon beginnt als **optional aktivierter lokaler HTTP-Dienst**. Der erste
Meilenstein enthält ausschließlich Erreichbarkeit und Protokollaushandlung:

- Standardbindung: `127.0.0.1:3737`; keine Freigabe ins LAN;
- `GET /health` bestätigt nur die Prozessbereitschaft;
- `GET /api/v1/handshake` liefert JSON mit Serverversion, Protokollversion,
  Instanzkennung und expliziten Fähigkeiten;
- die native Anwendung erhält eine globale Charon-Einstellung mit Aktivierung,
  Basis-URL, Verbindungstest und verständlichem Status;
- die Application-Schicht besitzt Netzwerkzugriff und Fehlerübersetzung; egui
  stellt nur Draft und Ergebnis dar;
- ein nicht gestarteter oder nicht erreichbarer Charon beeinträchtigt weder
  Editor, Projekte noch Laserbetrieb.

Die erste Protokollversion ist `1`. Fähigkeiten werden als stabile String-IDs
gemeldet. Der erste Server meldet nur `health` und `handshake`; unbekannte
Fähigkeiten müssen von Clients ignoriert werden.

## Invarianten

1. Charon steuert niemals direkt eine Maschine und besitzt keinen
   `MachineDriver`.
2. Charon ist kein Speicher-Wahrheitszentrum für den Editor. Lokale Dateien und
   der Core bleiben ohne Server vollständig nutzbar.
3. Netzwerk- und JSON-Details gelangen nicht in egui-Callbacks und nicht in
   `luxifer-core`.
4. Eine Bindung außerhalb des Loopback-Interfaces ist später eine bewusste
   Betriebsentscheidung mit eigener Authentifizierungs- und TLS-Grenze.
5. Handshake-Kompatibilität wird über die Protokollversion entschieden, nicht
   über die Charon-Binaryversion.

## Nicht Teil dieses Meilensteins

- Projekt-, Versions-, Profil- oder GUI-Settings-Synchronisation;
- Assetübertragung und Deduplizierung;
- Benutzerkonten, Tokens, TLS, Discovery oder Fernzugriff;
- Queueing, Maschinen-Sessions oder Jobübertragung;
- Proxmox-, Container- oder Systemdienst-Deployment.

## Nächste Schritte

1. Lokalen Server und serialisierbares Handshake-Modell implementieren.
2. UI-unabhängigen Charon-Client in `luxifer-application` ergänzen.
3. Persistente Charon-Konfiguration und Live-Verbindungstest in den globalen
   Einstellungen anbinden.
4. Erst nach realer lokaler Nutzung den ersten Synchronisationsfall separat
   entscheiden.

## Umsetzungsstand

Der erste Meilenstein ist umgesetzt:

- Charon bindet standardmäßig und erzwungen an `127.0.0.1:3737`;
- Health und Handshake antworten mit JSON und wurden gegen einen real
  gestarteten lokalen Prozess geprüft;
- der Client liegt in `luxifer-application`, validiert URL, HTTP-Status,
  Serverkennung und Protokollversion und übersetzt Fehler in `AppError`;
- Aktivierung, URL und Verbindungstest liegen in der globalen
  Charon-Einstellungssektion; alte Settings erhalten sichere Defaults.

Noch offen ist bewusst jede Form der Synchronisation.
