/*! Raw-message parsing entry point.
Thin adapter over the lossless model so backends keep one raw-bytes-to-document
boundary regardless of whether they read from disk or the network. */

use super::errors::MailResult;
use super::model::{parse, MessageDocument};

pub fn parse_message(raw: impl AsRef<[u8]>) -> MailResult<MessageDocument> {
    Ok(parse(raw.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::parse_message;

    #[test]
    fn parses_plain_message() {
        let raw = b"From: Demo <demo@example.com>\r\nSubject: Plain\r\n\r\nHello plain world.";
        let parsed = parse_message(raw).unwrap();

        assert_eq!(parsed.header_value("Subject"), Some("Plain"));
        assert_eq!(parsed.plain_body.as_deref(), Some("Hello plain world."));
        assert!(parsed.has_plain_body());
        assert!(!parsed.has_html_body());
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
        assert!(parsed.has_html_body());
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
    }

    #[test]
    fn decodes_rfc2047_subject_and_address_names() {
        let raw = b"From: =?UTF-8?B?SsO2cmc=?= <j@example.com>\r\nSubject: =?UTF-8?B?SMOpbGxv?=\r\n\r\nbody";
        let parsed = parse_message(raw).unwrap();

        assert_eq!(parsed.subject(), "Héllo");
        assert_eq!(parsed.headers.from[0].name.as_deref(), Some("Jörg"));
        assert_eq!(
            parsed.headers.from[0].email.as_deref(),
            Some("j@example.com")
        );
    }

    #[test]
    fn decodes_rfc2231_attachment_filename() {
        let raw = br#"MIME-Version: 1.0
Content-Type: multipart/mixed; boundary="n"

--n
Content-Type: text/plain

body
--n
Content-Type: application/pdf
Content-Disposition: attachment; filename*=utf-8''na%C3%AFve.pdf

PDFDATA
--n--"#;
        let parsed = parse_message(raw).unwrap();

        assert_eq!(parsed.attachments.len(), 1);
        assert_eq!(
            parsed.attachments[0].file_name.as_deref(),
            Some("naïve.pdf")
        );
    }

    #[test]
    fn extracts_threading_headers() {
        let raw = b"Subject: Re: Project update\r\nMessage-ID: <child@example.com>\r\nReferences: <root@example.com> <mid@example.com>\r\nIn-Reply-To: <mid@example.com>\r\n\r\nreply";
        let parsed = parse_message(raw).unwrap();

        assert_eq!(
            parsed.thread.message_id.as_deref(),
            Some("child@example.com")
        );
        assert_eq!(parsed.thread.root_id(), Some("root@example.com"));
        assert_eq!(parsed.thread.parent_id(), Some("mid@example.com"));
        assert_eq!(parsed.thread.base_subject, "Project update");
        assert!(parsed.thread.is_reply);
    }

    #[test]
    fn preserves_multiple_text_parts_in_the_tree() {
        let raw = br#"MIME-Version: 1.0
Content-Type: multipart/mixed; boundary="m"

--m
Content-Type: text/plain

FIRST
--m
Content-Type: text/plain

SECOND
--m--"#;
        let parsed = parse_message(raw).unwrap();

        let mut texts = Vec::new();
        for part in &parsed.parts {
            part.walk(&mut |part| {
                if let Some(text) = part.text() {
                    texts.push(text.trim().to_string());
                }
            });
        }
        assert!(texts.contains(&"FIRST".to_string()));
        assert!(texts.contains(&"SECOND".to_string()));
    }
}
