/*! Attachment path picker commands. */

use crate::file_picker::{FilePickerState, PickerPurpose};
use crate::keys::View;
use crate::mail::attachments;

use super::model::App;

impl App {
    /// Open the picker at the home directory.
    pub(crate) fn open_file_picker(&mut self) {
        self.open_file_picker_at(None);
    }

    /// Open the picker to choose a directory for saving an attachment.
    pub(crate) fn open_save_attachment_picker(&mut self) {
        self.file_picker = Some(FilePickerState::open_with(
            PickerPurpose::SaveAttachment,
            Some(crate::mail::attachments::downloads_dir()),
        ));
        self.view = View::FilePicker;
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
        if picker.purpose == PickerPurpose::SaveAttachment {
            self.save_attachment_into_selected();
            return;
        }
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

impl App {
    /// Save the selected attachment into the highlighted picker directory.
    fn save_attachment_into_selected(&mut self) {
        let Some(picker) = self.file_picker.as_ref() else {
            return;
        };
        // With nothing highlighted, save into the directory being browsed.
        let target = picker
            .selected()
            .map(|path| path.to_path_buf())
            .unwrap_or_else(|| picker.dir.clone());
        let directory = if target.is_dir() {
            target
        } else {
            target
                .parent()
                .map(|parent| parent.to_path_buf())
                .unwrap_or(target)
        };

        let payloads = self
            .message_content
            .as_ref()
            .map(attachments::payloads)
            .unwrap_or_default();
        let Some((name, bytes)) = payloads.into_iter().nth(self.attachment_index) else {
            self.set_status("No attachment is selected.");
            return;
        };
        match attachments::save_attachments(vec![(name, bytes)], &directory) {
            Ok(saved) => {
                self.file_picker = None;
                self.view = View::AttachmentList;
                self.set_status(&format!("Saved {saved}"));
            }
            Err(error) => self.set_error(&format!("Could not save attachment: {error}")),
        }
    }
}
