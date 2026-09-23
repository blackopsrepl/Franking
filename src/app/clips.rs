/*! Text clips: save an excerpt and copy it back on demand. */

use crate::db::clips::{self as store, Clip, ClipSource};
use crate::keys::View;

use super::model::App;

/// State of the clip library and the capture prompt.
#[derive(Default)]
pub struct ClipsState {
    pub items: Vec<Clip>,
    pub index: usize,
    pub pending_delete: Option<i64>,
    pub input: String,
    pub source: Option<ClipSource>,
}

impl App {
    pub(crate) fn open_clips(&mut self) {
        self.reload_clips();
        self.clips.pending_delete = None;
        self.view = View::Clips;
    }

    fn reload_clips(&mut self) {
        self.clips.items = self
            .db
            .as_ref()
            .and_then(|conn| store::list(conn).ok())
            .unwrap_or_default();
        self.clips.index = self
            .clips
            .index
            .min(self.clips.items.len().saturating_sub(1));
    }

    pub(crate) fn clips_next(&mut self) {
        if !self.clips.items.is_empty() {
            self.clips.index = (self.clips.index + 1).min(self.clips.items.len() - 1);
        }
    }

    pub(crate) fn clips_prev(&mut self) {
        self.clips.index = self.clips.index.saturating_sub(1);
    }

    pub(crate) fn close_clips(&mut self) {
        self.clips.pending_delete = None;
        self.view = View::EnvelopeList;
    }

    /// Open the capture prompt, prefilled with the message's opening line.
    pub(crate) fn open_clip_prompt(&mut self) {
        let Some(envelope) = self.selected_envelope().cloned() else {
            self.set_status("Open a message to clip from it.");
            return;
        };
        let subject = self
            .message_content
            .as_ref()
            .map(|message| message.subject().to_string())
            .unwrap_or_else(|| envelope.subject.clone());
        let label = format!("{subject} \u{00b7} {}", envelope.sender_display());
        self.clips.source = Some(ClipSource {
            account: envelope.account.clone().or_else(|| self.acct_owned()),
            folder: envelope
                .folder
                .clone()
                .or_else(|| Some(self.current_folder.clone())),
            uid: envelope.id.clone(),
            message_id: envelope.message_id.clone(),
            label,
        });
        self.clips.input = self
            .message_content
            .as_ref()
            .map(|message| {
                message
                    .body
                    .render(10_000)
                    .lines()
                    .map(str::trim)
                    .find(|line| !line.is_empty())
                    .unwrap_or_default()
                    .to_string()
            })
            .unwrap_or_default();
        self.view = View::ClipPrompt;
    }

    pub(crate) fn clip_input(&mut self, c: char) {
        self.clips.input.push(c);
    }

    pub(crate) fn clip_backspace(&mut self) {
        self.clips.input.pop();
    }

    pub(crate) fn cancel_clip(&mut self) {
        self.clips.input.clear();
        self.clips.source = None;
        self.view = View::MessageView;
    }

    pub(crate) fn submit_clip(&mut self) {
        let Some(source) = self.clips.source.clone() else {
            self.cancel_clip();
            return;
        };
        let body = self.clips.input.clone();
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Local database is unavailable.");
            return;
        };
        match store::add(conn, &source, &body) {
            Ok(_) => {
                self.clips.input.clear();
                self.clips.source = None;
                self.reload_clips();
                self.view = View::Clips;
                self.set_status("Clipped.");
            }
            Err(error) => self.set_error(&format!("Could not save the clip: {error}")),
        }
    }

    /// Copy the highlighted clip to the system clipboard (best effort).
    pub(crate) fn copy_clip(&mut self) {
        let Some(clip) = self.clips.items.get(self.clips.index) else {
            return;
        };
        match copy_to_clipboard(&clip.body) {
            Ok(()) => self.set_status("Clip copied to the clipboard."),
            Err(error) => self.set_error(&format!(
                "Could not copy the clip (install wl-clipboard): {error}"
            )),
        }
    }

    pub(crate) fn delete_clip(&mut self) {
        let Some(clip) = self.clips.items.get(self.clips.index) else {
            return;
        };
        let id = clip.id;
        if self.clips.pending_delete != Some(id) {
            self.clips.pending_delete = Some(id);
            self.set_status("Press d again to delete this clip.");
            return;
        }
        self.clips.pending_delete = None;
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Local database is unavailable.");
            return;
        };
        match store::delete(conn, id) {
            Ok(_) => {
                self.reload_clips();
                self.set_status("Clip deleted.");
            }
            Err(error) => self.set_error(&format!("Could not delete the clip: {error}")),
        }
    }
}

/// Hand text to the Wayland clipboard through `wl-copy`.
fn copy_to_clipboard(text: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new("wl-copy").stdin(Stdio::piped()).spawn()?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(text.as_bytes())?;
    }
    let _ = child.wait();
    Ok(())
}
