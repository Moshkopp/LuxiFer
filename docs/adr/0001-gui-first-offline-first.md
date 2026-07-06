# ADR 0001: GUI-first, Offline-first

## Status
Akzeptiert — 2026-07-06

## Kontext
LuxiFer soll eine Desktop-Anwendung zur Laser-Steuerung sein, optional erweitert
um einen Sync-Server. Es muss entschieden werden, wo die Fachlogik lebt und
welche Rolle der Server spielt.

## Entscheidung
- **Die GUI (LuxiFer) ist das Produkt.** Alle Kernfunktionen (Projekte, Canvas,
  Assets, Fonts, Preview, Ruida-Steuerung) funktionieren vollständig offline.
- **Der Server (Sharon) ist optional** und ausschließlich Koordinator:
  Sync, zentrale Bibliotheken, Machine-Session-Vergabe. Er spricht niemals
  selbst mit einer Maschine.
- **Fachlogik liegt in LuxiFer.Core**, nicht in der Avalonia-UI-Schicht.

## Konsequenzen
- Zwei getrennte Codebasen: `luxifer/` (C#/.NET, Avalonia) und `sharon/` (Rust).
- Die GUI darf keine Annahme über Server-Verfügbarkeit machen; `LuxiFer.Sync`
  kapselt die gesamte Server-Kommunikation und ist der einzige Ort dafür.
