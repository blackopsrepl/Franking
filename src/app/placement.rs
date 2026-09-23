/*! Place one message in a lane without changing its sender's future route. */

use crate::db::{message_routes, sender_routes::Route};
use crate::keys::View;

use super::model::App;

impl App {
    pub(crate) fn open_place_prompt(&mut self) {
        if self.selected_envelope().is_none() {
            self.set_status("Select a message to place.");
            return;
        }
        self.view = View::PlacePrompt;
    }

    pub(crate) fn cancel_place(&mut self) {
        self.view = View::EnvelopeList;
    }

    /// Store a per-message placement and refresh the lane so it applies.
    pub(crate) fn place_message(&mut self, route: Route) {
        let Some(envelope) = self.selected_envelope().cloned() else {
            self.cancel_place();
            return;
        };
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Local database is unavailable.");
            return;
        };
        match message_routes::set(conn, &envelope, Some(route)) {
            Ok(()) => {
                self.view = View::EnvelopeList;
                self.set_status(&format!("This message is placed in {route:?}."));
                self.load_envelopes();
            }
            Err(error) => self.set_error(&format!("Could not place the message: {error}")),
        }
    }
}
