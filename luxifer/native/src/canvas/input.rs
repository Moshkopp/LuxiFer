//! Eingabe-Übersetzung für den Canvas: physische Tasten → typisierte `Key` und
//! die reinen Zeiger-Events (Bewegen/Klicken/Scrollen) auf `CanvasState`-Gesten.
//!
//! Fenster-/GPU-Ereignisse (Resize) und die Tastatur-Koordination
//! (`apply_shortcut`, das auch Projekt/Dialoge berührt) bleiben im App-Root.

use luxifer_application::EditorSession;
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::keyboard::KeyCode;

use crate::tools::Drag;
use crate::tools::Key;

use super::state::CanvasState;

/// Übersetzt die für Shortcuts relevanten physischen Tasten in das
/// UI-unabhängige `tools::Key`. Alles andere ignoriert die Shortcut-Ebene.
pub fn map_keycode(code: KeyCode) -> Option<Key> {
    Some(match code {
        KeyCode::KeyS => Key::S,
        KeyCode::Delete | KeyCode::Backspace => Key::Delete,
        KeyCode::Escape => Key::Escape,
        KeyCode::Enter => Key::Enter,
        KeyCode::Space => Key::Space,
        KeyCode::KeyV => Key::V,
        KeyCode::KeyR => Key::R,
        KeyCode::KeyE => Key::E,
        KeyCode::KeyP => Key::P,
        KeyCode::KeyZ => Key::Z,
        KeyCode::KeyY => Key::Y,
        _ => return None,
    })
}

impl CanvasState {
    /// Read-only Navigation der Preview: Mittelmaus-Pan und Mausrad-Zoom,
    /// keinerlei Auswahl- oder Zeichenmutation.
    pub fn handle_preview_pointer_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let new = [position.x as f32, position.y as f32];
                if matches!(self.drag, Drag::Pan) {
                    self.cam
                        .pan_pixels(new[0] - self.cursor[0], new[1] - self.cursor[1]);
                }
                self.cursor = new;
            }
            WindowEvent::MouseInput {
                state,
                button: winit::event::MouseButton::Middle,
                ..
            } => {
                self.drag = if *state == ElementState::Pressed {
                    Drag::Pan
                } else {
                    Drag::None
                };
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let steps = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                self.cam.zoom_at(1.12_f32.powf(steps), self.cursor);
            }
            _ => {}
        }
    }

    /// Behandelt ein reines Canvas-Zeiger-Event (Bewegen/Klicken/Scrollen) und
    /// meldet dessen Ergebnis (Shape entstanden, Doppelklick auf Shape). Für
    /// andere Event-Arten (Tastatur, Resize) ein leeres Ergebnis; die behandelt
    /// der Root.
    pub fn handle_pointer_event(
        &mut self,
        session: &mut EditorSession,
        event: &WindowEvent,
    ) -> super::gestures::PointerOutcome {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let new = [position.x as f32, position.y as f32];
                self.on_cursor_move(session, new);
                Default::default()
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.on_mouse(session, *button, *state == ElementState::Pressed)
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let s = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                self.cam.zoom_at(1.12_f32.powf(s), self.cursor);
                Default::default()
            }
            _ => Default::default(),
        }
    }
}
