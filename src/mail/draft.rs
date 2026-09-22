/*! Build a compose template from a stored draft message. */

use super::model::MessageDocument;

/// True when a header value contains a line break, which would allow header
/// injection (RFC 5322 forbids CR/LF in field values).
pub fn has_header_injection(value: &str) -> bool {
    value.contains('\n') || value.contains('\r')
}

/// Render a parsed draft back into the compose template format so it can be
/// resumed in the editor.
pub fn draft_template(document: &MessageDocument) -> String {
    let mut out = String::new();
    for header in ["To", "Cc", "Bcc", "Subject"] {
        if let Some(value) = document.header_value(header) {
            if !value.trim().is_empty() {
                out.push_str(&format!("{header}: {}\n", value.trim()));
            }
        }
    }
    out.push('\n');
    out.push_str(&document.render(100_000));
    out
}

#[cfg(test)]
mod tests {
    use super::{draft_template, has_header_injection};
    use crate::mail::mime;

    #[test]
    fn detects_header_injection() {
        assert!(!has_header_injection("Plain subject"));
        assert!(has_header_injection("Subject\r\nBcc: evil@example.com"));
        assert!(has_header_injection("line\nbreak"));
    }

    #[test]
    fn builds_a_template_from_a_draft() {
        let raw = b"From: alice@example.com\r\nTo: bob@example.com\r\nCc: team@example.com\r\nSubject: Draft subject\r\n\r\nwork in progress";
        let document = mime::parse_message(raw).unwrap();
        let template = draft_template(&document);

        assert!(template.contains("To: bob@example.com"));
        assert!(template.contains("Cc: team@example.com"));
        assert!(template.contains("Subject: Draft subject"));
        assert!(template.ends_with("work in progress"));
    }
}
