/*! Reply/forward/outgoing template assembly and attachment payloads. */

use std::fs;
use std::path::Path;

use chrono::Local;

use crate::mail::errors::{MailError, MailResult};
use crate::mail::service::SendOptions;

use crate::mail::model::{MessageDocument, PartBody};

use super::flags::local_message_id;

/// PGP/MIME wrapping is not implemented for local maildir accounts.
pub(super) fn ensure_no_pgp(options: &SendOptions) -> MailResult<()> {
    if options.is_pgp() {
        return Err(MailError::unsupported_feature(
            "PGP/MIME signing and encryption require an IMAP account",
        ));
    }
    Ok(())
}

pub(super) fn attachment_payloads(document: &MessageDocument) -> Vec<(String, Vec<u8>)> {
    let mut payloads = Vec::new();
    let mut index = 0;
    for part in &document.parts {
        part.walk(&mut |part| {
            if let PartBody::Binary(bytes) = &part.body {
                if part.is_attachment() {
                    index += 1;
                    let name = part
                        .filename
                        .clone()
                        .unwrap_or_else(|| format!("attachment-{index}"));
                    payloads.push((name, bytes.clone()));
                }
            }
        });
    }
    payloads
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

pub(super) fn render_outgoing(parsed: &TemplateMessage) -> MailResult<String> {
    for (name, value) in &parsed.headers {
        if crate::mail::draft::has_header_injection(value) {
            return Err(MailError::invalid_input(format!(
                "header {name} contains a line break"
            )));
        }
    }

    let header = |name: &str| {
        parsed
            .headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.clone())
            .unwrap_or_default()
    };
    let from = {
        let value = header("from");
        if value.is_empty() {
            "SolverForge Mail <test@solverforge.local>".to_string()
        } else {
            value
        }
    };
    let to = header("to");
    let cc = header("cc");
    let bcc = header("bcc");
    let subject = header("subject");
    let in_reply_to = header("in-reply-to");
    let references = header("references");
    let date = Local::now().format("%Y-%m-%d %H:%M:%S%:z").to_string();

    let mut raw = String::new();
    raw.push_str(&format!("From: {from}\r\n"));
    if !to.is_empty() {
        raw.push_str(&format!("To: {to}\r\n"));
    }
    if !cc.is_empty() {
        raw.push_str(&format!("Cc: {cc}\r\n"));
    }
    if !bcc.is_empty() {
        raw.push_str(&format!("Bcc: {bcc}\r\n"));
    }
    if !subject.is_empty() {
        raw.push_str(&format!("Subject: {subject}\r\n"));
    }
    if !in_reply_to.is_empty() {
        raw.push_str(&format!("In-Reply-To: {in_reply_to}\r\n"));
    }
    if !references.is_empty() {
        raw.push_str(&format!("References: {references}\r\n"));
    }
    raw.push_str(&format!("Message-ID: {}\r\n", local_message_id()));
    raw.push_str(&format!("Date: {date}\r\n"));

    let attachments = parsed
        .headers
        .iter()
        .filter(|(key, _)| key.eq_ignore_ascii_case("attachment"))
        .map(|(_, value)| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();

    if attachments.is_empty() {
        raw.push_str("\r\n");
        raw.push_str(&crlf(&parsed.body));
        return Ok(raw);
    }

    let boundary = format!(
        "solverforge-{}",
        local_message_id().trim_matches(['<', '>'])
    );
    raw.push_str(&format!(
        "Content-Type: multipart/mixed; boundary=\"{boundary}\"\r\n\r\n"
    ));
    raw.push_str(&format!(
        "--{boundary}\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{}\r\n",
        crlf(&parsed.body)
    ));
    for path in attachments {
        let bytes = fs::read(&path).map_err(|err| {
            MailError::invalid_input(format!("cannot read attachment {path}: {err}"))
        })?;
        let file_name = Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("attachment");
        raw.push_str(&format!(
            "--{boundary}\r\nContent-Type: application/octet-stream; name=\"{file_name}\"\r\nContent-Disposition: attachment; filename=\"{file_name}\"\r\nContent-Transfer-Encoding: base64\r\n\r\n"
        ));
        raw.push_str(&wrap_base64(&bytes));
        raw.push_str("\r\n");
    }
    raw.push_str(&format!("--{boundary}--\r\n"));
    Ok(raw)
}

fn wrap_base64(bytes: &[u8]) -> String {
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    encoded
        .as_bytes()
        .chunks(76)
        .map(|chunk| String::from_utf8_lossy(chunk).to_string())
        .collect::<Vec<_>>()
        .join("\n")
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
            let label = match header {
                "From" => "From",
                "Date" => "Date",
                "Subject" => "Subject",
                "To" => "To",
                "Cc" => "Cc",
                _ => continue,
            };
            lines.push(format!("{label}: {value}"));
        }
    }
    lines.push(String::new());
    lines.push(message.render(78));
    lines.join("\n")
}

#[derive(Debug, Clone)]
pub(super) struct TemplateMessage {
    headers: Vec<(String, String)>,
    body: String,
}

fn crlf(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\n', "\r\n")
}
