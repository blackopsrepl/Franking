/*! Screening bypass token and the previously-seen cover. */

use crate::db::account_policy;
use crate::keys::View;

use super::model::App;

impl App {
    /// Reload the current account's bypass token from the database.
    pub(crate) fn refresh_bypass_token(&mut self) {
        self.bypass_token = self
            .db
            .as_ref()
            .zip(self.account_name.as_deref())
            .and_then(|(conn, account)| account_policy::bypass_token(conn, account).ok().flatten());
    }

    pub(crate) fn open_bypass_prompt(&mut self) {
        self.bypass_input = self.bypass_token.clone().unwrap_or_default();
        self.view = View::BypassPrompt;
    }

    pub(crate) fn cancel_bypass(&mut self) {
        self.bypass_input.clear();
        self.view = View::Settings;
    }

    pub(crate) fn bypass_input(&mut self, c: char) {
        self.bypass_input.push(c);
    }

    pub(crate) fn bypass_backspace(&mut self) {
        self.bypass_input.pop();
    }

    /// Fill the prompt with a fresh random token.
    pub(crate) fn bypass_generate(&mut self) {
        self.bypass_input = account_policy::generate_token();
    }

    /// Store the typed token, or clear it when the field is blank.
    pub(crate) fn submit_bypass(&mut self) {
        let Some(account) = self.acct_owned() else {
            self.set_error("Select an account first.");
            return;
        };
        let token = self.bypass_input.trim().to_string();
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Local database is unavailable.");
            return;
        };
        let value = (!token.is_empty()).then_some(token.as_str());
        match account_policy::set_bypass_token(conn, &account, value) {
            Ok(()) => {
                self.bypass_input.clear();
                self.refresh_bypass_token();
                self.view = View::Settings;
                self.set_status(match value {
                    Some(_) => "Bypass token set.",
                    None => "Bypass token cleared.",
                });
            }
            Err(error) => self.set_error(&format!("Could not save the token: {error}")),
        }
    }

    /// Lift or replace the cover over previously seen mail.
    pub(crate) fn toggle_cover_reveal(&mut self) {
        self.cover_revealed = !self.cover_revealed;
        if self.cover_revealed {
            self.set_status("Cover lifted for this session.");
        } else {
            self.set_status("Previously seen mail is covered.");
        }
        self.load_envelopes();
    }
}
