use super::App;

impl App {
    /// Liefert die vollständige oder auf die Auswahl beschränkte Job-Eingabe.
    fn laser_shapes(&self) -> (Vec<luxifer_core::Shape>, Vec<luxifer_core::Layer>) {
        let shapes = if self.laser.selection_only {
            self.session
                .selected
                .iter()
                .filter_map(|&index| self.session.shapes.get(index).cloned())
                .collect()
        } else {
            self.session.shapes.clone()
        };
        (shapes, self.session.layers.clone())
    }

    pub fn laser_select(&mut self, id: &str) {
        self.laser_backend.set_active(id);
    }

    pub fn laser_run(&mut self, action: luxifer_core::JobAction) {
        let (shapes, layers) = self.laser_shapes();
        let start_mode = self.laser.start_mode;
        let anchor = self.laser.anchor;
        match self
            .laser_backend
            .run_action(action, &shapes, &layers, start_mode, anchor)
        {
            Ok(message) => self.toasts.success(message),
            Err(error) => self.app_error = Some(error),
        }
    }

    pub fn laser_export(&mut self) {
        let extension = match self
            .laser_backend
            .active_profile()
            .map(|profile| profile.kind)
        {
            Some(luxifer_core::DriverKind::Ruida) => "rd",
            _ => "gcode",
        };
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(format!("job.{extension}"))
            .save_file()
        else {
            return;
        };

        let (shapes, layers) = self.laser_shapes();
        let start_mode = self.laser.start_mode;
        let anchor = self.laser.anchor;
        match self
            .laser_backend
            .export_to(&path, &shapes, &layers, start_mode, anchor)
        {
            Ok(()) => self
                .toasts
                .success(format!("Exportiert: {}", path.display())),
            Err(error) => self.app_error = Some(error),
        }
    }

    pub fn laser_jog(&mut self, dx: f64, dy: f64) {
        if let Err(error) = self.laser_backend.jog(dx, dy, self.laser.jog_speed) {
            self.app_error = Some(error);
        }
    }

    pub fn laser_home(&mut self) {
        if let Err(error) = self.laser_backend.home(self.laser.jog_speed) {
            self.app_error = Some(error);
        }
    }

    // Die Laser-Profil-Verwaltung (öffnen/speichern/löschen) lebt in der
    // Laser-Sektion des Einstellungen-Dialogs — siehe app/settings.rs.
}
