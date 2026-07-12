use super::EditorSession;

impl EditorSession {
    /// Baut die read-only Laser-Vorschau aus demselben JobPlan wie Export und
    /// Treiber. Keine Session-Mutation, keine zweite Geometrie-Wahrheit.
    pub fn job_preview(&self, selection_only: bool) -> luxifer_core::preview::JobPreview {
        let shapes: Vec<luxifer_core::Shape> = if selection_only {
            self.state
                .selected
                .iter()
                .filter_map(|&i| self.state.shapes.get(i).cloned())
                .collect()
        } else {
            self.state.shapes.clone()
        };
        // Bild-Layer werden mit denselben Asset-Pixeln gerastert wie der echte
        // Job (assets::resolve_luma) — die Vorschau zeigt die verarbeitete
        // Rastertextur, nicht das Design-Original.
        let plan = luxifer_core::JobPlan::from_shapes_with_assets(
            &shapes,
            &self.state.layers,
            crate::assets::resolve_luma,
        );
        luxifer_core::preview::JobPreview::from_plan(&plan)
    }
}
