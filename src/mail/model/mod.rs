/*! Lossless, decoded mail message model.
`MessageDocument` is the single representation the rest of the application
consumes. Headers are decoded, the MIME tree is retained in full, threading
metadata is extracted, and one canonical terminal-native body is selected. */

mod address;
mod body;
mod headers;
mod parse;
mod part;
mod thread;

pub use address::Address;
pub use body::{Block, BodyDocument, Link};
pub use headers::{DecodedHeaders, HeaderField};
pub use part::{Attachment, Disposition, Part, PartBody};
pub use thread::{base_subject, ThreadRefs};

pub use parse::parse;

/// Normalize CRLF and bare CR to LF.
pub fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n").replace('\r', "\n")
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MessageDocument {
    /// Decoded headers, both ordered-display and typed.
    pub headers: DecodedHeaders,
    /// The full MIME part tree (root first).
    pub parts: Vec<Part>,
    /// Decoded `text/plain` source, when present.
    pub plain_body: Option<String>,
    /// Decoded `text/html` source, when present.
    pub html_body: Option<String>,
    /// The canonical body selected once for rendering.
    pub body: BodyDocument,
    /// Attachment and inline-part metadata.
    pub attachments: Vec<Attachment>,
    /// Message identity and threading relations.
    pub thread: ThreadRefs,
}

impl MessageDocument {
    pub fn header_value(&self, name: &str) -> Option<&str> {
        self.headers.get(name)
    }

    pub fn subject(&self) -> &str {
        self.headers.subject.as_deref().unwrap_or_default()
    }

    pub fn header_fields(&self) -> &[HeaderField] {
        &self.headers.fields
    }

    pub fn has_plain_body(&self) -> bool {
        self.plain_body
            .as_deref()
            .is_some_and(|body| !body.trim().is_empty())
    }

    pub fn has_html_body(&self) -> bool {
        self.html_body
            .as_deref()
            .is_some_and(|body| !body.trim().is_empty())
    }

    /// Render the canonical body at the requested width.
    pub fn render(&self, width: usize) -> String {
        self.body.render(width)
    }

    /// Headers plus body, for the legacy flat-string `read message` path.
    pub fn render_for_legacy_view(&self, width: usize) -> String {
        let mut out = String::new();
        for field in &self.headers.fields {
            out.push_str(&field.name);
            out.push_str(": ");
            out.push_str(&field.value);
            out.push('\n');
        }
        out.push('\n');
        out.push_str(&self.render(width));
        out
    }

    /// Plain text used for local search indexing.
    pub fn search_text(&self) -> String {
        self.body.to_search_text()
    }
}

impl MessageDocument {
    /// SPF/DKIM/DMARC verdicts reported by the receiving server, if present.
    pub fn authentication(&self) -> crate::mail::security::AuthenticationVerdicts {
        crate::mail::security::parse_authentication_results(
            &self.headers.get_all("Authentication-Results"),
        )
    }
}
