<script lang="ts" module>
  // Zentrale Icon-Sammlung im Lucide-Stil (MIT), inline als SVG-Pfade — keine
  // externe Abhaengigkeit, faerbt sich per currentColor. 24×24-viewBox,
  // stroke-basiert. Icons ohne direktes Lucide-Pendant (Untersetzer, Haltesteg,
  // Formen) sind im gleichen Stil selbst gezeichnet.
  //
  // Jeder Eintrag ist der innere SVG-Inhalt (Pfade/Formen). Das umgebende <svg>
  // mit den gemeinsamen Attributen liefert die Komponente unten.
  export type IconName =
    // Werkzeuge
    | "select" | "rect" | "ellipse" | "polygon" | "line" | "polyline"
    | "spline" | "bezier" | "text" | "node" | "trim" | "bridge" | "boolean"
    | "fillet" | "pattern-fill" | "offset" | "measure" | "mirror-h" | "mirror-v"
    | "coaster-rect" | "coaster-circle"
    // Formen im Polygon-Flyout
    | "tri" | "quad" | "penta" | "hex" | "octa" | "star" | "sun" | "gear" | "heart"
    // Header-/Panel-Aktionen
    | "undo" | "redo" | "trash" | "gcode" | "pause" | "stop" | "home" | "frame"
    | "contour" | "send" | "power" | "speed" | "wind" | "eye" | "eye-off" | "lock"
    | "settings" | "logo";

  export const ICONS: Record<IconName, string> = {
    // ── Werkzeuge ──────────────────────────────────────────────────────────
    select: `<path d="m3 3 7.07 16.97 2.51-7.39 7.39-2.51L3 3z"/><path d="m13 13 6 6"/>`,
    rect: `<rect x="3" y="5" width="18" height="14" rx="1"/>`,
    ellipse: `<ellipse cx="12" cy="12" rx="9" ry="6.5"/>`,
    polygon: `<path d="M12 2 21 8.5 17.5 19h-11L3 8.5z"/>`,
    line: `<path d="M4 20 20 4"/>`,
    polyline: `<path d="M3 17 9 9l4 3 8-9"/>`,
    spline: `<path d="M4 18c6 0 4-12 10-12" /><circle cx="4" cy="18" r="1.6"/><circle cx="14" cy="6" r="1.6"/>`,
    bezier: `<path d="M3 16c4 0 5-8 9-8s5 8 9 8"/><circle cx="3" cy="16" r="1.4"/><circle cx="21" cy="16" r="1.4"/>`,
    text: `<path d="M4 6V5h16v1"/><path d="M12 5v14"/><path d="M9 19h6"/>`,
    node: `<circle cx="12" cy="12" r="2.4"/><path d="M12 2v5M12 17v5M2 12h5M17 12h5"/>`,
    trim: `<circle cx="6" cy="7" r="2.4"/><circle cx="6" cy="17" r="2.4"/><path d="M8 8.5 20 17M8 15.5 20 7"/>`,
    bridge: `<path d="M3 16v-2a9 9 0 0 1 18 0v2"/><path d="M3 16h18"/><path d="M9 16v4M15 16v4"/>`,
    boolean: `<circle cx="9" cy="12" r="6"/><circle cx="15" cy="12" r="6"/>`,
    fillet: `<path d="M5 19V11a6 6 0 0 1 6-6h8"/>`,
    "pattern-fill": `<rect x="3" y="3" width="18" height="18" rx="1"/><path d="M3 9h18M3 15h18M9 3v18M15 3v18"/>`,
    offset: `<rect x="7" y="7" width="12" height="12" rx="1"/><path d="M4 16V4h12"/>`,
    measure: `<path d="M3 8h18v8H3z"/><path d="M7 8v3M11 8v4M15 8v3M19 8v4"/>`,
    "mirror-h": `<path d="M12 3v18"/><path d="M8 7 3 12l5 5z"/><path d="M16 7l5 5-5 5z"/>`,
    "mirror-v": `<path d="M3 12h18"/><path d="M7 8 12 3l5 5z"/><path d="M7 16l5 5 5-5z"/>`,
    "coaster-rect": `<rect x="3" y="4" width="8" height="7" rx="1"/><rect x="13" y="4" width="8" height="7" rx="1"/><rect x="3" y="13" width="8" height="7" rx="1"/><rect x="13" y="13" width="8" height="7" rx="1"/>`,
    "coaster-circle": `<circle cx="7" cy="7.5" r="3.5"/><circle cx="17" cy="7.5" r="3.5"/><circle cx="7" cy="16.5" r="3.5"/><circle cx="17" cy="16.5" r="3.5"/>`,

    // ── Formen (Polygon-Flyout) ────────────────────────────────────────────
    tri: `<path d="M12 4 21 19H3z"/>`,
    quad: `<path d="M12 3 21 12 12 21 3 12z"/>`,
    penta: `<path d="M12 3 21 10 17.5 21h-11L3 10z"/>`,
    hex: `<path d="M8 4h8l4 8-4 8H8l-4-8z"/>`,
    octa: `<path d="M8 3h8l5 5v8l-5 5H8l-5-5V8z"/>`,
    star: `<path d="M12 3l2.6 6h6.4l-5 4.5 1.9 6.5L12 16.8 6.1 20l1.9-6.5-5-4.5h6.4z"/>`,
    sun: `<circle cx="12" cy="12" r="4"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3M5 5l2 2M17 17l2 2M19 5l-2 2M7 17l-2 2"/>`,
    gear: `<circle cx="12" cy="12" r="3"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.5 4.5 6.6 6.6M17.4 17.4l2.1 2.1M19.5 4.5 17.4 6.6M6.6 17.4 4.5 19.5"/>`,
    heart: `<path d="M12 20s-7-4.5-9-9c-1.5-3.5 1-6.5 4-6.5 2 0 3.5 1.5 5 3.5 1.5-2 3-3.5 5-3.5 3 0 5.5 3 4 6.5-2 4.5-9 9-9 9z"/>`,

    // ── Aktionen ───────────────────────────────────────────────────────────
    undo: `<path d="M9 14 4 9l5-5"/><path d="M4 9h11a6 6 0 0 1 0 12h-3"/>`,
    redo: `<path d="m15 14 5-5-5-5"/><path d="M20 9H9a6 6 0 0 0 0 12h3"/>`,
    trash: `<path d="M3 6h18M8 6V4h8v2M6 6l1 14h10l1-14"/>`,
    gcode: `<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M8 9l-2 3 2 3M16 9l2 3-2 3M13 8l-2 8"/>`,
    pause: `<rect x="6" y="5" width="4" height="14" rx="1"/><rect x="14" y="5" width="4" height="14" rx="1"/>`,
    stop: `<rect x="5" y="5" width="14" height="14" rx="2"/>`,
    home: `<path d="M3 11 12 3l9 8"/><path d="M5 10v10h14V10"/>`,
    frame: `<rect x="4" y="4" width="16" height="16" rx="1" stroke-dasharray="3 3"/>`,
    contour: `<path d="M7 4h10a3 3 0 0 1 3 3v10a3 3 0 0 1-3 3H7a3 3 0 0 1-3-3V7a3 3 0 0 1 3-3z"/><circle cx="12" cy="12" r="3"/>`,
    send: `<path d="M22 2 11 13"/><path d="M22 2 15 22l-4-9-9-4z"/>`,
    power: `<path d="M13 2 4 14h7l-1 8 9-12h-7z"/>`,
    speed: `<path d="M12 3a9 9 0 1 0 9 9"/><path d="M12 12 17 8"/>`,
    wind: `<path d="M3 8h11a3 3 0 1 0-3-3M3 12h15a3 3 0 1 1-3 3M3 16h9"/>`,
    eye: `<path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7-10-7-10-7z"/><circle cx="12" cy="12" r="3"/>`,
    "eye-off": `<path d="M9.9 5A9.8 9.8 0 0 1 12 5c6 0 10 7 10 7a13 13 0 0 1-2 2.5M6 6C3 8 2 12 2 12s4 7 10 7a9.6 9.6 0 0 0 4-.9"/><path d="M3 3l18 18"/>`,
    lock: `<rect x="5" y="11" width="14" height="9" rx="1"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/>`,
    settings: `<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.6 1.6 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.6 1.6 0 0 0-1.8-.3 1.6 1.6 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.6 1.6 0 0 0-1-1.5 1.6 1.6 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.6 1.6 0 0 0 .3-1.8 1.6 1.6 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.6 1.6 0 0 0 1.5-1 1.6 1.6 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.6 1.6 0 0 0 1.8.3H9a1.6 1.6 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.6 1.6 0 0 0 1 1.5 1.6 1.6 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.6 1.6 0 0 0-.3 1.8V9a1.6 1.6 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.6 1.6 0 0 0-1.5 1z"/>`,
    // LuxiFer-Platzhalter-Logo: stilisierter Laser-Funke / Strahl. Leicht ersetzbar.
    logo: `<path d="M12 2v6"/><path d="M12 16v6"/><path d="M2 12h6"/><path d="M16 12h6"/><circle cx="12" cy="12" r="3.2" fill="currentColor" stroke="none"/><path d="M5.6 5.6l3 3M15.4 15.4l3 3M18.4 5.6l-3 3M8.6 15.4l-3 3"/>`,
  };
</script>

<script lang="ts">
  let {
    name,
    size = 18,
    // `fill`: Icon skaliert mit der Schriftgroesse (1em) des Buttons statt
    // fixer Pixel — fuer Werkzeug-/Anordnen-Buttons, die sich dem Panel anpassen.
    fill = false,
  }: {
    name: IconName;
    size?: number;
    fill?: boolean;
  } = $props();
</script>

<svg
  class:fill
  width={fill ? undefined : size}
  height={fill ? undefined : size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {@html ICONS[name]}
</svg>

<style>
  svg {
    display: block;
    flex-shrink: 0;
  }
  /* Skaliert mit der Schriftgroesse des umgebenden Buttons. */
  svg.fill {
    width: 1.2em;
    height: 1.2em;
  }
</style>
