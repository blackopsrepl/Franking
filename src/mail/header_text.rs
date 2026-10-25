/*! Display text for raw header values.

IMAP ENVELOPE returns subjects and address display names exactly as they
appear in the message, so RFC 2047 encoded-words (`=?UTF-8?Q?...?=`) arrive
undecoded. Decode them with mail-parser's unstructured-header rules: adjacent
encoded-words join without the whitespace between them, any charset it knows
is converted, and malformed words stay as written. */

use mail_parser::parsers::MessageStream;
use mail_parser::HeaderValue;

/// Decode a raw header value into display text.
pub fn decode(raw: &[u8]) -> String {
    if !raw.windows(2).any(|pair| pair == b"=?") {
        return String::from_utf8_lossy(raw).trim().to_string();
    }
    let mut line = Vec::with_capacity(raw.len() + 2);
    line.extend_from_slice(raw);
    line.extend_from_slice(b"\r\n");
    match MessageStream::new(&line).parse_unstructured() {
        HeaderValue::Text(text) => text.trim().to_string(),
        _ => String::new(),
    }
}
