# ADR 0017: Konfigurierbares GPU-Antialiasing

## Status

Angenommen und umgesetzt am 2026-07-17.

## Kontext

Der native Canvas zeichnet Konturen bereits als GPU-Dreiecke. Harte
Fragmentkanten bleiben bei Diagonalen und Kurven sichtbar. Gleichzeitig reicht
die Zielhardware von integrierten Mini-PC-GPUs bis zu leistungsfähigen
Grafikkarten. Eine fest erzwungene hohe MSAA-Stufe wäre deshalb ungeeignet.

## Entscheidung

- Konturen, Raster und Overlays erhalten optionales analytisches Shader-AA.
- Flächenkanten verwenden optional MSAA mit den Einstellungen
  `Aus`, `2×`, `4×`, `8×` und `16×`.
- Standard sind analytisches Linien-AA und `4×` MSAA.
- Unterstützen Farb- oder Stencil-Format die gewählte Stufe nicht, verwendet
  LuxiFer automatisch die höchste unterstützte niedrigere Stufe.
- Canvas-, Bild-, Fill-, Stencil- und egui-Pipelines verwenden dieselbe
  effektive Sample-Zahl.
- MSAA wird in ein separates Multisample-Farbziel gerendert und erst im letzten
  Pass in die Surface-Texture aufgelöst.
- Änderungen gelten nach dem nächsten Programmstart, weil die Renderpipelines
  beim Start erzeugt werden.

## Folgen

Schwächere Rechner können beide Verfahren abschalten. Leistungsfähige Rechner
können höhere Stufen nutzen, sofern die GPU sie für beide Renderformate
unterstützt. Hohe Stufen erhöhen GPU-Speicherbedarf und Speicherbandbreite,
nicht aber die Core- oder Vertex-Geometrieberechnung.
