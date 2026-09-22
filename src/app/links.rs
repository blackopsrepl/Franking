/*! Link list overlay for the loaded message. */

use crate::keys::View;

use super::model::{App, PendingOpenCommand};

impl App {
    /// Open the link list for the loaded message.
    pub(crate) fn open_links(&mut self) {
        let count = self
            .message_content
            .as_ref()
            .map(|message| message.body.links.len())
            .unwrap_or(0);
        if count == 0 {
            self.set_status("This message has no links.");
            return;
        }
        self.link_index = 0;
        self.view = View::LinkList;
    }

    pub(crate) fn link_next(&mut self) {
        let count = self.link_count();
        if count > 0 {
            self.link_index = (self.link_index + 1).min(count - 1);
        }
    }

    pub(crate) fn link_prev(&mut self) {
        self.link_index = self.link_index.saturating_sub(1);
    }

    pub(crate) fn close_links(&mut self) {
        self.view = View::MessageView;
    }

    /// Open the selected link with the desktop handler.
    pub(crate) fn open_selected_link(&mut self) {
        let href = self
            .message_content
            .as_ref()
            .and_then(|message| message.body.links.get(self.link_index))
            .map(|link| link.href.clone());
        let Some(href) = href else {
            self.set_status("No link is selected.");
            return;
        };
        let program =
            std::env::var("SOLVERFORGE_OPENER").unwrap_or_else(|_| "xdg-open".to_string());
        self.pending_open_command = Some(PendingOpenCommand {
            program,
            args: vec![href],
        });
    }

    fn link_count(&self) -> usize {
        self.message_content
            .as_ref()
            .map(|message| message.body.links.len())
            .unwrap_or(0)
    }
}
