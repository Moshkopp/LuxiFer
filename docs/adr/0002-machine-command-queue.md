# ADR 0002: Zentrale Machine Command Queue mit State Guards

## Status
Akzeptiert — 2026-07-06

## Kontext
Maschinenbefehle (Connect, Jog, StartJob, EmergencyStop, ...) müssen sicher und
nachvollziehbar ausgeführt werden. Direkte Treiberzugriffe aus ViewModels wären
schwer testbar und fehleranfällig.

## Entscheidung
- Alle Befehle laufen als `MachineCommand`-Records über die zentrale
  `MachineCommandQueue` (single reader, serialisierte Ausführung).
- Vor jeder Ausführung prüft der `StateGuard` die Zulässigkeit im aktuellen
  `MachineState`. `EmergencyStop` ist in jedem Zustand erlaubt.
- Die UI kennt nur die Queue, niemals den `IMachineDriver` direkt.
- Treiber implementieren ausschließlich `IMachineDriver` (erste: Ruida).

## Konsequenzen
- State-Übergänge sind zentral testbar (siehe `StateGuardTests`).
- Neue Controller (Simulator, GRBL, ...) sind reine Treiber-Implementierungen
  ohne Änderungen an UI oder Core.
