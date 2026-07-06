# LuxiFer

Moderne Desktop-Anwendung zur Steuerung von Laser- und Werkzeugmaschinen.
**Offline-first. Die GUI ist das Produkt.**

## Struktur

```
luxifer/    C# / Avalonia Desktop-Anwendung (das Produkt)
  src/
    LuxiFer.App             Avalonia-UI (MVVM), Canvas-Rendering
    LuxiFer.Core            Fachlogik: Canvas-Dokument, Projekte, Events, State Guards
    LuxiFer.Machines        IMachineDriver + zentrale MachineCommandQueue
    LuxiFer.Machines.Ruida  Ruida-Treiber (erste Maschinenanbindung)
    LuxiFer.Persistence     lokale Projekt-/Asset-/Font-Speicherung
    LuxiFer.Sync            optionaler Client für Sharon (REST/WS)
  tests/
    LuxiFer.Core.Tests

sharon/     Rust — optionaler Koordinations-Server (niemals Pflicht)
  crates/
    sharon-server           Binary: REST API + WebSocket-Events (axum)
    sharon-core             Domänenmodelle (Projekte, Machine Sessions)
    sharon-store            Projekt-/Asset-Ablage

docs/adr/   Architekturentscheidungen
```

## Bauen

```bash
# GUI
cd luxifer && dotnet build && dotnet test

# Server
cd sharon && cargo build && cargo run   # lauscht auf :7878
```

## Architekturregeln

1. Kernfunktionen laufen vollständig ohne Server (ADR 0001).
2. Fachlogik gehört in `LuxiFer.Core`, nicht in ViewModels.
3. Maschinenbefehle nur über die `MachineCommandQueue` mit State Guards (ADR 0002).
4. Sharon koordiniert nur — er spricht niemals selbst mit einer Maschine.
5. Server-Kommunikation ausschließlich in `LuxiFer.Sync`.
