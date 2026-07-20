# ADR 0023: Rotary-Gravur

- Status: **Teilweise umgesetzt** — Einrichtung und Controller-Register stehen,
  die Job-Seite ist offen. Die Modus-Frage ist an Hardware geklärt (siehe
  „Befund an der Maschine").
- Datum: 2026-07-20
- Betrifft: studio-core (Job-Kompilierung), Treiber (Ruida), Application,
  Laserprofile, Laserpanel
- Baut auf: ADR 0021 (Zusatzachsen/Jog/Rotary-Modi), ADR 0022 (Rotary-Bauarten
  und Achsenkalibrierung)

## Kontext

ADR 0021 legt die drei Rotary-Modi (`Aus`, `UAchse`, `YAchse`) und ihr
**Jog**-Verhalten fest. ADR 0022 liefert das **Fachmodell** (Chuck/Roller,
`circumference_mm`, `steps_per_mm`) und die Achsenkalibrierung. Beide ADRs
verweisen die eigentliche **Gravur** ausdrücklich auf ein eigenes ADR — dieses.

Die offene Frage ist nicht, wie sich ein Rotary dreht, sondern **wann und wo aus
einem flachen Job ein Rotary-Job wird**. Der Job-Pfad ist heute durchgängig
zweidimensional: Shapes in mm → `JobPlan` → Treiber → Controller. Ein Rotary
ersetzt eine der beiden Achsen durch eine Drehung.

### Der Kern: zwei Modi, zwei völlig verschiedene Verantwortungen

Die beiden Rotary-Modi aus ADR 0021 unterscheiden sich in der Gravur **nicht
graduell, sondern grundsätzlich** — und zwar darin, *wer* rechnet:

| | `YAchse` | `UAchse` |
|---|---|---|
| Wer skaliert | **Der Controller** (`0x0226` + `0x021F`/`0x0221`) | **Studio** |
| Was Studio sendet | ein ganz normaler X/Y-Job | X + U statt X/Y |
| Was Studio wissen muss | dass die Register stimmen | die volle Rotary-Physik |
| Risiko | Doppelskalierung | falsche Abwicklung |

Daraus folgt die zentrale Feststellung dieses Entwurfs:

> **Im Modus `YAchse` darf Studio die Bewegung nicht selbst umrechnen.** Der
> Controller tut es bereits. Eine zusätzliche app-seitige Skalierung ergäbe eine
> doppelte Umrechnung — der Job führe um den Faktor der Rotary-Skalierung falsch.

Das ist der gefährlichste denkbare Fehler in diesem Bereich, und er entsteht
gerade dann, wenn man „Rotary-Gravur" als *einen* Fall behandelt.

## Befund an der Maschine (2026-07-20)

Am RDC6445G gemessen und recherchiert — das korrigiert die Annahmen aus
ADR 0021 §D:

1. **Die Rotary-Register gelten nur für Y.** Mit `rotary_enable` an ändert sich
   das Verhalten der **Y**-Achse (langsamer, eingeschränkter Weg); die U-Achse
   bleibt unberührt. Ein Jog auf U ändert sich nicht, wenn man
   `rotary_diameter` verstellt.
2. **U als Rotary-Achse gibt es im Serien-Ruida nicht.** Es existiert eine
   inoffizielle Firmware (`RDC6445G-V15.01.22-LIB`), die die Y-Pulse auf den
   U-Ausgang **umleitet**. Auch dort bleibt der Job ein Y-Job — es gibt keine
   U-Koordinate im Protokoll. Im Controller-Menü der offiziellen Firmware gibt
   es entsprechend keine Y/U-Auswahl.
3. **Das Job-Format kennt nur X und Y.** `cmd_move_abs`/`cmd_cut_abs` tragen
   zwei Koordinaten. Eine dritte Bewegungsachse ließe sich nicht in eine
   Schnittbahn schreiben, auch wenn man wollte.

**Folge:** Rotary am Ruida heißt Rotary über Y — entweder per Firmware-Patch
umgeleitet oder durch Umstecken des Motorkabels. Der Modus `RotaryMode::UAchse`
aus ADR 0021 ist für die **Gravur** damit gegenstandslos; für den **Jog** bleibt
U eine normale, nutzbare Achse.

### Y-Rotary an Hardware bestätigt (2026-07-20)

Motorkabel getauscht (Rotary an Y, Y-Motor an U), `rotary_enable` samt Pulsen
und Durchmesser über den Rotary-Dialog geschrieben, Controller neu gestartet:

- **§B trägt.** Studio sendet einen unveränderten X/Y-Job; die Y-Werte sind
  gewöhnliche Millimeter. Der Controller rechnet die Drehung selbst. Eine
  app-seitige Skalierung fand nicht statt und darf auch nicht dazukommen.
- Maßhaltig: 20×20 mm auf einer Dose, 52-mm-Walze, 1250 Pulse.
- Praxisgriff des Nutzers: erst mit echtem Y-Motor homen, **dann** umstecken.
  Der alte Y-Motor an U hält den Portalarm über den Haltestrom fest, damit er
  beim Rastern in X nicht wandert.
- Wohin der Kopf nach dem Job fährt, ist eine **Controller-Einstellung**
  (Ausgangspunkt bzw. Referenzfahrt) — Studio muss dafür nichts in den Job
  schreiben.

Für G-Code-Steuerungen (GRBL/grblHAL/FluidNC) liegt der Fall anders: dort ist
die A-Achse gleichberechtigt und steht als eigener Buchstabe in jeder Zeile
(`G1 X.. Y.. A..`). Ein späterer Treiber dorthin ist der saubere Weg zu echter
Mehrachsigkeit.

## Entscheidung (Vorschlag)

**(A) Rotary-Gravur ist kein eigener Job-Typ, sondern eine Eigenschaft der
Kompilierung. (B) Im Modus `YAchse` bleibt der Job unverändert — Studio schreibt
nur die Controller-Register konsistent. (C) Nur im Modus `UAchse` rechnet der
Core eine Achse in Drehung um, über das Fachmodell aus ADR 0022. (D) Die
Umrechnung liegt im Core, nicht im Treiber und nicht in der UI.**

### (A) Kein eigener Job-Typ

Ein Rotary-Job ist derselbe `JobPlan` wie ein flacher Job. Es gibt **keinen**
`RotaryJobPlan`. Was sich ändert, ist ausschließlich, wie eine Achse beim
Kompilieren interpretiert wird. Damit bleiben Vorschau, Ausführungsspur (ADR
0015), Materialrezepte (ADR 0019) und Nullpunkte (ADR 0020) unangetastet.

### (B) `YAchse`: Studio rechnet nicht

Studio kompiliert einen normalen X/Y-Job. Zusätzlich stellt es sicher, dass die
Controller-Register zum Profil passen:

- `0x0226 rotary_enable` = 1
- `0x021F pulses_per_rot` und `0x0221 rotary_diameter` entsprechend dem Rotary
  aus dem Profil (ADR 0022)

Offen zur Klärung: ob Studio diese Register **schreibt** (Risiko: überschreibt
Nutzereinstellungen) oder nur **prüft und warnt** (Risiko: Nutzer graviert mit
falscher Skalierung). Der Entwurf neigt zu *prüfen und warnen*, weil das
Schreiben von Registern ohne Not eine fremde Maschinenkonfiguration verändert.

### (C) `UAchse`: entfällt am Ruida — gilt später für G-Code-Treiber

Ursprünglich stand hier „der Core rechnet die Y-Koordinate in eine U-Strecke
um". Das ist am Ruida **nicht umsetzbar**: das Job-Format trägt nur X und Y
(siehe Befund oben). Der Abschnitt bleibt als Vorgabe für einen späteren
G-Code-Treiber stehen, wo A eine echte Achse ist.

Dort gilt: Der Core rechnet die abgewickelte Strecke über `Rotary` aus ADR
0022, der Treiber setzt sie in die Einheit seiner Achse um. Bei GRBL/FluidNC
ist das **Grad** (`steps_per_mm` einer A-Achse bedeutet dort faktisch
Schritte pro Grad), nicht mm — die Umrechnung mm-Abwicklung → Grad gehört in
den Treiber, nicht in den Core.

### (D) Verortung

- **Core**: die Umrechnung Y→U als reine Funktion über dem Rotary-Modell,
  testbar ohne Gerät. Keine Register, kein Treiberwissen.
- **Treiber**: bildet die umgerechneten Werte auf seine Ausgabe ab, wie bisher.
- **Application**: wählt anhand des Profil-Modus, welcher Weg gilt, und
  verweigert die Ausführung bei unstimmiger Konfiguration.
- **Native**: zeigt den Modus an und warnt sichtbar, wenn ein Rotary-Job ansteht.

## Offene Fragen (vor Umsetzung zu klären)

1. **Register schreiben oder nur prüfen?** (§B) — betrifft fremde Maschinen.
2. **U in mm oder Grad?** (§C) — betrifft Materialparameter und Vorschub.
3. **Was passiert mit der zweiten Achse?** Bei `UAchse` bleibt Y physisch
   vorhanden. Wird sie im Rotary-Job gesperrt, oder darf ein Job X/Y/U mischen?
4. **Bettgrenzen.** Eine Drehung hat keine Begrenzung, die flache Y-Achse schon.
   Wie wird die Arbeitsbereichsprüfung im Rotary-Modus umgestellt?
5. **Vorschau.** Zeigt die Vorschau die abgewickelte Fläche (flach) oder deutet
   sie die Rundung an? Der Entwurf neigt zu flach-abgewickelt.
6. **Umfang-Überlauf.** Was, wenn das Objekt länger als der Umfang ist — Abbruch,
   Warnung oder Wickeln?

## Konsequenzen

**Positiv**

- Die gefährliche Doppelskalierung im `YAchse`-Modus ist ausdrücklich
  ausgeschlossen statt implizit vermieden.
- Rotary-Gravur erbt Vorschau, Spur und Materiallogik, statt sie zu duplizieren.
- Die Physik bleibt an einer Stelle (ADR 0022), die Gravur fragt sie nur ab.

**Aufwand / Risiko**

- Der Job-Kompilierungspfad im Core bekommt eine achsabhängige Verzweigung —
  die Stelle, an der ein Fehler direkt Hardware bewegt. Braucht Tests mit
  bytegenauen Erwartungen, analog zum vorhandenen Jog-Test.
- Die offenen Fragen 3–6 sind keine Details: jede kann die Umsetzung ändern.
- **Nichts davon ist an Hardware verifiziert.** Insbesondere die Annahme, dass
  der Controller im `YAchse`-Modus vollständig selbst skaliert, stammt aus der
  Registeranalyse (ADR 0021 §D) und ist nicht gemessen.

## Nicht Teil dieser Entscheidung

- GRBL/FluidNC-Rotary (A-Achse) — das Modell ist vorbereitet, die Abbildung
  folgt, wenn ein solcher Treiber produktiv wird.
- Die U/Z-Enable-Bit-Dekodierung (weiterhin offen aus ADR 0021).
