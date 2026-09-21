/*! Reply/forward/outgoing template assembly and attachment payloads. */

use chrono::Local;

use crate::mail::model::{MessageDocument, PartBody};

use super::flags::local_message_id;

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

pub(super) fn render_outgoing(parsed: &TemplateMessage) -> String {
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
    raw.push_str(&format!("From: {from}\n"));
    if !to.is_empty() {
        raw.push_str(&format!("To: {to}\n"));
    }
    if !cc.is_empty() {
        raw.push_str(&format!("Cc: {cc}\n"));
    }
    if !bcc.is_empty() {
        raw.push_str(&format!("Bcc: {bcc}\n"));
    }
    if !subject.is_empty() {
        raw.push_str(&format!("Subject: {subject}\n"));
    }
    if !in_reply_to.is_empty() {
        raw.push_str(&format!("In-Reply-To: {in_reply_to}\n"));
    }
    if !references.is_empty() {
        raw.push_str(&format!("References: {references}\n"));
    }
    raw.push_str(&format!("Message-ID: {}\n", local_message_id()));
    raw.push_str(&format!("Date: {date}\n\n"));
    raw.push_str(&parsed.body);
    raw
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
