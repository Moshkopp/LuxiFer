use super::EditorSession;
use crate::AppError;
use std::collections::HashMap;

impl EditorSession {
    /// Kopiert die aktuelle, bereits auf Gruppen erweiterte Auswahl in die
    /// sitzungsinterne Objekt-Zwischenablage. Der Editorzustand bleibt dabei
    /// unveraendert und erhaelt deshalb keinen Undo-Schritt.
    pub fn copy_selected(&mut self) -> Result<usize, AppError> {
        self.require_selection("Kopieren")?;
        self.clipboard = self
            .state
            .selected
            .iter()
            .filter_map(|&index| {
                let shape = self.state.shapes.get(index)?.clone();
                let layer = self.state.layers.get(shape.layer_id)?.clone();
                Some((shape, layer))
            })
            .collect();
        self.paste_generation = 0;
        if self.clipboard.is_empty() {
            return Err(AppError::new(
                "clipboard_empty",
                "Die Auswahl enthält keine kopierbaren Objekte.",
            ));
        }
        Ok(self.clipboard.len())
    }

    /// Fuegt den zuletzt kopierten Objektbestand mit sichtbarem, bei jedem
    /// Einfuegen wachsendem Versatz ein. Layerbezug und interne Gruppen bleiben
    /// erhalten, erhalten aber neue IDs. Der gesamte Vorgang ist ein Undo-Schritt.
    pub fn paste(&mut self) -> Result<Vec<usize>, AppError> {
        if self.clipboard.is_empty() {
            return Err(AppError::new(
                "clipboard_empty",
                "Es wurden noch keine Objekte kopiert.",
            ));
        }
        self.state.push_undo();
        self.paste_generation = self.paste_generation.saturating_add(1);
        let offset = 5.0 * f64::from(self.paste_generation);
        let mut groups = HashMap::<u32, u32>::new();
        let mut fill_groups = HashMap::<u32, u32>::new();
        let mut next_group = self
            .state
            .shapes
            .iter()
            .filter_map(|shape| shape.group_id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let mut next_fill_group = self
            .state
            .shapes
            .iter()
            .filter_map(|shape| shape.fill_group_id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let mut inserted = Vec::with_capacity(self.clipboard.len());
        for (source, source_layer) in &self.clipboard {
            let layer_id = self
                .state
                .layers
                .iter()
                .position(|layer| layer == source_layer)
                .unwrap_or_else(|| {
                    self.state.layers.push(source_layer.clone());
                    self.state.layers.len() - 1
                });
            let mut shape = source.clone();
            shape.layer_id = layer_id;
            shape.group_id = shape.group_id.map(|old| {
                *groups.entry(old).or_insert_with(|| {
                    let assigned = next_group;
                    next_group = next_group.saturating_add(1);
                    assigned
                })
            });
            shape.fill_group_id = shape.fill_group_id.map(|old| {
                *fill_groups.entry(old).or_insert_with(|| {
                    let assigned = next_fill_group;
                    next_fill_group = next_fill_group.saturating_add(1);
                    assigned
                })
            });
            shape.translate(offset, offset);
            inserted.push(self.state.shapes.len());
            self.state.shapes.push(shape);
        }
        self.state.selected = inserted.clone();
        self.state.dirty = true;
        Ok(inserted)
    }

    pub fn set_selection(&mut self, indices: Vec<usize>) {
        self.state.selected = indices
            .into_iter()
            .filter(|&index| index < self.state.shapes.len())
            .collect();
        self.state.expand_selection_to_groups();
    }

    /// Stellt waehrend einer laufenden direkten Manipulation den geometrischen
    /// Ausgangszustand wieder her. Undo und Dirty-State bleiben Eigentum der
    /// Session; die GUI verwaltet lediglich den kurzlebigen Gesten-Snapshot.
    pub fn restore_shape_snapshot(&mut self, shapes: &[(usize, studio_core::Shape)]) {
        debug_assert!(self.edit_active(), "restore_shape_snapshot ohne begin_edit");
        for (index, shape) in shapes {
            if let Some(target) = self.state.shapes.get_mut(*index) {
                *target = shape.clone();
            }
        }
    }

    pub fn select_at(&mut self, x: f64, y: f64, tolerance: f64, additive: bool) -> Option<usize> {
        let hit = self.state.hit_test(x, y, tolerance);
        match hit {
            Some(index) if additive => {
                if let Some(position) = self.state.selected.iter().position(|&item| item == index) {
                    self.state.selected.remove(position);
                } else {
                    self.state.selected.push(index);
                }
            }
            Some(index) if !self.state.selected.contains(&index) => {
                self.state.selected = vec![index];
            }
            None if !additive => self.state.selected.clear(),
            _ => {}
        }
        self.state.expand_selection_to_groups();
        hit
    }

    pub fn select_rect(&mut self, start: [f64; 2], end: [f64; 2], inverted: bool) {
        let crossing = studio_core::interact::marquee_crossing(start[0], end[0], inverted);
        self.state
            .select_in_rect(start[0], start[1], end[0], end[1], crossing);
        self.state.expand_selection_to_groups();
    }

    pub fn clear_selection(&mut self) {
        self.state.selected.clear();
    }

    /// Skaliert die aktuelle Auswahl numerisch auf die gewünschte gemeinsame
    /// Breite/Höhe. Die linke obere Ecke der Auswahlbox bleibt stehen und die
    /// gesamte Änderung bildet genau einen Undo-Schritt.
    pub fn resize_selection(&mut self, width: f64, height: f64) -> Result<(), AppError> {
        let Some(start) = self.state.selection_bbox() else {
            return Err(AppError::new(
                "selection_required",
                "Zum Skalieren muss mindestens ein Objekt ausgewählt sein.",
            ));
        };
        if !width.is_finite() || !height.is_finite() || width < 0.1 || height < 0.1 {
            return Err(AppError::new(
                "invalid_selection_size",
                "Breite und Höhe müssen mindestens 0,1 mm betragen.",
            ));
        }
        let target = studio_core::BBox::new(start.x, start.y, width, height);
        self.begin_edit();
        self.scale_edit(start, target);
        self.commit_edit();
        Ok(())
    }

    /// Wählt alle Objekte auf dem Canvas aus (Strg+A).
    pub fn select_all(&mut self) {
        self.state.selected = (0..self.state.shapes.len()).collect();
    }

    /// Beginnt eine zusammenhängende direkte Manipulation. Beliebig viele
    /// Zwischenstände bilden danach genau einen Undo-Schritt.
    pub fn begin_edit(&mut self) {
        if self.edit_start.is_none() {
            self.edit_start = Some(self.state.clone());
            self.state.push_undo();
        }
    }

    pub fn edit_active(&self) -> bool {
        self.edit_start.is_some()
    }

    pub fn translate_edit(&mut self, dx: f64, dy: f64) {
        debug_assert!(self.edit_active(), "translate_edit ohne begin_edit");
        self.state.translate_selected(dx, dy);
    }

    pub fn scale_edit(&mut self, start: studio_core::BBox, target: studio_core::BBox) {
        debug_assert!(self.edit_active(), "scale_edit ohne begin_edit");
        self.state.scale_selection_to(start, target);
    }

    pub fn rotate_edit_around(&mut self, pivot: [f64; 2], degrees: f64) {
        debug_assert!(self.edit_active(), "rotate_edit_around ohne begin_edit");
        self.state
            .rotate_selection_around((pivot[0], pivot[1]), degrees);
    }

    pub fn commit_edit(&mut self) {
        if self.edit_start.take().is_some() {
            self.state.discard_last_undo_if_no_change();
        }
    }

    pub fn cancel_edit(&mut self) -> bool {
        let Some(start) = self.edit_start.take() else {
            return false;
        };
        self.state = start;
        true
    }
}
