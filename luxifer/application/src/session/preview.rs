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
        let dir = luxifer_core::assets_dir();
        let plan =
            luxifer_core::JobPlan::from_shapes_with_assets(&shapes, &self.state.layers, |asset| {
                let (pixels, width, height) =
                    luxifer_core::load_asset_luma(&dir, &asset.to_string()).ok()?;
                Some((
                    std::borrow::Cow::Owned(pixels),
                    width as usize,
                    height as usize,
                ))
            });
        luxifer_core::preview::JobPreview::from_plan(&plan)
    }
}
