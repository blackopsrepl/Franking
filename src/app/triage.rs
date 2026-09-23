/*! Local sender routing controls over incoming mail. */

use crate::db::sender_routes::{self, Route};
use crate::keys::View;

use super::model::{App, UNIFIED_INBOX};

impl App {
    pub(crate) fn cycle_triage_lane(&mut self) {
        if !self.current_folder.eq_ignore_ascii_case("INBOX")
            && self.current_folder != UNIFIED_INBOX
        {
            self.set_status("Select an inbox to use triage.");
            return;
        }
        self.triage_lane = match self.triage_lane {
            None => Some(Route::Screening),
            Some(Route::Screening) => Some(Route::Inbox),
            Some(Route::Inbox) => Some(Route::Reading),
            Some(Route::Reading) => Some(Route::Receipts),
            Some(Route::Receipts) => Some(Route::Blocked),
            Some(Route::Blocked) => None,
        };
        self.page = 1;
        self.envelopes.clear();
        self.envelope_state.select(None);
        self.load_envelopes();
    }

    pub(crate) fn route_selected_sender(&mut self, route: Route) {
        let Some(envelope) = self.selected_envelope() else {
            return;
        };
        let Some(account) = envelope.account.as_deref() else {
            self.set_error("Cannot identify this message's receiving account.");
            return;
        };
        let Some(sender) = sender_routes::sender_address(&envelope.sender) else {
            self.set_error("Cannot identify this sender's mailbox.");
            return;
        };
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Triage needs the local database.");
            return;
        };
        if let Err(error) = sender_routes::set(conn, account, &sender, route) {
            self.set_error(&format!("Could not route sender: {error}"));
            return;
        }
        self.set_status(&format!("{sender} routed for {account}: {route:?}"));
        if self.view == View::MessageView {
            self.view = View::EnvelopeList;
            self.message_content = None;
        }
        self.load_envelopes();
    }
}
