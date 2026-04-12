use std::borrow::Cow;

use mail_parser::{MessageParser, MimeHeaders};

use super::errors::MailResult;
use super::message::{
    normalize_newlines, MessageAttachment, MessageContent, MessageDisplayMode, MessageHeader,
};

pub fn parse_message(raw: impl AsRef<[u8]>) -> MailResult<MessageContent> {
    let raw = raw.as_ref();
    let parser = MessageParser::new()
        .with_minimal_headers()
        .default_header_text();

    if let Some(message) = parser.parse(raw) {
        let headers = message
            .headers_raw()
            .map(|(name, value)| MessageHeader {
                name: name.to_string(),
                value: normalize_newlines(value).trim().to_string(),
            })
            .collect::<Vec<_>>();
        let plain_body = message.body_text(0).map(cow_to_normalized_string);
        let html_body = message.body_html(0).map(cow_to_normalized_string);
        let attachments = (0..message.attachment_count())
            .filter_map(|index| message.attachment(index as u32))
            .map(|part| MessageAttachment {
                file_name: part.attachment_name().map(str::to_string),
                content_type: part.content_type().map(|content_type| {
                    if let Some(subtype) = content_type.c_subtype.as_ref() {
                        format!("{}/{}", content_type.c_type, subtype)
                    } else {
                        content_type.c_type.to_string()
                    }
                }),
                is_inline: part
                    .content_disposition()
                    .map(|disposition| disposition.c_type.eq_ignore_ascii_case("inline"))
                    .unwrap_or(false),
                size: part.len(),
            })
            .collect::<Vec<_>>();

        return Ok(MessageContent {
            headers,
            plain_body,
            html_body: html_body.clone(),
            attachments,
            preferred_display: choose_preferred_display(raw, html_body.as_deref()),
        });
    }

    Ok(fallback_parse(raw))
}

fn choose_preferred_display(raw: &[u8], html_body: Option<&str>) -> MessageDisplayMode {
    let raw = String::from_utf8_lossy(raw).to_ascii_lowercase();
    let has_plain = raw.contains("content-type: text/plain");
    let has_html = raw.contains("content-type: text/html") || (html_body.is_some() && has_plain);

    if has_plain {
        MessageDisplayMode::Plain
    } else if has_html {
        MessageDisplayMode::Html
    } else {
        MessageDisplayMode::Plain
    }
}

fn fallback_parse(raw: &[u8]) -> MessageContent {
    let rendered = String::from_utf8_lossy(raw);
    let mut headers: Vec<MessageHeader> = Vec::new();
    let mut body = Vec::new();
    let mut in_body = false;
    let mut current_header: Option<usize> = None;

    for line in normalize_newlines(&rendered).lines() {
        if in_body {
            body.push(line.to_string());
            continue;
        }

        if line.trim().is_empty() {
            in_body = true;
            continue;
        }

        if (line.starts_with(' ') || line.starts_with('\t')) && current_header.is_some() {
            if let Some(header) = current_header.and_then(|index| headers.get_mut(index)) {
                if !header.value.is_empty() {
                    header.value.push(' ');
                }
                header.value.push_str(line.trim());
            }
            continue;
        }

        if let Some((name, value)) = line.split_once(':') {
            headers.push(MessageHeader {
                name: name.trim().to_string(),
                value: value.trim().to_string(),
            });
            current_header = Some(headers.len() - 1);
        } else {
            in_body = true;
            body.push(line.to_string());
        }
    }

    MessageContent {
        headers,
        plain_body: Some(body.join("\n")),
        html_body: None,
        attachments: Vec::new(),
        preferred_display: MessageDisplayMode::Plain,
    }
}

fn cow_to_normalized_string(input: Cow<'_, str>) -> String {
    normalize_newlines(input.as_ref())
}

#[cfg(test)]
mod tests {
    use super::parse_message;
    use crate::mail::message::MessageDisplayMode;

    #[test]
    fn parses_plain_message() {
        let raw = b"From: Demo <demo@example.com>\r\nSubject: Plain\r\n\r\nHello plain world.";
        let parsed = parse_message(raw).unwrap();

        assert_eq!(parsed.header_value("Subject"), Some("Plain"));
        assert_eq!(parsed.plain_body.as_deref(), Some("Hello plain world."));
        assert_eq!(parsed.preferred_display, MessageDisplayMode::Plain);
    }

    #[test]
    fn parses_html_only_message() {
        let raw = br#"From: Demo <demo@example.com>
Subject: HTML
MIME-Version: 1.0
Content-Type: text/html; charset=utf-8

<html><body><h1>Hello</h1><p>Rendered body</p></body></html>"#;
        let parsed = parse_message(raw).unwrap();

        assert!(parsed
            .html_body
            .as_deref()
            .unwrap_or_default()
            .contains("<h1>Hello</h1>"));
        assert_eq!(parsed.preferred_display, MessageDisplayMode::Html);
    }

    #[test]
    fn parses_multipart_alternative_message() {
        let raw = br#"From: Demo <demo@example.com>
Subject: Multipart
MIME-Version: 1.0
Content-Type: multipart/alternative; boundary="b"

--b
Content-Type: text/plain; charset=utf-8

Hello plain body.
--b
Content-Type: text/html; charset=utf-8

<html><body><p>Hello <strong>HTML</strong> body.</p></body></html>
--b--"#;
        let parsed = parse_message(raw).unwrap();

        assert_eq!(parsed.plain_body.as_deref(), Some("Hello plain body."));
        assert!(parsed
            .html_body
            .as_deref()
            .unwrap_or_default()
            .contains("HTML"));
        assert_eq!(parsed.preferred_display, MessageDisplayMode::Plain);
    }
}
