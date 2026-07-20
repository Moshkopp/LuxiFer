# ADR 0021: Zusatzachsen Z/U, Jog-Modell und Rotary

- Status: Vorgeschlagen
- Datum: 2026-07-20
- Betrifft: Laserpanel, Laserprofile, Ruida-Treiber, Maschinenbewegung
  (Jog/Dauerlauf), Positionsanzeige, spätere Rotary-Gravur

## Kontext

Auf dem Branch `feature/ruida-u-achse-jog` wurde an echter Hardware
(RDC6445G) verifiziert, wie sich die Zusatzachsen Z (Fokus/Betthöhe) und U
(Rotary/Drehachse) über das Ruida-Protokoll ansteuern lassen. Verifiziert und
byte-genau durch Tests gepinnt sind:

- **Schritt-Move** einer Achse: `D9 <achse> 02 <coord>` (relativ interpretiert),
  Achs-Byte X=0/Y=1/Z=2/U=3.
- **Dauerlauf** (Halten): `D9 D8 <flags>` mit
  `flags = 0x20 | (achse << 1) | richtung | stop`, alle vier Achsen in beiden
  Richtungen aufgezeichnet.
- **Positionsregister** X=`0x0421`, Y=`0x0431`, Z=`0x0441`, U=`0x0451`.
- Ein Kodierungsfehler in `encode_coord` für negative Koordinaten wurde
  nebenbei gefunden und behoben (32-Bit-Zweierkomplement statt 35-Bit-Maske).

Die Machbarkeit ist damit geklärt. Der Prototyp auf dem Branch ist aber
bewusst **nicht** für `main` gedacht, weil beim Ausprobieren mehrere
Design-Schulden entstanden sind:

1. **Keine Achsen-Verfügbarkeit.** Die Z/U-Bedienelemente sind immer aktiv,
   auch wenn die Maschine gar keine Z- oder U-Achse angeschlossen/aktiviert
   hat. Solche Klicks senden Kommandos ins Leere bzw. an nicht vorhandene
   Achsen.

2. **Zwei Kommando-Pfade mit widersprüchlicher Richtung.** Schritt-Move
   (`D9 <achse> 02`) und Dauerlauf (`D9 D8`) sind bei Ruida pro Achse
   gegeneinander invertiert. Der Prototyp gleicht das mit zwei ad-hoc-Helfern
   (`hold_dir`, `step_dir`) aus, die pro Achse einen der beiden Pfade drehen.
   Das ist fragil: „Schritt" und „Dauer" sind zwei verschiedene Prozesse mit
   eigener Richtungslogik, obwohl es fachlich **eine** Richtung „Achse +/−"
   gibt.

3. **Keine konfigurierbare Inversion.** Ob „U +" im oder gegen den
   Uhrzeigersinn dreht bzw. ob „Z +" hoch oder runter fährt, hängt von Verkabelung
   und Betrachtungsseite ab. Der Prototyp kodiert eine feste Konvention; es gibt
   keine nutzerseitige Umkehrung pro Achse.

4. **Rotary nur über U betrachtet.** Viele Ruida-Maschinen ohne U-Firmware
   betreiben die Rotary klassisch über die **Y-Achse** (Y-Motor abgeklemmt,
   Rotary an den Y-Ausgang). Dieser Fall wurde bisher nicht mitgedacht.

## Entscheidung

**(A) Achsen-Verfügbarkeit wird aus dem Controller gelesen. (B) Jog hat EIN
Richtungsmodell; „Tippen" und „Halten" sind nur zwei Auslöse-Arten desselben
Achsen-Jog. (C) Achsen-Inversion ist pro Achse im Laserprofil konfigurierbar.
(D) Rotary über Y wird als eigener Betriebsmodus des Profils unterstützt.**

### (A) Achsen-Verfügbarkeit: Z/U aus dem Profil, Y-Rotary aus dem Controller

**Wichtige, an Hardware verifizierte Korrektur** (Mitschnitt 2026-07-20): Ob
eine Z- bzw. U-Achse vorhanden ist, steht **NICHT** im Controller. Die
Achsen-Aktivierung („Z-Achse aktivieren", „U-Achse aktivieren") ist in der
Fremdsoftware eine **reine App-Einstellung** — beim Umschalten und Speichern
wurde nachweislich **kein einziges Register** in den Controller geschrieben
(nur Lese-/Status-Polls). Es gibt also kein Enable-Register für Z/U, das Studio
lesen könnte. (Die früher als Kandidaten genannten `0x0040`/`0x0050` scheiden
damit als Verfügbarkeitsquelle aus.)

Daraus folgt die Trennung:

- **Z-Achse / U-Achse vorhanden** → **Profil-Einstellung** in Studio (wie die
  Fremdsoftware es hält). In der Laser-Verwaltung je eine Checkbox „Z-Achse
  (Fokus)" / „U-Achse (Rotary)". Der Nutzer setzt sie beim Einrichten; sie wird
  im `LaserProfile` persistiert (offline, kein Gerät nötig).

- **Rotary über Y** dagegen IST ein Controller-Zustand: `0x0226`
  **rotary_enable** (`bit_mask: Some(1)`, schon im Settings-Block) schaltet den
  Controller in den Y-Rotary-Modus (Y-Bewegung wird zur Drehung). Zusammen mit
  `0x021F` **pulses_per_rot** und `0x0221` **rotary_diameter**. Diese sind aus
  dem Controller lesbar und für die spätere Y-Rotary-**Gravur** die Grundlage —
  sie müssen dort nicht neu erfunden werden.

Modellierung:

```
// LaserProfile (persistente Profil-Einstellung, serde(default)):
pub struct AxisConfig {
    pub has_z_axis: bool,   // vom Nutzer gesetzt (kein Controller-Register)
    pub has_u_axis: bool,
    pub invert_z: bool,     // Richtungs-Inversion, siehe (C)
    pub invert_u: bool,
}

// DriverCapabilities (nur das, was der Treiber wirklich aus dem Gerät weiß):
pub struct DriverCapabilities {
    …
    pub rotary_on_y: bool,  // aus 0x0226 rotary_enable (bekannt)
}
```

Der Z-Achsen-Positionswert (`0x0441`) und der U-Wert (`0x0451`) bleiben zur
Anzeige lesbar; sie sagen aber nichts über *Vorhandensein* aus (ein Register
kann einen Wert liefern, ohne dass eine Achse angeschlossen ist).

Solange nicht verbunden, sind Zusatzachsen gesperrt (wie schon Jog/Home). Die
UI (Jog-Kreuz-Ecken für Z/U) ist genau dann klickbar, wenn verbunden **und**
die jeweilige Capability `true` ist — analog zum bestehenden Muster, das
`position_read`/`user_origin_read` bereits für andere Bedienelemente nutzt.

### (B) Ein Jog-Prozess, ein Richtungsmodell

Es gibt fachlich **eine** Achsenbewegung mit **einer** Richtung „vorwärts/
rückwärts". Das Auslösen kennt zwei Arten:

- **Tippen** → fahre um einen festen Schritt (mm).
- **Halten** → fahre, solange gehalten wird (Watchdog stoppt beim Loslassen).

Der Treiber bekommt genau diese Semantik, z. B.:

```
enum JogMotion { Step(f64 /* mm */), HoldStart, HoldStop }
fn jog(&self, axis: MachineAxis, dir: AxisDir, motion: JogMotion, speed) -> …
```

Die pro-Achse-Invertierung zwischen `D9 <achse> 02` und `D9 D8` wird **im
Treiber** einmal aufgelöst (dort, wo die zwei Ruida-Kommandos gebaut werden),
nicht in der UI. Nach außen (Application, UI) existiert nur die eine logische
Richtung. Die Helfer `hold_dir`/`step_dir` aus dem Prototyp entfallen ersatzlos;
die UI meldet nur „Achse X, Richtung +, Tippen/Halten".

### (C) Konfigurierbare Inversion pro Achse

Das Laserprofil bekommt je Achse ein Invertierungs-Flag:

```
pub struct AxisConfig {         // im LaserProfile, serde(default)
    pub invert_z: bool,
    pub invert_u: bool,
    …
}
```

Die Inversion greift an genau einer Stelle im Treiber (Richtung → Achs-Kommando),
sodass sie Tippen und Halten gleichermaßen betrifft — es kann keine Divergenz
zwischen den Modi mehr geben. Damit beantwortet sich auch die offene
U-Rotationsfrage: Die „richtige" Drehrichtung ist Verkabelungs-/Betrachtungs-
sache und wird pro Maschine im Profil festgelegt, nicht hartkodiert.

### (D) Rotary über Y

Das Profil bekommt einen Rotary-Betriebsmodus:

```
pub enum RotaryMode {
    Aus,
    UAchse,   // Rotary am U-Ausgang (Firmware-Patch), Y bleibt Gantry
    YAchse,   // Rotary am Y-Ausgang, Y-Motor abgeklemmt (klassisch)
}
```

- **`UAchse`**: U ist eine eigenständige Achse; der **Jog** nutzt sie
  zusätzlich zur X/Y-Ebene.

  > **Korrektur 2026-07-20 (an Hardware geprüft):** Für die **Gravur** ist
  > dieser Modus am Ruida gegenstandslos. Die Rotary-Register wirken nur auf Y,
  > und das Job-Format trägt ohnehin nur X/Y — eine U-Bewegung lässt sich nicht
  > in eine Schnittbahn schreiben. U-Rotary am Ruida gibt es nur über eine
  > inoffizielle Firmware, die Y-Pulse auf den U-Ausgang umleitet; auch dann
  > bleibt der Job ein Y-Job. Einzelheiten in ADR 0023.
- **`YAchse`**: Rotary läuft über den Y-Ausgang; im Controller ist
  `0x0226 rotary_enable` gesetzt. Der Controller behandelt die Y-Bewegung
  selbst als Drehung und skaliert sie über seine `pulses_per_rot`/`diameter`
  (`0x021F`/`0x0221`). Für den **Jog** heißt das: die U-Bedienelemente
  entfallen, die Y-Pfeile *sind* die Drehung — es braucht **keine** app-seitige
  Skalierung, der Controller macht sie. Für die **Gravur** ist damit im
  einfachsten Fall nichts Besonderes zu tun: ein normaler Y-Job wird vom
  Controller als Rotary interpretiert, solange `rotary_enable` an ist (Details
  klärt das Gravur-ADR).

Dieses ADR legt nur den **Modus im Profil und sein Jog-Verhalten** fest. Die
eigentliche Rotary-**Gravur** ist Gegenstand eines eigenen späteren ADR. Der
Y-Fall wird hier nicht verbaut, und die Controller-Register (`0x0226`/`0x021F`/
`0x0221`) sind bereits im Settings-System verfügbar.

## Konsequenzen

**Positiv**

- Bedienelemente sind nur aktiv, wenn die Achse real vorhanden ist — keine
  Kommandos an nicht existierende Achsen.
- Eine einzige Richtungslogik statt zweier widersprüchlicher Pfade; die
  Modus-Divergenz (Tippen ≠ Halten) ist strukturell ausgeschlossen.
- Inversion und Rotary-Modus sind pro Maschine konfigurierbar und persistent
  (Laserprofil, wie Bett/Nullpunkte).
- Der klassische Rotary-über-Y-Betrieb bleibt möglich.

**Aufwand / Risiko**

- Z/U-Verfügbarkeit ist eine **Profil-Einstellung** (an HW verifiziert: kein
  Controller-Register). Y-Rotary bleibt aus `0x0226 rotary_enable` lesbar.
- `LaserProfile` (AxisConfig) und `DriverCapabilities` wachsen um Felder; die
  Serialisierung bleibt über `serde(default)` rückwärtskompatibel (bestehende
  Profile laden weiter, Zusatzachsen zunächst aus/gesperrt).
- Das Trait `MachineDriver` wird auf das einheitliche `jog(axis, dir, motion,
  speed)` umgestellt; die Prototyp-Methoden (`jog_axis`, `hold_axis_start/stop`)
  werden dabei zusammengeführt.

## Umsetzungsreihenfolge (nach Annahme des ADR)

1. `AxisConfig` (has_z_axis/has_u_axis) ins `LaserProfile` + Checkboxen in der
   Laser-Verwaltung; `DriverCapabilities.rotary_on_y` aus `0x0226`. UI sperrt
   Z/U-Ecken anhand der Profil-Einstellung.
2. Trait auf `jog(axis, dir, motion, speed)` vereinheitlichen; Inversion +
   Kommando-Auswahl in den Treiber ziehen; `hold_dir`/`step_dir` entfernen.
3. `AxisConfig` (Inversion) ins `LaserProfile` + Bedienung in der
   Laser-Verwaltung.
4. `RotaryMode` ins `LaserProfile`; Jog-Verhalten für `YAchse` (U-Elemente aus,
   Y ist die Drehung).
5. Die Rotary-**Gravur** bleibt einem eigenen ADR vorbehalten.

## Nicht Teil dieser Entscheidung

- Die Rotary-Gravur selbst (Y-Skalierung, Durchmesser, Schritte/Umdrehung).
- Z-Positionsregister-Semantik: `0x0441` zeigte im Test zeitweise einen festen
  Wert (3200) statt der Ist-Position; ob es die echte Z-Position ist, ist noch
  offen und für die Anzeige gesondert zu klären.
