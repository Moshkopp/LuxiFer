use luxifer_core::state::AppState;

use super::App;
use crate::ui::PendingProjectAction;

impl App {
    pub fn project_open(&mut self, name: &str) {
        if self.session.is_dirty() {
            self.pending_project = Some(PendingProjectAction::Open(name.to_string()));
        } else {
            self.do_project_open(name);
        }
    }

    pub fn project_new(&mut self, name: &str) {
        if self.session.is_dirty() {
            self.pending_project = Some(PendingProjectAction::New(name.to_string()));
        } else {
            self.do_project_new(name);
        }
    }

    pub fn confirm_pending_project(&mut self) {
        match self.pending_project.take() {
            Some(PendingProjectAction::New(name)) => self.do_project_new(&name),
            Some(PendingProjectAction::Open(name)) => self.do_project_open(&name),
            Some(PendingProjectAction::OpenVersion(id)) => self.do_project_open_version(&id),
            Some(PendingProjectAction::DeleteVersion(id)) => self.do_project_delete_version(&id),
            None => {}
        }
    }

    pub fn request_close(&mut self) -> bool {
        if self.session.is_dirty() {
            self.close_pending = true;
            self.window.request_redraw();
            false
        } else {
            true
        }
    }

    pub fn confirm_close(&mut self) {
        self.close_pending = false;
        self.should_exit = true;
    }

    pub fn should_exit(&self) -> bool {
        self.should_exit
    }

    fn replace_editor_state(&mut self, state: AppState) {
        self.session.replace_state(state);
        self.refresh_accent();
        self.image_dirty = true;
        self.renderer.invalidate_scene();
        self.fit_all();
    }

    fn do_project_open(&mut self, name: &str) {
        match self.project.open(name) {
            Ok(state) => {
                self.replace_editor_state(state);
                self.toasts.success(format!("Geöffnet: {name}"));
                self.view = crate::tools::View::Design;
            }
            Err(error) => self.app_error = Some(error),
        }
    }

    pub fn project_open_version(&mut self, id: &str) {
        if self.session.is_dirty() {
            self.pending_project = Some(PendingProjectAction::OpenVersion(id.to_string()));
        } else {
            self.do_project_open_version(id);
        }
    }

    fn do_project_open_version(&mut self, id: &str) {
        match self.project.open_version(id) {
            Ok(state) => {
                self.replace_editor_state(state);
                self.toasts.success("Version geladen.");
                self.view = crate::tools::View::Design;
            }
            Err(error) => self.app_error = Some(error),
        }
    }

    pub fn project_delete_version(&mut self, id: &str) {
        let deletes_current = self.project.current_version_id() == Some(id);
        if deletes_current && self.session.is_dirty() {
            self.pending_project = Some(PendingProjectAction::DeleteVersion(id.to_string()));
            return;
        }
        self.do_project_delete_version(id);
    }

    fn do_project_delete_version(&mut self, id: &str) {
        match self.project.delete_version(id) {
            Ok(Some(state)) => {
                self.replace_editor_state(state);
                self.toasts
                    .success("Version gelöscht — vorherige Version geladen.");
            }
            Ok(None) => self.toasts.success("Version gelöscht."),
            Err(error) => self.app_error = Some(error),
        }
    }

    pub fn project_rename(&mut self, from: &str, to: &str) {
        match self.project.rename(from, to) {
            Ok(()) => {
                let to = to.trim();
                if self.project_browser.selected.as_deref() == Some(from) {
                    self.project_browser.selected = Some(to.to_string());
                }
                self.toasts.success(format!("Umbenannt: {from} → {to}"));
            }
            Err(error) => self.app_error = Some(error),
        }
    }

    fn do_project_new(&mut self, name: &str) {
        match self.project.new_project(self.session.state(), name) {
            Ok(()) => {
                self.session.mark_saved();
                self.toasts
                    .success(format!("Neues Projekt: {}", name.trim()));
                self.view = crate::tools::View::Design;
            }
            Err(error) => self.app_error = Some(error),
        }
    }

    pub fn project_delete(&mut self, name: &str) {
        match self.project.delete(name) {
            Ok(()) => self.toasts.success(format!("Gelöscht: {name}")),
            Err(error) => self.app_error = Some(error),
        }
    }

    pub fn project_export(&mut self, name: &str) {
        let Some(target) = rfd::FileDialog::new()
            .add_filter("LuxiFer-Projekt", &["luxi"])
            .set_file_name(format!("{name}.luxi"))
            .save_file()
        else {
            return;
        };
        match self.project.export(name, &target) {
            Ok(()) => self
                .toasts
                .success(format!("Exportiert: {}", target.display())),
            Err(error) => self.app_error = Some(error),
        }
    }

    pub fn project_save(&mut self) {
        match self.project.save(self.session.state()) {
            Ok(version) => {
                self.session.mark_saved();
                self.toasts
                    .success(format!("Gespeichert ({})", version.label));
            }
            Err(error) => self.app_error = Some(error),
        }
    }

    pub fn project_save_version(&mut self) {
        match self.project.save_version(self.session.state()) {
            Ok(version) => {
                self.session.mark_saved();
                self.toasts
                    .success(format!("Neue Version {}", version.label));
            }
            Err(error) => self.app_error = Some(error),
        }
    }
}
