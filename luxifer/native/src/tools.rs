//! Werkzeug-Zustand des Canvas. Reines UI-Anliegen (welches Tool ist aktiv,
//! welcher Zug läuft gerade) — die eigentliche Mutation macht immer der Core.

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Rect,
    Ellipse,
    Polygon,
    Line,
    Polyline,
    Spline,
    Bezier,
    Measure,
    Node,
}

/// Haupt-Ansicht (Reiterleiste oben), analog zur Tauri-App.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum View {
    Projekt,
    Design,
    Laser,
}

impl View {
    pub fn label(self) -> &'static str {
        match self {
            View::Projekt => "Projekt",
            View::Design => "Design",
            View::Laser => "Laser",
        }
    }
}

/// Laser-Bedien-Zustand (UI-seitig). Ohne echten Treiber-Anschluss im nativen
/// Umbau — die Aktionen loggen vorerst; die Treiber-Verdrahtung kommt später.
pub struct LaserUi {
    pub jog_step: f64,
    pub jog_speed: f64,
    /// Job-Nullpunkt-Anker (0..8, 4 = Mitte).
    pub anchor: usize,
    pub selection_only: bool,
    /// Startmodus des Jobs (Absolut / aktuelle Position / Benutzerursprung).
    pub start_mode: luxifer_core::StartMode,
}

impl Default for LaserUi {
    fn default() -> Self {
        Self {
            jog_step: 10.0,
            jog_speed: 100.0,
            anchor: 4,
            selection_only: false,
            start_mode: luxifer_core::StartMode::Absolut,
        }
    }
}

impl Tool {
    pub fn label(self) -> &'static str {
        match self {
            Tool::Select => "Auswahl",
            Tool::Rect => "Rechteck",
            Tool::Ellipse => "Ellipse",
            Tool::Polygon => "Polygon",
            Tool::Line => "Linie",
            Tool::Polyline => "Polylinie",
            Tool::Spline => "Spline",
            Tool::Bezier => "Bézier",
            Tool::Measure => "Messen",
            Tool::Node => "Knoten",
        }
    }

    /// Icon-Name (siehe icons.rs).
    pub fn icon(self) -> &'static str {
        match self {
            Tool::Select => "select",
            Tool::Rect => "rect",
            Tool::Ellipse => "ellipse",
            Tool::Polygon => "polygon",
            Tool::Line => "line",
            Tool::Polyline => "polyline",
            Tool::Spline => "spline",
            Tool::Bezier => "bezier",
            Tool::Measure => "measure",
            Tool::Node => "node",
        }
    }
}

/// Sofort-Befehl auf der Auswahl (kein Zeichenmodus). Entspricht den `action`-
/// Werkzeugen der Tauri-ToolsPanel + den Arrange-Aktionen.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToolAction {
    Boolean,
    Fillet,
    Offset,
    PatternFill,
    Bridge,
}

/// Laufende Maus-Geste im Canvas (zwischen Press und Release).
pub enum Drag {
    None,
    /// Canvas verschieben (mittlere Maustaste oder Leertaste+links).
    Pan,
    /// Auswahl-Rechteck aufziehen (Welt-Startpunkt).
    Marquee {
        start: [f64; 2],
    },
    /// Selektierte Shapes verschieben (letzter Welt-Punkt).
    MoveShapes {
        last: [f64; 2],
    },
    /// Neues Rechteck/Ellipse aufziehen (Welt-Startpunkt).
    DrawBox {
        start: [f64; 2],
    },
    /// Auswahl über ein Handle skalieren. `handle` = gezogene Ecke/Kante,
    /// `start_box` = Auswahl-BBox bei Drag-Beginn, `orig` = Snapshot der
    /// selektierten Shapes bei Drag-Beginn (Index + Shape). So wird bei jedem
    /// Maus-Schritt vom Ausgangszustand skaliert statt vom bereits skalierten
    /// (sonst schaukelt sich die Größe exponentiell auf).
    Resize {
        handle: luxifer_core::Handle,
        start_box: luxifer_core::BBox,
        orig: Vec<(usize, luxifer_core::Shape)>,
    },
    /// Auswahl drehen. `pivot` = Mittelpunkt, `orig` = Snapshot bei Drag-Beginn,
    /// `start_angle` = Mauswinkel bei Beginn. Rotation immer vom Ausgangszustand.
    Rotate {
        pivot: [f64; 2],
        start_angle: f64,
        orig: Vec<(usize, luxifer_core::Shape)>,
    },
}
