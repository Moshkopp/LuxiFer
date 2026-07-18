//! UI-freies Shortcut-Modell (ADR 0018): stabile Aktionen, serialisierbare
//! Tastatur-/Maustrigger, Defaults und konfliktfreie Umbelegung.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutAction {
    Save,
    SaveVersion,
    Undo,
    Redo,
    SelectAll,
    Delete,
    Group,
    Ungroup,
    FitView,
    ToolSelect,
    ToolRect,
    ToolEllipse,
    ToolPolygon,
    ToolLine,
    ToolPolyline,
    ToolSpline,
    ToolBezier,
    ToolMeasure,
    ToolNode,
    ToolTrim,
    ToolBridge,
    OpenText,
    ViewProject,
    ViewDesign,
    ViewLaser,
    ViewPreview,
    OpenAssets,
}

impl ShortcutAction {
    pub const ALL: [Self; 27] = [
        Self::Save,
        Self::SaveVersion,
        Self::Undo,
        Self::Redo,
        Self::SelectAll,
        Self::Delete,
        Self::Group,
        Self::Ungroup,
        Self::FitView,
        Self::ToolSelect,
        Self::ToolRect,
        Self::ToolEllipse,
        Self::ToolPolygon,
        Self::ToolLine,
        Self::ToolPolyline,
        Self::ToolSpline,
        Self::ToolBezier,
        Self::ToolMeasure,
        Self::ToolNode,
        Self::ToolTrim,
        Self::ToolBridge,
        Self::OpenText,
        Self::ViewProject,
        Self::ViewDesign,
        Self::ViewLaser,
        Self::ViewPreview,
        Self::OpenAssets,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Save => "Speichern",
            Self::SaveVersion => "Neue Version speichern",
            Self::Undo => "Rückgängig",
            Self::Redo => "Wiederholen",
            Self::SelectAll => "Alles auswählen",
            Self::Delete => "Löschen",
            Self::Group => "Gruppieren",
            Self::Ungroup => "Gruppierung lösen",
            Self::FitView => "Ansicht einpassen",
            Self::ToolSelect => "Auswahlwerkzeug",
            Self::ToolRect => "Rechteck",
            Self::ToolEllipse => "Ellipse",
            Self::ToolPolygon => "Polygon",
            Self::ToolLine => "Linie",
            Self::ToolPolyline => "Polylinie",
            Self::ToolSpline => "Spline",
            Self::ToolBezier => "Bézier",
            Self::ToolMeasure => "Messen",
            Self::ToolNode => "Knoten",
            Self::ToolTrim => "Trimmen",
            Self::ToolBridge => "Haltesteg",
            Self::OpenText => "Text",
            Self::ViewProject => "Projektansicht",
            Self::ViewDesign => "Designansicht",
            Self::ViewLaser => "Laseransicht",
            Self::ViewPreview => "Laser-Vorschau",
            Self::OpenAssets => "Asset-Bibliothek",
        }
    }

    pub fn category(self) -> &'static str {
        match self {
            Self::Save | Self::SaveVersion => "Allgemein",
            Self::Undo
            | Self::Redo
            | Self::SelectAll
            | Self::Delete
            | Self::Group
            | Self::Ungroup
            | Self::FitView => "Bearbeiten",
            Self::ViewProject
            | Self::ViewDesign
            | Self::ViewLaser
            | Self::ViewPreview
            | Self::OpenAssets => "Ansichten",
            _ => "Werkzeuge",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutKey {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Delete,
    Backspace,
    Home,
    End,
    PageUp,
    PageDown,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Escape,
    Enter,
    Space,
}

impl ShortcutKey {
    pub fn label(self) -> &'static str {
        use ShortcutKey as K;
        match self {
            K::A => "A",
            K::B => "B",
            K::C => "C",
            K::D => "D",
            K::E => "E",
            K::F => "F",
            K::G => "G",
            K::H => "H",
            K::I => "I",
            K::J => "J",
            K::K => "K",
            K::L => "L",
            K::M => "M",
            K::N => "N",
            K::O => "O",
            K::P => "P",
            K::Q => "Q",
            K::R => "R",
            K::S => "S",
            K::T => "T",
            K::U => "U",
            K::V => "V",
            K::W => "W",
            K::X => "X",
            K::Y => "Y",
            K::Z => "Z",
            K::Num0 => "0",
            K::Num1 => "1",
            K::Num2 => "2",
            K::Num3 => "3",
            K::Num4 => "4",
            K::Num5 => "5",
            K::Num6 => "6",
            K::Num7 => "7",
            K::Num8 => "8",
            K::Num9 => "9",
            K::F1 => "F1",
            K::F2 => "F2",
            K::F3 => "F3",
            K::F4 => "F4",
            K::F5 => "F5",
            K::F6 => "F6",
            K::F7 => "F7",
            K::F8 => "F8",
            K::F9 => "F9",
            K::F10 => "F10",
            K::F11 => "F11",
            K::F12 => "F12",
            K::Delete => "Delete",
            K::Backspace => "Backspace",
            K::Home => "Home",
            K::End => "End",
            K::PageUp => "PageUp",
            K::PageDown => "PageDown",
            K::ArrowUp => "Pfeil hoch",
            K::ArrowDown => "Pfeil runter",
            K::ArrowLeft => "Pfeil links",
            K::ArrowRight => "Pfeil rechts",
            K::Escape => "Escape",
            K::Enter => "Enter",
            K::Space => "Space",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ShortcutChord {
    pub key: ShortcutKey,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
}

impl ShortcutChord {
    pub const fn key(key: ShortcutKey) -> Self {
        Self {
            key,
            ctrl: false,
            shift: false,
            alt: false,
        }
    }

    pub const fn ctrl(key: ShortcutKey) -> Self {
        Self {
            key,
            ctrl: true,
            shift: false,
            alt: false,
        }
    }

    pub const fn ctrl_shift(key: ShortcutKey) -> Self {
        Self {
            key,
            ctrl: true,
            shift: true,
            alt: false,
        }
    }

    pub fn label(self) -> String {
        let mut parts = Vec::with_capacity(4);
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.shift {
            parts.push("Shift");
        }
        if self.alt {
            parts.push("Alt");
        }
        parts.push(self.key.label());
        parts.join("+")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutMouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ShortcutTrigger {
    Key(ShortcutChord),
    Mouse(ShortcutMouseButton),
}

impl ShortcutTrigger {
    pub fn label(self) -> String {
        match self {
            Self::Key(chord) => chord.label(),
            Self::Mouse(ShortcutMouseButton::Left) => "Linke Maustaste".into(),
            Self::Mouse(ShortcutMouseButton::Right) => "Rechte Maustaste".into(),
            Self::Mouse(ShortcutMouseButton::Middle) => "Mittlere Maustaste".into(),
        }
    }

    pub fn reserved_reason(self) -> Option<&'static str> {
        match self {
            Self::Key(ShortcutChord {
                key: ShortcutKey::Escape,
                ..
            }) => Some("Escape bleibt für Abbrechen reserviert."),
            Self::Key(ShortcutChord {
                key: ShortcutKey::Enter,
                ..
            }) => Some("Enter bleibt für das Abschließen laufender Pfade reserviert."),
            Self::Key(ShortcutChord {
                key: ShortcutKey::Space,
                ..
            }) => Some("Space bleibt als gehaltener Pan-Modifier reserviert."),
            Self::Key(ShortcutChord {
                key: ShortcutKey::F,
                ctrl: true,
                ..
            }) => Some("Ctrl+F bleibt für die Suche reserviert."),
            Self::Key(ShortcutChord {
                key: ShortcutKey::F4,
                alt: true,
                ..
            }) => Some("Alt+F4 wird vom Fenstersystem verwendet."),
            Self::Mouse(ShortcutMouseButton::Left) => {
                Some("Die linke Maustaste bleibt die primäre Canvas-Bedienung.")
            }
            Self::Mouse(ShortcutMouseButton::Middle) => {
                Some("Die mittlere Maustaste bleibt für Pan reserviert.")
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ShortcutBindings(pub BTreeMap<ShortcutAction, Vec<ShortcutTrigger>>);

impl Default for ShortcutBindings {
    fn default() -> Self {
        use ShortcutAction as A;
        use ShortcutKey as K;
        use ShortcutMouseButton as M;
        use ShortcutTrigger as T;
        let mut map = BTreeMap::new();
        for action in A::ALL {
            map.insert(action, Vec::new());
        }
        let defaults = [
            (A::Save, vec![T::Key(ShortcutChord::ctrl(K::S))]),
            (
                A::SaveVersion,
                vec![T::Key(ShortcutChord::ctrl_shift(K::S))],
            ),
            (A::Undo, vec![T::Key(ShortcutChord::ctrl(K::Z))]),
            (
                A::Redo,
                vec![
                    T::Key(ShortcutChord::ctrl_shift(K::Z)),
                    T::Key(ShortcutChord::ctrl(K::Y)),
                ],
            ),
            (A::SelectAll, vec![T::Key(ShortcutChord::ctrl(K::A))]),
            (A::Delete, vec![T::Key(ShortcutChord::key(K::Delete))]),
            (A::Group, vec![T::Key(ShortcutChord::key(K::G))]),
            (A::Ungroup, vec![T::Key(ShortcutChord::ctrl(K::G))]),
            (A::FitView, vec![T::Key(ShortcutChord::key(K::F))]),
            (
                A::ToolSelect,
                vec![T::Key(ShortcutChord::key(K::V)), T::Mouse(M::Right)],
            ),
            (A::ToolRect, vec![T::Key(ShortcutChord::key(K::R))]),
            (A::ToolEllipse, vec![T::Key(ShortcutChord::key(K::E))]),
            (A::ToolPolyline, vec![T::Key(ShortcutChord::key(K::P))]),
            (A::ToolPolygon, vec![T::Key(ShortcutChord::ctrl(K::P))]),
            (A::ToolBezier, vec![T::Key(ShortcutChord::key(K::B))]),
            (A::OpenText, vec![T::Key(ShortcutChord::key(K::T))]),
            (A::ToolTrim, vec![T::Key(ShortcutChord::ctrl(K::T))]),
            (A::ViewProject, vec![T::Key(ShortcutChord::key(K::F1))]),
            (A::ViewDesign, vec![T::Key(ShortcutChord::key(K::F2))]),
            (A::ViewLaser, vec![T::Key(ShortcutChord::key(K::F3))]),
            (A::ViewPreview, vec![T::Key(ShortcutChord::key(K::F4))]),
            (A::OpenAssets, vec![T::Key(ShortcutChord::key(K::F5))]),
        ];
        for (action, triggers) in defaults {
            map.insert(action, triggers);
        }
        Self(map)
    }
}

impl ShortcutBindings {
    pub fn triggers(&self, action: ShortcutAction) -> &[ShortcutTrigger] {
        self.0.get(&action).map(Vec::as_slice).unwrap_or_default()
    }

    pub fn resolve(&self, trigger: ShortcutTrigger) -> Option<ShortcutAction> {
        self.0
            .iter()
            .find_map(|(action, triggers)| triggers.contains(&trigger).then_some(*action))
    }

    pub fn conflict(
        &self,
        action: ShortcutAction,
        trigger: ShortcutTrigger,
    ) -> Option<ShortcutAction> {
        self.resolve(trigger).filter(|existing| *existing != action)
    }

    pub fn reassign(
        &mut self,
        action: ShortcutAction,
        trigger: ShortcutTrigger,
    ) -> Result<(), String> {
        if let Some(reason) = trigger.reserved_reason() {
            return Err(reason.into());
        }
        for (candidate, triggers) in &mut self.0 {
            if *candidate != action {
                triggers.retain(|value| *value != trigger);
            }
        }
        let triggers = self.0.entry(action).or_default();
        if !triggers.contains(&trigger) {
            triggers.push(trigger);
        }
        Ok(())
    }

    pub fn remove(&mut self, action: ShortcutAction, trigger: ShortcutTrigger) {
        if let Some(triggers) = self.0.get_mut(&action) {
            triggers.retain(|value| *value != trigger);
        }
    }

    pub fn reset_action(&mut self, action: ShortcutAction) {
        self.0
            .insert(action, Self::default().triggers(action).to_vec());
    }

    pub fn normalize(&mut self) {
        for action in ShortcutAction::ALL {
            self.0.entry(action).or_default();
        }
        for triggers in self.0.values_mut() {
            triggers.retain(|trigger| trigger.reserved_reason().is_none());
            let mut seen = BTreeSet::new();
            triggers.retain(|trigger| seen.insert(*trigger));
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        let mut owners = BTreeMap::new();
        for (action, triggers) in &self.0 {
            for trigger in triggers {
                if let Some(reason) = trigger.reserved_reason() {
                    return Err(reason.into());
                }
                if let Some(previous) = owners.insert(*trigger, *action) {
                    if previous != *action {
                        return Err(format!(
                            "{} ist sowohl {} als auch {} zugewiesen.",
                            trigger.label(),
                            previous.label(),
                            action.label()
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_enthalten_bestaetigte_belegungen() {
        let b = ShortcutBindings::default();
        assert_eq!(
            b.resolve(ShortcutTrigger::Key(ShortcutChord::key(ShortcutKey::P))),
            Some(ShortcutAction::ToolPolyline)
        );
        assert_eq!(
            b.resolve(ShortcutTrigger::Key(ShortcutChord::ctrl(ShortcutKey::P))),
            Some(ShortcutAction::ToolPolygon)
        );
        assert_eq!(
            b.resolve(ShortcutTrigger::Mouse(ShortcutMouseButton::Right)),
            Some(ShortcutAction::ToolSelect)
        );
        assert_eq!(
            b.resolve(ShortcutTrigger::Key(ShortcutChord::key(ShortcutKey::F5))),
            Some(ShortcutAction::OpenAssets)
        );
        assert_eq!(b.triggers(ShortcutAction::Redo).len(), 2);
        b.validate().unwrap();
    }

    #[test]
    fn umbelegen_entfernt_nur_den_kollidierenden_trigger() {
        let mut b = ShortcutBindings::default();
        let trigger = ShortcutTrigger::Key(ShortcutChord::ctrl(ShortcutKey::Y));
        b.reassign(ShortcutAction::Group, trigger).unwrap();
        assert_eq!(b.resolve(trigger), Some(ShortcutAction::Group));
        assert_eq!(
            b.triggers(ShortcutAction::Redo),
            &[ShortcutTrigger::Key(ShortcutChord::ctrl_shift(
                ShortcutKey::Z
            ))]
        );
    }

    #[test]
    fn reservierte_trigger_werden_abgewiesen() {
        let mut b = ShortcutBindings::default();
        assert!(b
            .reassign(
                ShortcutAction::Save,
                ShortcutTrigger::Key(ShortcutChord::key(ShortcutKey::Escape))
            )
            .is_err());
        assert!(b
            .reassign(
                ShortcutAction::Save,
                ShortcutTrigger::Mouse(ShortcutMouseButton::Middle)
            )
            .is_err());
    }
}
