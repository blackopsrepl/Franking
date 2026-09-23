/*! Reply and reference queues backed by local message metadata. */

use crate::db::message_markers::{self, Marker};
use crate::keys::View;

use super::model::{App, UNIFIED_INBOX};

impl App {
    pub(crate) fn complete_reply_marker(&mut self) -> anyhow::Result<()> {
        let Some(ref envelope) = self.pending_reply_marker else {
            return Ok(());
        };
        let conn = self
            .db
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("local database is unavailable"))?;
        message_markers::set(conn, envelope, Marker::ReplyLater, false)?;
        self.pending_reply_marker = None;
        Ok(())
    }

    pub(crate) fn open_followup(&mut self, marker: Marker) {
        self.followup_lane = if self.followup_lane == Some(marker) {
            None
        } else {
            Some(marker)
        };
        self.triage_lane = None;
        if self.current_folder != UNIFIED_INBOX {
            self.current_folder = "INBOX".into();
        }
        self.active_query = None;
        self.page = 1;
        self.envelopes.clear();
        self.envelope_state.select(None);
        self.view = View::EnvelopeList;
        self.load_envelopes();
    }

    pub(crate) fn toggle_marker(&mut self, marker: Marker) {
        let Some(envelope) = self.selected_envelope().cloned() else {
            return;
        };
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Local database is unavailable.");
            return;
        };
        let result = message_markers::has(conn, &envelope, marker).and_then(|enabled| {
            message_markers::set(conn, &envelope, marker, !enabled)?;
            Ok(!enabled)
        });
        match result {
            Ok(enabled) => {
                self.set_status(&format!(
                    "{} {}.",
                    marker.label(),
                    if enabled { "added" } else { "removed" }
                ));
                if self.followup_lane == Some(marker) && !enabled {
                    if self.view == View::MessageView {
                        self.view = View::EnvelopeList;
                        self.message_content = None;
                    }
                    self.load_envelopes();
                }
            }
            Err(error) => self.set_error(&format!("Could not update {}: {error}", marker.label())),
        }
    }
}
