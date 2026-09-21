/*! In-message text search.
Matches are line offsets inside the rendered message (headers, separator,
then body), so a jump is a scroll assignment. */

use crate::keys::View;

use super::model::App;

/// Non-overlapping occurrences of `query` (already lowercase) in `line`.
fn count_occurrences(line: &str, query: &str) -> usize {
    line.to_ascii_lowercase().match_indices(query).count()
}

impl App {
    /// Open the in-message search prompt.
    pub(crate) fn enter_message_search(&mut self) {
        self.message_search.clear();
        self.message_search_active = true;
        self.view = View::MessageSearch;
    }

    pub(crate) fn message_search_input(&mut self, c: char) {
        self.message_search.push(c);
    }

    pub(crate) fn message_search_backspace(&mut self) {
        self.message_search.pop();
    }

    pub(crate) fn cancel_message_search(&mut self) {
        self.message_search_active = false;
        self.view = View::MessageView;
    }

    /// Run the search and jump to the first match.
    pub(crate) fn submit_message_search(&mut self) {
        self.message_search_active = false;
        self.view = View::MessageView;
        self.message_matches = self.find_message_matches();
        self.message_match_index = 0;
        if self.message_matches.is_empty() {
            self.set_status(&format!("No matches for \"{}\".", self.message_search));
            return;
        }
        self.jump_to_match();
    }

    /// Step one match forward (`next`) or backward, wrapping around.
    pub(crate) fn step_match(&mut self, next: bool) {
        if next {
            self.next_match();
        } else {
            self.prev_match();
        }
    }

    /// Advance to the next match, wrapping around.
    pub(crate) fn next_match(&mut self) {
        if self.message_matches.is_empty() {
            self.set_status("No active search (press /).");
            return;
        }
        self.message_match_index = (self.message_match_index + 1) % self.message_matches.len();
        self.jump_to_match();
    }

    /// Move to the previous match, wrapping around.
    pub(crate) fn prev_match(&mut self) {
        if self.message_matches.is_empty() {
            self.set_status("No active search (press /).");
            return;
        }
        let count = self.message_matches.len();
        self.message_match_index = (self.message_match_index + count - 1) % count;
        self.jump_to_match();
    }

    fn jump_to_match(&mut self) {
        let Some(&line) = self.message_matches.get(self.message_match_index) else {
            return;
        };
        self.message_scroll = line.saturating_sub(2);
        self.set_status(&format!(
            "Match {}/{} for \"{}\".",
            self.message_match_index + 1,
            self.message_matches.len(),
            self.message_search
        ));
    }

    /// Line offsets of every case-insensitive match in the rendered message.
    pub(crate) fn find_message_matches(&self) -> Vec<u16> {
        let query = self.message_search.trim().to_ascii_lowercase();
        if query.is_empty() {
            return Vec::new();
        }
        let width = self.message_body_width();
        let header_lines = self.message_header_line_count(width);
        let mut matches = Vec::new();

        for (index, line) in self.all_header_lines().iter().enumerate() {
            for _ in 0..count_occurrences(line, &query) {
                matches.push(index as u16);
            }
        }
        let body = self.render_message_body(width);
        for (index, line) in body.lines().enumerate() {
            for _ in 0..count_occurrences(line, &query) {
                matches.push(header_lines + index as u16);
            }
        }
        matches
    }

    /// Text lines rendered above the body (headers, separator, and extras).
    fn all_header_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if let Some(message) = self.current_message() {
            for field in message.header_fields() {
                lines.push(format!("{}: {}", field.name, field.value));
            }
            if let Some(summary) = message.authentication().summary() {
                lines.push(summary);
            }
            if let Some(protection) = message.protection() {
                lines.push(protection.label().to_string());
            }
            if let Some(event) = message.invitation() {
                if let Some(summary) = event.summary_line() {
                    lines.push(format!("Invitation: {summary}"));
                }
            }
        }
        lines.push(String::new());
        lines.push("\u{2500}".repeat(self.message_body_width().saturating_sub(1)));
        lines
    }

    /// Width used to render the message body.
    pub(crate) fn message_body_width(&self) -> usize {
        self.last_terminal_width.saturating_sub(4) as usize
    }

    /// Header lines above the body, including separator and blank line.
    pub(crate) fn message_header_line_count(&self, width: usize) -> u16 {
        let header = self
            .current_message()
            .map(|message| {
                let attachment_lines = if message.attachments.is_empty() {
                    0
                } else {
                    message.attachments.len() as u16 + 2
                };
                message.header_fields().len() as u16 + 3 + attachment_lines
            })
            .unwrap_or(0);
        let _ = width;
        header
    }
}
