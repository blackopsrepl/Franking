/*! Preferences overlay. */

use crate::db::preferences;
use crate::keys::View;

use super::model::App;

/// Preference key for the desktop-notification rule.
const NOTIFICATION_RULE: &str = "notification_rule";
/// Preference key for encrypting stored drafts.
const ENCRYPT_DRAFTS: &str = "encrypt_drafts";
/// Preference key for marking messages read when they are opened.
const MARK_READ_ON_OPEN: &str = "mark_read_on_open";
/// Preference keys for the page size and autosave interval.
const PAGE_SIZE: &str = "page_size";
const AUTOSAVE_SECONDS: &str = "autosave_seconds";
/// Preference key for covering previously seen mail in the Inbox lane.
const COVER_SEEN: &str = "cover_seen";

/// Page sizes offered in the preferences overlay.
const PAGE_SIZES: [usize; 4] = [25, 50, 100, 200];
/// Autosave intervals offered in the preferences overlay (0 disables).
const AUTOSAVE_CHOICES: [u64; 4] = [15, 30, 60, 0];

/// Number of rows in the preferences overlay.
pub(crate) const SETTINGS_ROWS: usize = 7;

impl App {
    /// Load persisted preferences (called during startup).
    pub(crate) fn load_preferences(&mut self) {
        if let Some(conn) = self.db.as_ref() {
            if let Ok(Some(rule)) = preferences::get_text(conn, NOTIFICATION_RULE) {
                self.notification_rule = super::notification_rules::NotificationRule::parse(&rule);
            } else if let Ok(false) = preferences::get(conn, "notifications", true) {
                // Older installs stored a boolean; off means off.
                self.notification_rule = super::notification_rules::NotificationRule::Off;
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
            if let Ok(enabled) = preferences::get(conn, ENCRYPT_DRAFTS, false) {
                self.encrypt_drafts = enabled;
            }
            if let Ok(enabled) = preferences::get(conn, COVER_SEEN, false) {
                self.cover_seen = enabled;
            }
        }
        self.refresh_bypass_token();
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
            0 => self.cycle_notification_rule(),
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
            3 => self.toggle_encrypt_drafts(),
            4 => {
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
            5 => self.toggle_cover_seen(),
            _ => self.open_bypass_prompt(),
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
}

impl App {
    /// Toggle covering previously seen mail and remember the choice.
    pub(crate) fn toggle_cover_seen(&mut self) {
        self.cover_seen = !self.cover_seen;
        self.cover_revealed = false;
        if let Some(conn) = self.db.as_ref() {
            if let Err(error) = preferences::set(conn, COVER_SEEN, self.cover_seen) {
                self.set_error(&format!("Could not save the preference: {error}"));
                return;
            }
        }
        self.set_status(if self.cover_seen {
            "Previously seen mail is covered; press V to lift it."
        } else {
            "Previously seen mail is shown."
        });
    }
}

impl App {
    /// Toggle encrypting stored drafts and remember the choice.
    pub(crate) fn toggle_encrypt_drafts(&mut self) {
        self.encrypt_drafts = !self.encrypt_drafts;
        if let Some(conn) = self.db.as_ref() {
            if let Err(error) = preferences::set(conn, ENCRYPT_DRAFTS, self.encrypt_drafts) {
                self.set_error(&format!("Could not save the preference: {error}"));
                return;
            }
        }
        self.set_status(if self.encrypt_drafts {
            "Drafts are encrypted to you when stored."
        } else {
            "Drafts are stored unencrypted."
        });
    }

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
