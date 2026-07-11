//! Werkzeug-Zustand des Canvas. Reines UI-Anliegen (welches Tool ist aktiv,
//! welcher Zug läuft gerade) — die eigentliche Mutation macht immer der Core.

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Rect,
    Ellipse,
    Polygon,
}

/// Rechter Reiter: Design-Inspektor (Ebenen/Palette) oder Laser-Bedienung.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Design,
    Laser,
}

/// Laser-Bedien-Zustand (UI-seitig). Ohne echten Treiber-Anschluss im nativen
/// Umbau — die Aktionen loggen vorerst; die Treiber-Verdrahtung kommt später.
pub struct LaserUi {
    pub jog_step: f64,
    pub jog_speed: f64,
    /// Job-Nullpunkt-Anker (0..8, 4 = Mitte).
    pub anchor: usize,
    pub selection_only: bool,
    /// „Verbunden"-Zustand (Demo-Umschalter, bis der Treiber angebunden ist).
    pub connected: bool,
}

impl Default for LaserUi {
    fn default() -> Self {
        Self {
            jog_step: 10.0,
            jog_speed: 100.0,
            anchor: 4,
            selection_only: false,
            connected: false,
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
        }
    }
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
}
