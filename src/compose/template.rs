/*! Compose template parsing, population, and reassembly. */

use crate::compose_editor::ComposeEditor;

use super::state::{ComposeMode, ComposeState, FocusedField};

/// Parsed headers from an MML template.
pub(crate) struct ParsedHeaders {
    from: Option<String>,
    to: String,
    cc: String,
    bcc: String,
    subject: String,
    in_reply_to: Option<String>,
    references: Option<String>,
    /// All unrecognised header lines (preserved verbatim).
    extra: Vec<String>,
}

/// Parse a backend template string into its components.
///
/// The template format is:
/// ```text
/// From: sender@example.com
/// To: recipient@example.com
/// Subject: Hello
///
/// Body text starts here.
/// ```
///
/// The header block ends at the first blank line.
pub(crate) fn parse_template(raw: &str) -> (ParsedHeaders, String) {
    let mut from = None;
    let mut to = String::new();
    let mut cc = String::new();
    let mut bcc = String::new();
    let mut subject = String::new();
    let mut in_reply_to = None;
    let mut references = None;
    let mut extra = Vec::new();
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
        // Try to split "Header: value"
        if let Some(colon) = line.find(':') {
            let key = line[..colon].trim().to_lowercase();
            let value = line[colon + 1..].trim().to_string();
            match key.as_str() {
                "from" => from = Some(value),
                "to" => to = value,
                "cc" => cc = value,
                "bcc" => bcc = value,
                "subject" => subject = value,
                "in-reply-to" => in_reply_to = Some(value),
                "references" => references = Some(value),
                _ => extra.push(line.to_string()),
            }
        } else {
            extra.push(line.to_string());
        }
    }

    let body = body_lines.join("\n");
    (
        ParsedHeaders {
            from,
            to,
            cc,
            bcc,
            subject,
            in_reply_to,
            references,
            extra,
        },
        body,
    )
}

/// Populate a `ComposeState` from a raw backend template string.
pub fn populate_from_template(state: &mut ComposeState, raw: &str) {
    let (headers, body) = parse_template(raw);
    state.to = headers.to;
    state.cc = headers.cc;
    state.bcc = headers.bcc;
    state.subject = headers.subject;
    state.in_reply_to = headers.in_reply_to;
    state.references = headers.references;

    state.body = ComposeEditor::from_text(&body);

    // Focus From for new messages (so user can pick identity first),
    // body for replies (quote is already there).
    // But if there are no identities configured, skip straight to To.
    state.focused = match state.mode {
        ComposeMode::New => {
            if state.from_identities.is_empty() {
                FocusedField::To
            } else {
                FocusedField::From
            }
        }
        _ => FocusedField::Body,
    };
    state.dirty = false;
    let _ = headers.from; // used by display if needed
    let _ = headers.extra;
}

// ── Template reassembly ──────────────────────────────────────────────────────

/// Reassemble a ComposeState into a backend template string for sending.
pub fn reassemble_template(state: &ComposeState) -> String {
    let mut out = String::new();

    // Emit From: when the user has explicitly selected an identity.
    if let Some(identity) = state.selected_identity() {
        out.push_str(&format!("From: {}\n", identity.formatted()));
    }

    if !state.to.is_empty() {
        out.push_str(&format!("To: {}\n", state.to));
    }
    if !state.cc.is_empty() {
        out.push_str(&format!("Cc: {}\n", state.cc));
    }
    if !state.bcc.is_empty() {
        out.push_str(&format!("Bcc: {}\n", state.bcc));
    }
    if !state.subject.is_empty() {
        out.push_str(&format!("Subject: {}\n", state.subject));
    }
    if let Some(in_reply_to) = state
        .in_reply_to
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        out.push_str(&format!("In-Reply-To: {in_reply_to}\n"));
    }
    if let Some(references) = state
        .references
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        out.push_str(&format!("References: {references}\n"));
    }

    out.push('\n'); // blank line separating headers from body

    let body = state.body.text();
    out.push_str(&body);

    out
}

/// Check if the body has any non-whitespace content.
pub fn body_is_empty(state: &ComposeState) -> bool {
    state.body.is_empty()
}
