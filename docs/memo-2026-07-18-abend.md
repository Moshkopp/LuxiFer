# Memo für den Abend – 2026-07-18

## 1. Anpassbare Tastenkürzel

Die bisher fest im nativen Editor hinterlegten Shortcuts sollen über die
Einstellungen anpassbar werden.

Für die Planung klären:

- zentrale, typisierte Liste aller belegbaren Aktionen;
- persistente Zuordnung in den UI-Einstellungen;
- Eingabefeld zum Aufzeichnen einer Tastenkombination statt freier Texteingabe;
- Erkennung und verständliche Anzeige von Doppelbelegungen;
- reservierte beziehungsweise nicht überschreibbare Systemkombinationen;
- Schaltfläche zum Zurücksetzen einzelner Einträge und aller Shortcuts;
- Migration bestehender Settings und stabile Standardbelegung;
- Fokusregel: Shortcuts dürfen in Textfeldern und modalen Dialogen weiterhin
  nicht versehentlich den Canvas verändern.

Vor der Umsetzung den aktuellen Weg
`KeyboardInput -> resolve_shortcut -> Shortcut -> App` erfassen und festlegen,
welche Teile weiterhin statisch bleiben müssen. Anschließend Tests für
Standardwerte, benutzerdefinierte Belegung, Konflikte, Persistenz und
Fokus-/Modal-Sperren vorsehen.

## 2. Material-Templates für Feed und Speed

Vor der Implementierung einen eigenen ADR erstellen und die fachliche Grenze
planen. Ziel sind wiederverwendbare Materialvorlagen, die sinnvolle
Geschwindigkeits- und Leistungswerte für Laser-Layer vorbelegen.

Der ADR soll mindestens entscheiden:

- ob Vorlagen global, pro Laserprofil oder zusätzlich projektspezifisch sind;
- Identität und Felder einer Vorlage: Material, Stärke, Prozessart
  (Schnitt/Gravur), Geschwindigkeit, Leistung und optionale Zusatzparameter;
- Einheit und Bedeutung von „Feed/Speed“ je Treiber sowie die Umrechnung an der
  treiberneutralen Grenze;
- Verhältnis zwischen Vorlage und Layer: Kopie der Werte oder dauerhafte
  Referenz mit kontrollierter Aktualisierung;
- Verhalten bei manueller Änderung eines aus einer Vorlage erzeugten Layers;
- eingebaute Standardvorlagen gegenüber benutzerdefinierten Vorlagen;
- Import, Export, Duplizieren, Umbenennen und Löschen;
- Validierung anhand der Fähigkeiten und Grenzwerte des aktiven Laserprofils;
- Versions-/Migrationsstrategie für gespeicherte Vorlagen;
- klare Sicherheitskennzeichnung: Vorlagen sind Startwerte und keine Garantie
  für ein bestimmtes Material- oder Maschinenergebnis.

Geplanter Ablauf:

1. Bestehendes Layer-Parameter- und Laserprofil-Modell sowie die
   ThorBurn/ThorLaser-Referenz untersuchen.
2. ADR mit Datenhoheit, Anwendungssemantik, Treibergrenze und UX-Ablauf
   verfassen.
3. Datenmodell und Persistenz mit Tests umsetzen.
4. Materialvorlagen-Verwaltung in den Einstellungen ergänzen.
5. Vorlagenauswahl im Layer-Parameterdialog integrieren.
6. Anwendung, manuelle Abweichung, Migration und Projekt-Roundtrip testen.

Noch keine Implementierung beginnen, bevor der ADR abgestimmt ist.
