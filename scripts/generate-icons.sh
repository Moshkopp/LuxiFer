#!/usr/bin/env bash
# Erzeugt die App-Icon-Staffel aus dem Branding-Sheet
# (docs/branding/luxifer-branding-sheet.png, 1536x1024).
#
# Größenstaffel wie im Sheet vorgesehen:
#   256/512  Hauptlogo (detailliert)
#   48–128   Desktop-Icon (vereinfachte Flügel)
#   16–32    Favicon-Motiv (nur Ring + Strahl, fette Striche)
#
# Die Crop-Koordinaten beziehen sich auf das Sheet; wenn das Sheet neu
# generiert wird, müssen sie neu vermessen werden (fuzz-trim je Kachel).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SHEET="$ROOT_DIR/docs/branding/luxifer-branding-sheet.png"
OUT="$ROOT_DIR/luxifer/native/assets/icon"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

mkdir -p "$OUT"

# Abgerundeter dunkler Grund mit leichtem Verlauf (wie die Sheet-Kacheln).
rounded_bg() { # $1=Größe $2=Ausgabe
  local s="$1" r=$(($1 * 22 / 100))
  magick -size "${s}x${s}" gradient:'#191919'-'#0b0c0c' \
    \( -size "${s}x${s}" xc:none -fill white \
       -draw "roundrectangle 0,0,$((s - 1)),$((s - 1)),$r,$r" \) \
    -alpha off -compose CopyOpacity -composite "$2"
}

# 256/512: Hauptlogo, direkt rund maskiert.
magick "$SHEET" -crop 535x535+50+50 +repage \
  \( -size 535x535 xc:none -fill white -draw "roundrectangle 0,0,534,534,118,118" \) \
  -alpha off -compose CopyOpacity -composite "$TMP/haupt.png"
magick "$TMP/haupt.png" -resize 512x512 "$OUT/luxifer-512.png"
magick "$TMP/haupt.png" -resize 256x256 "$OUT/luxifer-256.png"

# 48–128: Flügel-Motiv aus der Desktop-Icon-Kachel auf eigenem Grund.
magick "$SHEET" -crop 153x167+846+198 +repage "$TMP/fluegel.png"
rounded_bg 512 "$TMP/bg512.png"
magick "$TMP/bg512.png" \( "$TMP/fluegel.png" -resize x400 \) \
  -gravity center -compose over -composite "$TMP/fluegel-512.png"
for s in 48 64 128; do
  magick "$TMP/fluegel-512.png" -resize "${s}x${s}" "$OUT/luxifer-$s.png"
done

# 16–32: Favicon-Motiv, je Größe frisch komponiert (kaum skaliert = scharf).
magick "$SHEET" -crop 37x46+1214+270 +repage "$TMP/minimal.png"
for s in 16 24 32; do
  rounded_bg "$s" "$TMP/bg$s.png"
  magick "$TMP/bg$s.png" \
    \( "$TMP/minimal.png" -resize x$((s * 88 / 100)) -modulate 112 \) \
    -gravity center -compose over -composite "$OUT/luxifer-$s.png"
done

echo "» Icons erzeugt in $OUT"
