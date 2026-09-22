/*! Reply and forward templates for a stored maildir message. */

use crate::mail::errors::MailResult;

use super::flags::{find_message_path, read_parsed_message};
use super::model::MaildirService;
use super::template::{
    forward_subject, forwarded_body, quoted_reply_body, render_template, reply_subject,
};

impl MaildirService {
    /// Build a reply template for a stored message.
    pub(super) fn reply_template(&self, folder: &str, id: &str, all: bool) -> MailResult<String> {
        self.ensure_ready()?;
        let original = read_parsed_message(&find_message_path(&self.folder_path(folder)?, id)?)?;
        let to = original
            .header_value("Reply-To")
            .map(str::to_string)
            .or_else(|| original.header_value("From").map(str::to_string))
            .unwrap_or_default();
        let cc = if all {
            original.header_value("Cc").unwrap_or_default().to_string()
        } else {
            String::new()
        };
        let subject = reply_subject(original.header_value("Subject").map(str::to_string));
        let body = quoted_reply_body(&original);

        let mut headers: Vec<(&str, String)> = vec![("To", to), ("Cc", cc), ("Subject", subject)];
        headers.extend(original.thread.reply_headers());
        Ok(render_template(&headers, &body))
    }

    /// Build a forward template for a stored message.
    pub(super) fn forward_template(&self, folder: &str, id: &str) -> MailResult<String> {
        self.ensure_ready()?;
        let original = read_parsed_message(&find_message_path(&self.folder_path(folder)?, id)?)?;
        let subject = forward_subject(original.header_value("Subject").map(str::to_string));
        let body = forwarded_body(&original);

        Ok(render_template(&[("Subject", subject)], &body))
    }
}
