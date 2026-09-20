/*! A lossless MIME part tree.
Every part retains its content type, charset, disposition, transfer encoding,
decoded content, and children, so callers can walk the structure instead of
relying on flat "first text/plain" accessors. */

use mail_parser::{ContentType, Message, MessagePart, MimeHeaders, PartType};

use super::headers::{render_header_value, HeaderField};
use super::normalize_newlines;
use super::parse::parse_message_object;
use super::MessageDocument;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    Inline,
    Attachment,
    Unspecified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartBody {
    Text(String),
    Html(String),
    Binary(Vec<u8>),
    Nested(Box<MessageDocument>),
    Multipart,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub content_type: String,
    pub charset: Option<String>,
    pub disposition: Disposition,
    pub filename: Option<String>,
    pub content_id: Option<String>,
    pub transfer_encoding: Option<String>,
    pub headers: Vec<HeaderField>,
    pub body: PartBody,
    pub children: Vec<Part>,
}

impl Part {
    pub fn is_multipart(&self) -> bool {
        matches!(self.body, PartBody::Multipart)
    }

    pub fn is_attachment(&self) -> bool {
        self.disposition == Disposition::Attachment
            || (self.filename.is_some() && self.disposition != Disposition::Inline)
    }

    pub fn text(&self) -> Option<&str> {
        match &self.body {
            PartBody::Text(text) | PartBody::Html(text) => Some(text),
            _ => None,
        }
    }

    pub fn size(&self) -> usize {
        match &self.body {
            PartBody::Text(text) | PartBody::Html(text) => text.len(),
            PartBody::Binary(bytes) => bytes.len(),
            PartBody::Nested(_) | PartBody::Multipart => 0,
        }
    }

    /// Depth-first walk over this part and all descendants.
    pub fn walk(&self, visit: &mut impl FnMut(&Part)) {
        visit(self);
        for child in &self.children {
            child.walk(visit);
        }
    }
}

/// Attachment metadata projected from a part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    pub file_name: Option<String>,
    pub content_type: Option<String>,
    pub is_inline: bool,
    pub size: usize,
    pub content_id: Option<String>,
}

impl Attachment {
    pub fn from_part(part: &Part) -> Self {
        Self {
            file_name: part.filename.clone(),
            content_type: Some(part.content_type.clone()),
            is_inline: part.disposition == Disposition::Inline,
            size: part.size(),
            content_id: part.content_id.clone(),
        }
    }
}

pub(super) fn build_tree(message: &Message<'_>) -> Vec<Part> {
    message
        .part(0)
        .map(|root| vec![build_part(message, root)])
        .unwrap_or_default()
}

fn build_part(message: &Message<'_>, part: &MessagePart<'_>) -> Part {
    let content_type = part
        .content_type()
        .map(format_content_type)
        .unwrap_or_else(|| "text/plain".to_string());
    let charset = part
        .content_type()
        .and_then(|content_type| content_type.attribute("charset"))
        .map(str::to_string);
    let disposition = match part
        .content_disposition()
        .map(|value| value.c_type.as_ref())
    {
        Some(value) if value.eq_ignore_ascii_case("inline") => Disposition::Inline,
        Some(value) if value.eq_ignore_ascii_case("attachment") => Disposition::Attachment,
        _ => Disposition::Unspecified,
    };
    let filename = part
        .attachment_name()
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty());
    let content_id = part
        .content_id()
        .map(|id| {
            id.trim()
                .trim_start_matches('<')
                .trim_end_matches('>')
                .to_string()
        })
        .filter(|id| !id.is_empty());
    let transfer_encoding = part.content_transfer_encoding().map(str::to_string);
    let headers = part
        .headers
        .iter()
        .filter_map(|header| {
            render_header_value(&header.value).map(|value| HeaderField {
                name: header.name.to_string(),
                value,
            })
        })
        .collect();

    let (body, children) = match &part.body {
        PartType::Text(text) => (PartBody::Text(normalize_newlines(text)), Vec::new()),
        PartType::Html(html) => (PartBody::Html(normalize_newlines(html)), Vec::new()),
        PartType::Binary(bytes) | PartType::InlineBinary(bytes) => {
            (PartBody::Binary(bytes.to_vec()), Vec::new())
        }
        PartType::Message(nested) => (
            PartBody::Nested(Box::new(parse_message_object(nested))),
            Vec::new(),
        ),
        PartType::Multipart(ids) => {
            let children = ids
                .iter()
                .filter_map(|id| message.part(*id))
                .map(|child| build_part(message, child))
                .collect();
            (PartBody::Multipart, children)
        }
    };

    Part {
        content_type,
        charset,
        disposition,
        filename,
        content_id,
        transfer_encoding,
        headers,
        body,
        children,
    }
}

fn format_content_type(content_type: &ContentType<'_>) -> String {
    match content_type.c_subtype.as_deref() {
        Some(subtype) => format!("{}/{subtype}", content_type.c_type),
        None => content_type.c_type.to_string(),
    }
}
