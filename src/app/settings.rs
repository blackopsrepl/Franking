/*! Preferences overlay. */

use crate::db::preferences;
use crate::keys::View;

use super::model::App;

/// Preference key for desktop notifications.
const NOTIFICATIONS: &str = "notifications";
/// Preference key for marking messages read when they are opened.
const MARK_READ_ON_OPEN: &str = "mark_read_on_open";

impl App {
    /// Load persisted preferences (called during startup).
    pub(crate) fn load_preferences(&mut self) {
        if let Some(conn) = self.db.as_ref() {
            if let Ok(enabled) = preferences::get(conn, NOTIFICATIONS, true) {
                self.notifications_enabled = enabled;
            }
            if let Ok(enabled) = preferences::get(conn, MARK_READ_ON_OPEN, true) {
                self.mark_read_on_open = enabled;
            }
        }
    }

    pub(crate) fn open_settings(&mut self) {
        self.settings_index = 0;
        self.view = View::Settings;
    }

    /// Move the settings highlight.
    pub(crate) fn settings_move(&mut self, delta: i32) {
        let count = 2i32;
        self.settings_index = ((self.settings_index as i32 + delta).rem_euclid(count)) as usize;
    }

    /// Toggle the highlighted preference.
    pub(crate) fn settings_toggle(&mut self) {
        if self.settings_index == 0 {
            self.toggle_notifications();
        } else {
            self.toggle_mark_read_on_open();
        }
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

impl App {
    /// Toggle "mark read on open" and remember the choice.
    pub(crate) fn toggle_mark_read_on_open(&mut self) {
        self.mark_read_on_open = !self.mark_read_on_open;
        if let Some(conn) = self.db.as_ref() {
            if let Err(error) = preferences::set(conn, MARK_READ_ON_OPEN, self.mark_read_on_open) {
                self.set_error(&format!("Could not save the preference: {error}"));
                return;
            }
        }
        self.set_status(if self.mark_read_on_open {
            "Messages are marked read when opened."
        } else {
            "Messages stay unread until you mark them."
        });
    }
}
