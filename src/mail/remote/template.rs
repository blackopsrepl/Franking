/*! Template parsing and message construction helpers. */

use lettre::message::Mailbox;
use mail_parser::{MessageParser, MimeHeaders};

use crate::mail::errors::{MailError, MailResult};
use crate::mail::model::MessageDocument;

pub(super) struct TemplateMessage {
    pub(super) headers: Vec<(String, String)>,
    pub(super) body: String,
}

impl TemplateMessage {
    pub(super) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
            .filter(|value| !value.trim().is_empty())
    }
}

pub(super) fn extract_attachments(raw: &[u8]) -> MailResult<Vec<(String, Vec<u8>)>> {
    let parser = MessageParser::new()
        .with_minimal_headers()
        .default_header_text();
    let message = parser
        .parse(raw)
        .ok_or_else(|| MailError::other("failed to parse message attachments"))?;

    Ok(message
        .attachments()
        .enumerate()
        .map(|(index, part)| {
            let name = part
                .attachment_name()
                .map(str::to_string)
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| format!("attachment-{}", index + 1));
            (name, part.contents().to_vec())
        })
        .collect())
}

pub(super) fn parse_template_message(raw: &str) -> TemplateMessage {
    let mut headers: Vec<(String, String)> = Vec::new();
    let mut current_key: Option<String> = None;
    let mut body_lines = Vec::new();
    let mut in_body = false;

    for line in raw.lines() {
        if in_body {
            body_lines.push(line);
            continue;
        }

        if line.is_empty() {
            in_body = true;
            continue;
        }

        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some(key) = current_key.as_ref() {
                if let Some((_, value)) = headers
                    .iter_mut()
                    .find(|(name, _)| name.eq_ignore_ascii_case(key))
                {
                    if !value.is_empty() {
                        value.push(' ');
                    }
                    value.push_str(line.trim());
                }
            }
            continue;
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_ascii_lowercase();
            headers.push((key.clone(), value.trim().to_string()));
            current_key = Some(key);
        }
    }

    TemplateMessage {
        headers,
        body: body_lines.join("\n"),
    }
}

pub(super) fn render_template(headers: &[(&str, String)], body: &str) -> String {
    let mut out = String::new();
    for (name, value) in headers {
        if !value.trim().is_empty() {
            out.push_str(&format!("{name}: {}\n", value.trim()));
        }
    }
    out.push('\n');
    out.push_str(body);
    out
}

pub(super) fn reply_subject(subject: Option<String>) -> String {
    let subject = subject.unwrap_or_default();
    if subject.to_ascii_lowercase().starts_with("re:") {
        subject
    } else if subject.is_empty() {
        "Re:".to_string()
    } else {
        format!("Re: {subject}")
    }
}

pub(super) fn forward_subject(subject: Option<String>) -> String {
    let subject = subject.unwrap_or_default();
    if subject.to_ascii_lowercase().starts_with("fwd:") {
        subject
    } else if subject.is_empty() {
        "Fwd:".to_string()
    } else {
        format!("Fwd: {subject}")
    }
}

pub(super) fn quoted_reply_body(message: &MessageDocument) -> String {
    let from = message.header_value("From").unwrap_or_default();
    let date = message.header_value("Date").unwrap_or_default();
    let intro = match (!date.is_empty(), !from.is_empty()) {
        (true, true) => format!("On {date}, {from} wrote:\n"),
        (false, true) => format!("{from} wrote:\n"),
        _ => "Previous message:\n".to_string(),
    };
    let rendered = message.render(78);
    let quoted = rendered
        .lines()
        .map(|line| format!("> {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let quoted = if rendered.is_empty() {
        String::new()
    } else {
        quoted
    };
    format!("\n{intro}{quoted}")
}

pub(super) fn forwarded_body(message: &MessageDocument) -> String {
    let mut lines = vec!["---------- Forwarded message ----------".to_string()];
    for header in ["From", "Date", "Subject", "To", "Cc"] {
        if let Some(value) = message.header_value(header) {
            lines.push(format!("{header}: {value}"));
        }
    }
    lines.push(String::new());
    lines.push(message.render(78));
    lines.join("\n")
}

pub(super) fn parse_mailboxes(value: &str) -> MailResult<Vec<Mailbox>> {
    let raw = format!("To: {value}\r\n\r\n");
    let parser = MessageParser::new()
        .with_minimal_headers()
        .default_header_text();
    let message = parser
        .parse(raw.as_bytes())
        .ok_or_else(|| MailError::invalid_input("failed to parse email address list"))?;
    let addresses = message
        .to()
        .ok_or_else(|| MailError::invalid_input("failed to parse email address list"))?;

    let mut mailboxes = Vec::new();
    for addr in addresses.iter() {
        let address = addr
            .address
            .as_deref()
            .ok_or_else(|| MailError::invalid_input("recipient address is missing"))?;
        let mailbox = match addr.name.as_deref().filter(|value| !value.is_empty()) {
            Some(name) => format!("{name} <{address}>"),
            None => address.to_string(),
        };
        mailboxes.push(
            mailbox
                .parse::<Mailbox>()
                .map_err(|err| MailError::invalid_input(err.to_string()))?,
        );
    }

    if mailboxes.is_empty() {
        Err(MailError::invalid_input(
            "at least one recipient address is required",
        ))
    } else {
        Ok(mailboxes)
    }
}

pub(super) fn parse_mailbox(value: &str) -> MailResult<Mailbox> {
    parse_mailboxes(value)?
        .into_iter()
        .next()
        .ok_or_else(|| MailError::invalid_input("address list was empty"))
}
