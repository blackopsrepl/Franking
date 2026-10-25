/*! Reply later and Starred: local views shown in place of a server folder. */

use crate::db::message_markers;
use crate::keys::View;

use super::model::{App, LocalView, UNIFIED_INBOX};

impl App {
    pub(crate) fn open_reply_later(&mut self) {
        self.open_local_view(LocalView::ReplyLater);
    }

    pub(crate) fn open_starred(&mut self) {
        self.open_local_view(LocalView::Starred);
    }

    /// Both views follow the selected account, or every account from All Inboxes.
    fn open_local_view(&mut self, view: LocalView) {
        self.local_view = Some(view);
        if self.current_folder != UNIFIED_INBOX {
            self.current_folder = "INBOX".into();
        }
        self.folder_index = self.folders.len()
            + match view {
                LocalView::ReplyLater => 0,
                LocalView::Starred => 1,
            };
        self.selected.clear();
        self.active_query = None;
        self.page = 1;
        self.envelopes.clear();
        self.envelope_state.select(None);
        self.view = View::EnvelopeList;
        self.load_envelopes();
    }

    /// The account a local view lists, or `None` for every account.
    pub(crate) fn local_view_account(&self) -> Option<&str> {
        (self.current_folder != UNIFIED_INBOX)
            .then_some(self.account_name.as_deref())
            .flatten()
    }

    /// Rows in a local view come from several folders and accounts while batch
    /// selection carries only UIDs, so only the cursor row of the selected
    /// account may be changed.
    pub(crate) fn guard_local_view_mutation(&mut self) -> Option<bool> {
        self.local_view?;
        let allowed = self.selected.is_empty()
            && self.selected_envelope().is_some_and(|e| {
                e.account.is_some() && e.account == self.account_name && e.folder.is_some()
            });
        if !allowed {
            self.set_status(
                "Open the source account and clear batch selection before changing this message.",
            );
        }
        Some(allowed)
    }

    pub(crate) fn load_reply_later(&mut self) {
        let result = self
            .db
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("local database is unavailable"))
            .and_then(|conn| message_markers::reply_later(conn, self.local_view_account()));
        match result {
            Ok(envelopes) => self.handle_envelopes_loaded(envelopes),
            Err(error) => {
                self.loading = false;
                self.set_error(&format!("Could not open Reply later: {error}"));
            }
        }
    }

    pub(crate) fn complete_reply_marker(&mut self) -> anyhow::Result<()> {
        let Some(ref envelope) = self.pending_reply_marker else {
            return Ok(());
        };
        let conn = self
            .db
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("local database is unavailable"))?;
        message_markers::set_reply_later(conn, envelope, false)?;
        self.pending_reply_marker = None;
        Ok(())
    }

    pub(crate) fn toggle_reply_later(&mut self) {
        let Some(envelope) = self.selected_envelope().cloned() else {
            return;
        };
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Local database is unavailable.");
            return;
        };
        let result = message_markers::is_reply_later(conn, &envelope).and_then(|enabled| {
            message_markers::set_reply_later(conn, &envelope, !enabled)?;
            Ok(!enabled)
        });
        match result {
            Ok(enabled) => {
                self.set_status(if enabled {
                    "Added to Reply later."
                } else {
                    "Removed from Reply later."
                });
                if self.local_view == Some(LocalView::ReplyLater) && !enabled {
                    if self.view == View::MessageView {
                        self.view = View::EnvelopeList;
                        self.message_content = None;
                    }
                    self.load_envelopes();
                }
            }
            Err(error) => self.set_error(&format!("Could not update Reply later: {error}")),
        }
    }
}
