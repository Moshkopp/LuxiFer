# ADR 0015: Treiberautoritative Ausführungsspur für Job und Laser-Vorschau

Status: umgesetzt (2026-07-17)

## Problem

`JobPreview::from_plan` interpretiert den geräteneutralen `JobPlan` selbst.
Der aktive Treiber führt danach jedoch weitere Entscheidungen aus: Reihenfolge
und Richtung von Scanzeilen, Laser-an/aus-Bewegungen, Startanker,
Maschinenursprung, Koordinatenquantisierung und bei Ruida den Scan-Offset. Die
Preview kann deshalb plausibel aussehen, ohne dem gesendeten Job zu entsprechen.

## Entscheidung

Jeder Treiber erzeugt vor der Serialisierung eine `ExecutionTrace`. Diese Spur
ist die einzige Quelle für Bewegungsreihenfolge, Preview und Wegkennzahlen. Erst
danach serialisiert derselbe Treiber die Spur beziehungsweise die gemeinsam mit
ihr erzeugten Maschinenbefehle zu Ruida-Bytes oder G-Code.

Jeder Trace-Schritt enthält mindestens:

- Start- und Endpunkt in Maschinenkoordinaten;
- Laser an/aus;
- Bewegungsart `Cut`, `Fill`, `Raster` oder `Travel`;
- Layer und Ausführungsreihenfolge;
- ideale Koordinaten vor Maschinenkompensation und ausgeführte Koordinaten nach
  Scan-Offset beziehungsweise Quantisierung.

Damit lassen sich logische Darstellung und maschinengenaue Kontrolle aus
derselben Spur ableiten, ohne zwei Bewegungsplaner zu pflegen.

## Preview-Optionen

- `Leerfahrten anzeigen`: Laser-aus-Schritte dezent darstellen.
- `Laserpfad hervorheben`: Laser-an-Schritte halbtransparent grün überlagern;
  Standard aus.
- `Scan-Offset anzeigen`: ausgeführte statt ideale Scan-Koordinaten darstellen;
  Standard aus. Ist im Profil ein Offset aktiv, aber ausgeblendet, weist die UI
  darauf hin. Der gesendete Job bleibt davon unberührt.

Die Legende zeigt `Laserweg`, `Leerweg` und `Gesamtfahrweg`. Alle Werte stammen
aus der vollständigen, ausgeführten Spur; Anzeige-Schalter verändern niemals
die Kennzahlen oder den Job.

Die Trace bleibt in Maschinenkoordinaten. Für den oben-links orientierten
Canvas rechnet ausschließlich der Renderer die Punkte mit dem im Laserprofil
gewählten Maschinen-Nullpunkt zurück. Dadurch bleibt die Vorschau auch bei
GRBL-Profilen mit unterem Nullpunkt aufrecht; G-Code und reale Fahrbewegung
werden durch diese reine Darstellungstransformation nicht verändert.

## Treiberspezifische Regeln

- Ruida: Boustrophedon-Reihenfolge, Scan-Offset, Startmodus/Anker und
  Mikrometerquantisierung gehören in die Spur.
- GRBL: Die Spur folgt den tatsächlich ausgegebenen `G0`-/`G1`-Bewegungen und
  darf nicht stillschweigend Ruida-Bidirektionalität annehmen.
- Ein Wechsel zwischen getrennten Rasterobjekten bleibt eine Leerfahrt. Nur
  die kontinuierliche Scanbewegung innerhalb desselben Objekts darf als solche
  zusammengefasst werden.

## Migrationsschritte

1. `ExecutionTrace` und Bewegungstypen im Core ergänzen.
2. Ruida- und GRBL-Compiler so zerlegen, dass Spur und Serialisierung dieselbe
   geordnete Befehlsquelle verwenden.
3. `LaserService` liefert die Spur des aktiven Profils einschließlich Ursprung,
   Startmodus und Anker.
4. Native Preview rendert ausschließlich diese Spur und erhält die drei
   unabhängigen Anzeigeoptionen.
5. Alte `JobPreview::from_plan`-Bewegungsheuristik entfernen.

Die native Laser-Vorschau verwendet seit diesem Umsetzungsschnitt nur noch
`LaserService::execution_trace` des aktiven Profils. Die ältere
`JobPreview::from_plan`-API bleibt vorläufig ausschließlich als kompatible
Core/Application-Hilfsoberfläche für bestehende Tests erhalten und ist keine
Renderautorität mehr.

### Interne Treiberquelle

GRBL baut eine gemeinsame `grbl_motion_program`-Folge, die sowohl der
G-Code-Serializer als auch `execution_trace` konsumieren. Ruida bündelt Cut-
Bewegungen in `ruida_cut_motions` und Scanbewegungen in
`ruida_scan_motions`; `compile_geometry`/`compile_scan` und die Trace-Erzeugung
verwenden dieselben Ergebnisse. Damit existieren Richtung, Reihenfolge,
Pass-Wiederholung, Travel-Grenzen und Scan-Offset nicht mehr als getrennte
Preview-Nachbildung neben dem Treiberprogramm.

## Abnahme

- Für beide Treiber stimmen Spur und serialisierte Bewegungsreihenfolge in
  Paritätstests überein.
- Mehrere Rasterobjekte zeigen Leerfahrten zwischen Objekten, aber keine
  erfundenen Rücksprünge innerhalb eines bidirektionalen Scans.
- Scan-Offset ist in der Darstellung standardmäßig verborgen und optional
  sichtbar.
- Der grüne Laserpfad und alle drei Weglängen stammen aus der Trace.
