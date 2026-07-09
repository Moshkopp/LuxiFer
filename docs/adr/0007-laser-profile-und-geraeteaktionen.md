# ADR 0007: Laser-Profile & gerätespezifisches Laserpanel

## Status
Akzeptiert — 2026-07-09

## Kontext

Das heutige Laserpanel (`luxifer/frontend/src/lib/LaserPanel.svelte`) ist **auf
zwei Geräte gleichzeitig verdrahtet** und vermischt sie:

- ein fester **„G-Code"-Knopf** (`ongenerate`) — reine **GRBL**-Denke,
- ein **„An Laser senden"**-Knopf mit `title="Job an Ruida-Maschine senden"` —
  reine **Ruida**-Denke.

Beide stehen nebeneinander, unabhängig vom tatsächlichen Gerät. Das ist genau die
Vermischung, die ADR 0006 §5 verbietet: Gerätespezifik gehört hinter den Treiber,
nicht ins gerätefreie UI. „Einen Job starten" bedeutet je Gerät etwas anderes —
Ruida: UDP-Upload + Start; GRBL: G-Code-Zeilen über die serielle Verbindung
streamen. **Das Panel darf das nicht wissen.**

Zusätzlich fehlt eine Grundlage, die ab jetzt gebraucht wird: Man kann **keinen
Laser anlegen und speichern**. IP, Port bzw. serieller Anschluss, Gerätetyp und
Bettgröße sind heute Hardcode/Platzhalter im Panel (`ip = "192.168.1.100"`). Für
den echten Betrieb (und für mehrere Maschinen) braucht es **speicherbare
Laser-Profile**.

## Entscheidung

**(A) Laser-Profile als app-globale, gespeicherte Geräte-Registry.
(B) Das Laserpanel rendert die Job-Aktionen, die der aktive Treiber meldet —
keine fixen geräteabhängigen Knöpfe mehr.**

### A. Laser-Profile (app-global, eigene Datei)

Ein Laser gehört zur **Werkstatt, nicht zum Projekt** — dasselbe Gerät bedient
alle Projekte. Profile leben daher in einer **eigenen Config-Datei im
App-Datenverzeichnis**, projektübergreifend (nicht im Projektformat ADR 0003).

Ein Profil im Core (gerätefrei modelliert):

```rust
pub struct LaserProfile {
    pub id: LaserId,
    pub name: String,          // frei, z. B. "Ruida groß (Keller)"
    pub kind: DriverKind,      // Ruida | Grbl | MiniGrbl
    pub connection: Connection,// Netz { ip, port } | Seriell { port, baud }
    pub bed_mm: (f64, f64),    // Arbeitsbereich B×H
    pub scan_offset: ScanOffset, // Reversal-Kalibrierung (s. u.)
}
```

**Scan-Offset (Reversal-Kalibrierung) gehört ins Profil.** Beim bidirektionalen
Rastern brennen die Rückwärts-Zeilen durch die mechanisch/optische Latenz des
Kopfes horizontal versetzt gegen die Vorwärts-Zeilen — der Rand franst aus. Der
Versatz ist **geschwindigkeitsabhängig** (bei 400 mm/s größer als bei 100) und
ein **physikalischer Kennwert der konkreten Maschine** (Antrieb, Beschleunigung,
Optik). Er ist damit **Geräte-Kalibrierung, kein Job-Inhalt** — und gehört zum
Profil, nicht in den `JobPlan`:

```rust
pub struct ScanOffsetPoint { pub speed_mm_s: f64, pub offset_mm: f64 }
pub struct ScanOffset { pub enabled: bool, pub points: Vec<ScanOffsetPoint> }
```

Eine **Stützpunkt-Tabelle Geschwindigkeit → Offset**, zwischen den Punkten
linear interpoliert (`enabled = false` → Offset 0). Ein einzelner fixer Wert
genügt nicht. Das Settings-UI editiert diese Tabelle pro Laser.

**Wo der Offset angewandt wird, ist bewusst NICHT hier:** der `JobPlan` bleibt
Ideal-Soll-Geometrie; **der Treiber** rechnet seinen Offset beim Serialisieren
ein (ADR 0006). Ein GRBL-Laser mit anderem Verhalten bekommt so **keine
Ruida-Korrektur** aufgezwungen.

`DriverKind` bestimmt, **welchen `MachineDriver` die App instanziiert**, und
`Connection` liefert dessen Verbindungsparameter. Der Core hält die Liste der
Profile + die **aktive** Auswahl; Laden/Speichern der Datei liegt im Tauri-
Backend (JSON), der Core bleibt I/O-frei und testbar (wie ADR 0004 beim
Asset-Store).

**Rollenteilung Settings ↔ Laserpanel:**

- **Settings = verwalten (selten).** Ein Dialog, der Laser-Profile **anlegen /
  bearbeiten / löschen** kann (CRUD), inkl. Scan-Offset-Tabelle. Erster und
  vorerst einziger Inhalt der Settings; weitere Kategorien folgen später.
- **Laserpanel = auswählen (ständig).** Ein **Dropdown** im Panel listet die
  gespeicherten Laser; die Auswahl setzt den **aktiven** Laser. Laser wechseln
  passiert beim Arbeiten laufend, anlegen selten — deshalb getrennt.

Der aktive Laser bestimmt, welcher Treiber im Panel arbeitet. Beim Wechsel
instanziiert die App den Treiber zum Profil **neu** (`RuidaDriver::new(profil)`,
ADR 0006) — der neue Treiber trägt IP/Bett/Scan-Offset des gewählten Lasers. Ist
kein Laser angelegt, zeigt das Dropdown einen Hinweis „In Settings anlegen"; die
Job-Aktionen sind dann inaktiv.

### B. Gerätegemeldete Job-Aktionen

Der `MachineDriver`-Trait (ADR 0001/0006) meldet seine **verfügbaren Aktionen**;
das Panel rendert genau diese — nichts Fixes:

```rust
pub enum JobAction {
    SendJob,       // Ruida: UDP-Upload + Start
    StreamGcode,   // GRBL: G-Code-Zeilen streamen
    ExportFile,    // Bytes in Datei schreiben (.rd bzw. .gcode)
    Frame, Home, GoOrigin, Contour, Pause, Stop,
}

pub trait MachineDriver {
    // … name(), compile(), Live-Steuerung (ADR 0006) …
    fn actions(&self) -> Vec<JobAction>;   // was DIESES Gerät anbietet
    fn run_action(&self, action: JobAction, plan: &JobPlan, params: &JobParams)
        -> Result<(), DriverError>;
}
```

- **Ruida** meldet `SendJob` (nicht „G-Code"). „Senden" heißt intern:
  `compile(plan)` → UDP-Upload.
- **GRBL/miniGRBL** melden `StreamGcode` (+ `ExportFile`). „Starten" heißt
  intern: `compile(plan)` → G-Code → seriell streamen.
- Gemeinsame Aktionen (`Frame`, `Home`, `Stop`, …) meldet jeder Treiber, der sie
  kann; das Panel zeigt nur, was gemeldet wird.

Das Panel bekommt vom Backend die **Aktionsliste des aktiven Treibers** und
zeichnet pro Aktion einen Knopf (Label/Glyph aus einer neutralen Zuordnung).
Klick → ein Tauri-Command `run_action(action, params)`; das Backend delegiert an
den aktiven Treiber. Das Panel kennt **weder G-Code noch Ruida-UDP**.

**Geräteneutral und daher im Panel bleibend:** „Starten von" (Absolut/Aktuell/
Ursprung), der **Job-Nullpunkt-Anker** (3×3), Jog-Schritt/Speed, Positions-
anzeige. Das sind Job-Parameter (`JobParams`), kein Gerätedetail.

## Invarianten

1. **Das Laserpanel enthält keinen gerätespezifischen Knopf** (kein fixes
   „G-Code", kein „…an Ruida senden"). Es rendert ausschließlich die vom aktiven
   Treiber gemeldeten `JobAction`s.
2. **„Job starten" ist Treiber-Sache.** Was beim Auslösen passiert (UDP-Upload
   vs. G-Code-Streamen), entscheidet allein der Treiber hinter `run_action`.
3. **Laser-Profile sind app-global**, in eigener Datei — nicht im Projekt (ADR
   0003). Der Core hält sie I/O-frei; Persistenz macht das Backend.
4. `LaserProfile`, `DriverKind`, `Connection`, `JobAction`, `JobParams` sind
   **gerätefreie Core-Typen**. Der Core baut daraus keinen Treiber-internen
   Zustand.
5. **Anlegen/Verwalten in Settings, Auswählen im Panel-Dropdown.** Der aktive
   Laser wird im Laserpanel gewählt; der Wechsel erzeugt den Treiber neu (ADR
   0006). Profile bearbeiten passiert nur in Settings.

## Konsequenzen

- Ruida-Nutzer sehen **kein G-Code** mehr; GRBL-Nutzer bekommen das passende
  „Streamen/Export" — dieselbe Oberfläche, treibergesteuerter Inhalt.
- Mehrere Maschinen sind anlegbar und persistent; der aktive Laser schaltet den
  Treiber um, ohne Panel-Code zu ändern.
- Ein neuer Treiber (GRBL) bringt seine Aktionen selbst mit — das Panel muss
  dafür **nicht** angefasst werden (analog ADR 0001: neues Crate, kein
  bestehender Code geändert).
- Der bisher fixe `ongenerate`/`onsend`-Vertrag des Panels entfällt und wird
  durch `actions()` + `run_action()` ersetzt.

## Reihenfolge der Umsetzung

1. Core-Typen: `LaserProfile`, `DriverKind`, `Connection`, `JobAction`,
   `JobParams`; Profil-Liste + aktive Auswahl im `AppState` (I/O-frei), Tests.
2. Backend-Persistenz: Profile-Datei (JSON) im App-Datenverzeichnis laden/
   speichern; Tauri-Commands (CRUD + aktiv setzen).
3. Settings-UI: Laser-Profile anlegen/bearbeiten/löschen/aktiv wählen.
4. `MachineDriver` um `actions()`/`run_action()` erweitern; Ruida meldet
   `SendJob` & Co.
5. Laserpanel umbauen: gerätegemeldete Aktions-Knöpfe statt fixem G-Code/Send.

Punkt 4 greift in den Ruida-Treiber aus **ADR 0006** — beide zusammen ergeben
das erste real bedienbare Gerät. Reihenfolge mit dem Nutzer:
erst besprechen, dann bauen.

## Nicht Teil dieser Entscheidung

- Der **GRBL/miniGRBL-Treiber** selbst (nur der Platz dafür wird geschaffen).
- Weitere Settings-Kategorien (Anzeige, Einheiten …) — später.
- Auto-Discovery von Maschinen im Netz; manuelles Anlegen genügt.
