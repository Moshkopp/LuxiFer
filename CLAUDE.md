# CLAUDE.md — Arbeitsrichtlinien für LuxiFer

Diese Datei ist verbindlich. Regeln mit **MUSS** / **DARF NICHT** sind
Architektur-Invarianten; Abweichung nur mit ausdrücklicher Zustimmung des
Nutzers und begleitendem ADR. Antworten und Code-Kommentare auf Deutsch.

## Projekt in einem Satz

LuxiFer ist eine offline-first Desktop-Anwendung (C#/Avalonia) zur
Laser-Steuerung. **Die GUI ist das Produkt.** Sharon (Rust) ist ein optionaler
Koordinations-Server und niemals Voraussetzung für lokale Arbeit.

## Verzeichnisse

| Pfad | Inhalt |
|------|--------|
| `luxifer/` | C#/.NET-10-Solution (die GUI, das Produkt) |
| `sharon/` | Rust-Cargo-Workspace (optionaler Server) |
| `docs/adr/` | Architekturentscheidungen |
| `nur zur Referenu/` | Altes ThorBurn-Projekt — **nur Referenz, gitignored** |

## Schichten und ihre Regeln (LuxiFer)

```
LuxiFer.App  →  Core, Machines, Machines.Ruida, Persistence, Sync
LuxiFer.Machines / .Ruida / .Persistence / .Sync  →  Core
LuxiFer.Core  →  (nichts)
```

1. **Fachlogik gehört in `LuxiFer.Core`**, nicht in ViewModels oder Controls.
   Geometrie, Hit-Test, Bounds, Skalierung, Domänenregeln sind im Core und
   dort testbar. Faustregel: Wenn es ohne Avalonia testbar sein sollte, gehört
   es in den Core.
2. **`LuxiFer.Core` MUSS UI-frei bleiben** — kein `using Avalonia.*`, keine
   Hardware-, Datei- oder Netzwerk-Zugriffe.
3. **ViewModels DÜRFEN NICHT `IMachineDriver` referenzieren.** Maschinenbefehle
   laufen ausschließlich als `MachineCommand` über die `MachineCommandQueue`.
   Vor Ausführung prüft `StateGuard` die Zulässigkeit (siehe ADR 0002).
4. **Server-Kommunikation MUSS in `LuxiFer.Sync` gekapselt sein.** Kein anderer
   Teil der GUI spricht direkt mit Sharon. Kein Kernfeature darf Sharon
   voraussetzen (siehe ADR 0001).
5. Code-Behind (`*.axaml.cs`) bleibt dünn: nur View-Verdrahtung, keine
   Fachlogik. State lebt im ViewModel, Fachlogik im Core.
6. Kommunikation zwischen Core/UI/Sync/Machine erfolgt über Domain Events
   (`IEventBus`), nicht über direkte Querverweise.

## Regeln für Sharon

7. **Sharon steuert niemals eine Maschine.** Er koordiniert nur (Sync,
   Bibliotheken, Machine-Session-Vergabe). Der Canvas-Inhalt ist für Sharon ein
   opakes Blob — keine Canvas-Fachlogik im Server.

## Build, Test, Format

```bash
# LuxiFer (aus luxifer/)
dotnet build                      # gesamte Solution
dotnet test                       # Unit-Tests (müssen grün sein)
dotnet run --project src/LuxiFer.App   # App starten (NICHT die .slnx angeben)
dotnet format                     # Formatierung vor dem Commit

# Sharon (aus sharon/)
cargo build
cargo test
cargo fmt
cargo clippy
```

8. **Vor jedem Commit:** betroffene Seite baut (`dotnet build` bzw.
   `cargo build`) und Tests sind grün. Neue Core-Fachlogik bekommt Tests.
9. `dotnet run` startet das **Projekt** `src/LuxiFer.App`, nie die Solution-Datei.

## Commits

- Sprache: Deutsch. Betreff im Imperativ, knapp; Body erklärt das *Warum*.
- Ein Commit = eine logische Änderung. Kein Vermischen von Refactoring und
  Feature.
- Nur committen/pushen, wenn der Nutzer es verlangt.
- Commit-Footer:
  `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`

## Architekturänderungen

10. Änderungen an den obigen Invarianten oder an der Schichtenstruktur werden
    **als ADR** in `docs/adr/` festgehalten (fortlaufend nummeriert), bevor sie
    umgesetzt werden. Neue Features dürfen die bestehende Architektur nicht
    verschlechtern; Refactoring nur aus konkretem Anlass.

## ThorBurn-Referenz

11. Aus `nur zur Referenu/` wird **kein Code kopiert.** Nur analysieren, welche
    Funktion wie gebaut war, und im aktuellen Stil sauber neu implementieren.
