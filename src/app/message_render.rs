/*! Message-view accessors: headers, file stem, and canonical body rendering. */

use crate::mail::MessageDocument;

use super::model::App;

impl App {
    pub fn all_headers(&self) -> Vec<String> {
        let Some(raw) = self
            .message_content
            .as_ref()
            .and_then(|message| message.raw.as_deref())
        else {
            return Vec::new();
        };
        let text = String::from_utf8_lossy(raw);
        let head = text
            .split("\r\n\r\n")
            .next()
            .and_then(|head| head.split("\n\n").next())
            .unwrap_or_default();
        head.lines()
            .map(|line| line.trim_end_matches('\r').to_string())
            .collect()
    }

    /// Base file name for saving the loaded message as an .eml file.
    pub fn message_file_stem(&self) -> String {
        let message_id = self.message_content.as_ref().and_then(|message| {
            message
                .header_fields()
                .iter()
                .find(|field| field.name.eq_ignore_ascii_case("message-id"))
                .map(|field| {
                    field
                        .value
                        .trim()
                        .trim_start_matches('<')
                        .trim_end_matches('>')
                        .to_string()
                })
                .filter(|value| !value.is_empty())
        });
        let stem = message_id
            .or_else(|| self.selected_envelope_id().map(str::to_string))
            .unwrap_or_else(|| "message".to_string());
        let sanitized = crate::mail::attachments::safe_file_name(&stem);
        if sanitized.is_empty() {
            "message".to_string()
        } else {
            sanitized
        }
    }

    pub fn current_message(&self) -> Option<&MessageDocument> {
        self.message_content.as_ref()
    }

    pub fn render_message_body(&self, width: usize) -> String {
        let body = self
            .current_message()
            .map(|message| message.render(width))
            .unwrap_or_default();
        match self
            .message_note
            .as_deref()
            .filter(|note| !note.trim().is_empty())
        {
            Some(note) => format!("Note: {note}\n\n{body}"),
            None => body,
        }
    }

    pub fn rendered_message_line_count(&self, width: usize) -> u16 {
        let header_lines = self
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
        let body_lines = self.render_message_body(width).lines().count() as u16;
        header_lines.saturating_add(body_lines)
    }
}
