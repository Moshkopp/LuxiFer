# Hub betreiben

Hub bleibt standardmäßig ausschließlich lokal erreichbar:

```bash
cargo run -p hub
```

## Internes Netzwerk / Proxmox

Eine Netzwerkbindung ist eine ausdrückliche Betriebsentscheidung. Hub hat
aktuell keine Benutzeranmeldung und kein TLS. Port `3737/tcp` darf deshalb nur
aus einem vertrauenswürdigen internen Netz erreichbar sein und darf nicht ins
Internet weitergeleitet werden.

Zum Test in einer Proxmox-VM oder einem LXC-Container:

```bash
HUB_BIND=0.0.0.0:3737 \
HUB_ALLOW_NETWORK=1 \
HUB_DATA_DIR=/var/lib/hub \
./hub
```

`0.0.0.0` lauscht auf allen IPv4-Interfaces. Studio verwendet als Hub-URL
die konkrete interne Adresse des Gasts, beispielsweise
`http://192.168.10.25:3737`.

Vor dem Einrichten des Dienstes kann die Erreichbarkeit von einem anderen
Rechner geprüft werden:

```bash
curl http://192.168.10.25:3737/health
curl http://192.168.10.25:3737/api/v1/handshake
```

## Installation als systemd-Dienst

Im Repository wird zuerst das Release-Binary gebaut und anschließend das
Installscript als root ausgeführt:

```bash
cargo build -p hub --release
sudo ./scripts/install-hub.sh
```

Das Script ist wiederholt ausführbar und aktualisiert eine bestehende
Installation. Es richtet Folgendes ein:

- Systembenutzer und -gruppe `hub`;
- Binary unter `/usr/local/bin/hub`;
- persistente Daten unter `/var/lib/hub`;
- Konfiguration unter `/etc/hub/hub.env`;
- gehärtete systemd-Unit `hub.service`.

Abweichende Adressen, Datenpfade oder ein separat übertragenes Binary können
explizit angegeben werden:

```bash
sudo ./scripts/install-hub.sh \
  --binary ./hub \
  --bind 192.168.10.25:3737 \
  --data-dir /srv/hub
```

Mit `--no-start` wird der Dienst installiert und aktiviert, aber noch nicht
gestartet. Das Script verändert absichtlich keine Firewallregeln.

Die Proxmox- oder Gast-Firewall sollte `3737/tcp` auf das tatsächlich genutzte
interne Subnetz beziehungsweise die Studio-Arbeitsplätze begrenzen.

## Update eines bestehenden Proxmox-Dienstes

Nach dem einmaligen Clone wird Hub direkt aus dem Repository aktualisiert:

```bash
cd /opt/Studio
./update.sh
```

Das Script wird als normaler Benutzer ausgeführt und fragt für Installation
und Dienstneustart selbst nach `sudo`. Es akzeptiert nur einen sauberen
Git-Stand, zieht den aktuellen Upstream per Fast-Forward, testet und baut
Hub und ersetzt anschließend `/usr/local/bin/hub`. Startet die neue
Version nicht, wird automatisch `/usr/local/bin/hub.previous`
wiederhergestellt. `/var/lib/hub` und `/etc/hub/hub.env` werden beim
Update nicht verändert.

Arbeitsplatzsicherungen werden versioniert und nur bei geändertem Inhalt neu
angelegt. Hub behält pro Arbeitsplatz und Sicherungstyp die letzten zehn
Änderungen, danach je einen Tagesstand für 30 Tage und anschließend je einen
30-Tage-Stand für zwölf Zeiträume. Bestehende einzelne Sicherungsdateien aus
Protokollversion 2 bleiben lesbar und gehen in diese Aufbewahrung ein.

Seit Protokollversion 3 werden Laser- und Materialprofile zusätzlich als
gemeinsamer Katalog automatisch zwischen allen Arbeitsplätzen abgeglichen.
Änderungen verwenden Inhaltshashes und Basisrevisionen; Löschungen bleiben als
Tombstones erhalten. Aktive Auswahlen sind weiterhin rein lokal. Die
arbeitsplatzbezogene Historie dient nur noch der bewussten Wiederherstellung.

Das Projektinventar erlaubt Studio außerdem, fehlende lokal gespeicherte
Projektversionen automatisch erneut hochzuladen. Nach dem Verlust des
Hub-Datenverzeichnisses ist daher kein manuelles Neuspeichern der Projekte
erforderlich.
