//! Werkzeug-Zustand des Canvas. Reines UI-Anliegen (welches Tool ist aktiv,
//! welcher Zug läuft gerade) — die eigentliche Mutation macht immer der Core.

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Rect,
    Ellipse,
    Polygon,
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
