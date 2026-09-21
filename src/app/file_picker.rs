/*! Attachment path picker commands. */

use crate::file_picker::FilePickerState;
use crate::keys::View;

use super::model::App;

impl App {
    /// Open the picker at the home directory.
    pub(crate) fn open_file_picker(&mut self) {
        self.open_file_picker_at(None);
    }

    /// Open the picker, optionally starting from `hint` (a typed path).
    pub(crate) fn open_file_picker_at(&mut self, hint: Option<String>) {
        let start = hint
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .map(std::path::PathBuf::from)
            .map(|path| {
                if path.is_dir() {
                    path
                } else {
                    path.parent()
                        .map(|parent| parent.to_path_buf())
                        .unwrap_or(path)
                }
            });
        self.file_picker = Some(FilePickerState::open(start));
        self.view = View::FilePicker;
    }

    pub(crate) fn close_file_picker(&mut self) {
        self.file_picker = None;
        self.view = View::Compose;
    }

    pub(crate) fn file_picker_next(&mut self) {
        if let Some(picker) = self.file_picker.as_mut() {
            picker.next();
        }
    }

    pub(crate) fn file_picker_prev(&mut self) {
        if let Some(picker) = self.file_picker.as_mut() {
            picker.prev();
        }
    }

    pub(crate) fn file_picker_up(&mut self) {
        if let Some(picker) = self.file_picker.as_mut() {
            picker.parent_directory();
        }
    }

    /// Select a file (attaching it) or descend into a directory.
    pub(crate) fn file_picker_enter(&mut self) {
        let Some(picker) = self.file_picker.as_mut() else {
            return;
        };
        let Some(path) = picker.activate() else {
            return;
        };
        let display = path.display().to_string();
        if let Some(cs) = self.compose_state.as_mut() {
            cs.attachments.push(display.clone());
            cs.dirty = true;
            cs.attach_input = None;
        }
        self.file_picker = None;
        self.view = View::Compose;
        self.set_status(&format!("Attached {display}."));
    }
}
