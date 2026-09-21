/*! Parse raw RFC 5322 bytes into a `MessageDocument` using `mail-parser`. */

use mail_parser::{Message as ParsedMessage, MessageParser};

use super::body::BodyDocument;
use super::headers::DecodedHeaders;
use super::part::{build_tree, Attachment, Part, PartBody};
use super::thread::ThreadRefs;
use super::{normalize_newlines, MessageDocument};

/// Parse raw message bytes. Parsing never fails: a malformed message degrades
/// to a plain-text document rather than an error.
pub fn parse(bytes: &[u8]) -> MessageDocument {
    let parser = MessageParser::new()
        .with_minimal_headers()
        .default_header_text();
    match parser.parse(bytes) {
        Some(message) => {
            let mut document = parse_message_object(&message);
            document.raw = Some(bytes.to_vec());
            document
        }
        None => fallback(bytes),
    }
}

/// Build a document from an already-parsed `mail-parser` message.
pub(super) fn parse_message_object(message: &ParsedMessage<'_>) -> MessageDocument {
    let headers = DecodedHeaders::from_message(message);
    let parts = build_tree(message);

    let mut plain_body = None;
    let mut html_body = None;
    let mut attachments = Vec::new();

    for part in &parts {
        part.walk(&mut |part| {
            if part.is_attachment() {
                if matches!(part.body, PartBody::Binary(_) | PartBody::Nested(_)) {
                    attachments.push(Attachment::from_part(part));
                }
                return;
            }
            match &part.body {
                PartBody::Text(text) if plain_body.is_none() => {
                    plain_body = Some(text.clone());
                }
                PartBody::Html(text) if html_body.is_none() => {
                    html_body = Some(text.clone());
                }
                _ => {}
            }
        });
    }

    let body = match &html_body {
        Some(html) => BodyDocument::from_html(html),
        None => plain_body
            .as_deref()
            .map(BodyDocument::from_plain)
            .unwrap_or_default(),
    };
    let thread = ThreadRefs::from_headers(&headers);

    MessageDocument {
        headers,
        parts,
        plain_body,
        html_body,
        body,
        attachments,
        thread,
        raw: None,
    }
}

/// Last-resort document for bytes the parser cannot structure.
fn fallback(bytes: &[u8]) -> MessageDocument {
    let text = normalize_newlines(&String::from_utf8_lossy(bytes));
    let mut parts = Vec::new();
    let mut headers: Vec<super::headers::HeaderField> = Vec::new();
    let mut body_lines = Vec::new();
    let mut in_body = false;

    for line in text.lines() {
        if in_body {
            body_lines.push(line.to_string());
            continue;
        }
        if line.trim().is_empty() {
            in_body = true;
            continue;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.push(super::headers::HeaderField {
                name: name.trim().to_string(),
                value: value.trim().to_string(),
            });
        } else {
            in_body = true;
            body_lines.push(line.to_string());
        }
    }

    let plain = body_lines.join("\n");
    parts.push(Part {
        content_type: "text/plain".to_string(),
        charset: Some("utf-8".to_string()),
        disposition: super::part::Disposition::Unspecified,
        filename: None,
        content_id: None,
        transfer_encoding: None,
        headers: Vec::new(),
        body: PartBody::Text(plain.clone()),
        children: Vec::new(),
    });

    MessageDocument {
        headers: DecodedHeaders {
            fields: headers,
            ..DecodedHeaders::default()
        },
        parts,
        plain_body: Some(plain.clone()),
        html_body: None,
        body: BodyDocument::from_plain(&plain),
        attachments: Vec::new(),
        thread: ThreadRefs::default(),
        raw: Some(bytes.to_vec()),
    }
}
