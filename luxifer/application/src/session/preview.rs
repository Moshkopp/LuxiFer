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
        // Rasterbilder werden im ersten nativen Preview-Schnitt als bereits
        // gecachte GPU-Textur dargestellt. Die teure Job-Rasterung folgt erst
        // mit der dedizierten Raster-Preview-Pipeline.
        let plan = luxifer_core::JobPlan::from_shapes(&shapes, &self.state.layers);
        luxifer_core::preview::JobPreview::from_plan(&plan)
    }
}
