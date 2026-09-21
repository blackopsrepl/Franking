/*! Preferences overlay. */

use crate::db::preferences;
use crate::keys::View;

use super::model::App;

/// Preference key for desktop notifications.
const NOTIFICATIONS: &str = "notifications";

impl App {
    /// Load persisted preferences (called during startup).
    pub(crate) fn load_preferences(&mut self) {
        if let Some(conn) = self.db.as_ref() {
            if let Ok(enabled) = preferences::get(conn, NOTIFICATIONS, true) {
                self.notifications_enabled = enabled;
            }
        }
    }

    pub(crate) fn open_settings(&mut self) {
        self.view = View::Settings;
    }

    pub(crate) fn close_settings(&mut self) {
        self.view = View::EnvelopeList;
    }

    /// Toggle desktop notifications and remember the choice.
    pub(crate) fn toggle_notifications(&mut self) {
        self.notifications_enabled = !self.notifications_enabled;
        if let Some(conn) = self.db.as_ref() {
            if let Err(error) = preferences::set(conn, NOTIFICATIONS, self.notifications_enabled) {
                self.set_error(&format!("Could not save the preference: {error}"));
                return;
            }
        }
        self.set_status(if self.notifications_enabled {
            "Desktop notifications on."
        } else {
            "Desktop notifications off."
        });
    }
}
