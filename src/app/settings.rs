/*! Preferences overlay. */

use crate::db::preferences;
use crate::keys::View;

use super::model::App;

/// Preference key for desktop notifications.
const NOTIFICATIONS: &str = "notifications";
/// Preference key for marking messages read when they are opened.
const MARK_READ_ON_OPEN: &str = "mark_read_on_open";
/// Preference keys for the page size and autosave interval.
const PAGE_SIZE: &str = "page_size";
const AUTOSAVE_SECONDS: &str = "autosave_seconds";

/// Page sizes offered in the preferences overlay.
const PAGE_SIZES: [usize; 4] = [25, 50, 100, 200];
/// Autosave intervals offered in the preferences overlay (0 disables).
const AUTOSAVE_CHOICES: [u64; 4] = [15, 30, 60, 0];

/// Number of rows in the preferences overlay.
pub(crate) const SETTINGS_ROWS: usize = 4;

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
            self.page_size = stored_number(conn, PAGE_SIZE)
                .unwrap_or(self.page_size)
                .max(1);
            self.autosave_seconds = stored_number(conn, AUTOSAVE_SECONDS)
                .map(|value| value as u64)
                .unwrap_or(self.autosave_seconds);
        }
    }

    pub(crate) fn open_settings(&mut self) {
        self.settings_index = 0;
        self.view = View::Settings;
    }

    /// Move the settings highlight.
    pub(crate) fn settings_move(&mut self, delta: i32) {
        let count = SETTINGS_ROWS as i32;
        self.settings_index = ((self.settings_index as i32 + delta).rem_euclid(count)) as usize;
    }

    /// Change the highlighted preference.
    pub(crate) fn settings_toggle(&mut self) {
        match self.settings_index {
            0 => self.toggle_notifications(),
            1 => self.toggle_mark_read_on_open(),
            2 => {
                let index = PAGE_SIZES
                    .iter()
                    .position(|size| *size == self.page_size)
                    .map(|index| (index + 1) % PAGE_SIZES.len())
                    .unwrap_or(0);
                self.page_size = PAGE_SIZES[index];
                self.persist_number(PAGE_SIZE, self.page_size);
                self.set_status(&format!("Page size: {}.", self.page_size));
            }
            _ => {
                let index = AUTOSAVE_CHOICES
                    .iter()
                    .position(|seconds| *seconds == self.autosave_seconds)
                    .map(|index| (index + 1) % AUTOSAVE_CHOICES.len())
                    .unwrap_or(0);
                self.autosave_seconds = AUTOSAVE_CHOICES[index];
                self.persist_number(AUTOSAVE_SECONDS, self.autosave_seconds as usize);
                let message = if self.autosave_seconds == 0 {
                    "Compose autosave off.".to_string()
                } else {
                    format!("Compose autosave every {}s.", self.autosave_seconds)
                };
                self.set_status(&message);
            }
        }
    }

    /// Store a numeric preference.
    fn persist_number(&mut self, key: &str, value: usize) {
        if let Some(conn) = self.db.as_ref() {
            let _ = preferences::set_number(conn, key, value);
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

/// Read a numeric preference, if one was stored.
fn stored_number(conn: &rusqlite::Connection, key: &str) -> Option<usize> {
    preferences::get_number(conn, key).ok().flatten()
}
