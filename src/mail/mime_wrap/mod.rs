/*! Shared MIME assembly for outbound protected messages.
Both PGP/MIME (RFC 3156) and S/MIME authenticate the *entity*, so the message
must be split into envelope headers and one MIME entity the same way, with CRLF
normalization, before either protection is applied. */

/// A message split into envelope headers and an inner MIME entity.
pub(crate) struct SplitMessage {
    /// Headers that belong on the outer message (From, To, Subject, …).
    pub(crate) envelope: Vec<String>,
    /// MIME headers of the inner entity (Content-Type, CTE, MIME-Version).
    pub(crate) mime_headers: Vec<String>,
    /// Body bytes of the inner entity.
    pub(crate) body: Vec<u8>,
}

impl SplitMessage {
    /// Recipient email addresses from To/Cc/Bcc, in header order.
    pub(crate) fn recipients(&self) -> Vec<String> {
        let mut addresses = Vec::new();
        for header in &self.envelope {
            let Some((name, value)) = header.split_once(':') else {
                continue;
            };
            if !matches!(
                name.trim().to_ascii_lowercase().as_str(),
                "to" | "cc" | "bcc"
            ) {
                continue;
            }
            addresses.extend(extract_addresses(value));
        }
        addresses
    }
}

/// Split a raw message at the first blank line.
pub(crate) fn split_message(raw: &[u8]) -> SplitMessage {
    let (head, body) = split_headers(raw);
    let text = String::from_utf8_lossy(head).replace("\r\n", "\n");
    let mut envelope = Vec::new();
    let mut mime_headers = Vec::new();
    for line in unfold(&text) {
        let name = line
            .split_once(':')
            .map(|(name, _)| name.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if matches!(
            name.as_str(),
            "content-type" | "content-transfer-encoding" | "mime-version"
        ) {
            mime_headers.push(line);
        } else {
            envelope.push(line);
        }
    }
    SplitMessage {
        envelope,
        mime_headers,
        body: body.to_vec(),
    }
}

/// The inner MIME entity, with CRLF line endings and its own MIME headers.
pub(crate) fn entity_bytes(message: &SplitMessage) -> Vec<u8> {
    let mut mime_headers = message.mime_headers.clone();
    if !mime_headers
        .iter()
        .any(|line| line.to_ascii_lowercase().starts_with("content-type:"))
    {
        mime_headers.push("Content-Type: text/plain; charset=utf-8".to_string());
    }
    entity_with(&mime_headers.join("\r\n"), &message.body)
}

/// Join MIME headers and a body into a CRLF entity, ensuring a trailing CRLF.
pub(crate) fn entity_with(mime_headers: &str, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(crlf(mime_headers).as_bytes());
    out.extend_from_slice(b"\r\n\r\n");
    out.extend_from_slice(&crlf_bytes(body));
    out.push(b'\r');
    out.push(b'\n');
    out
}

/// Assemble the outer message: envelope headers plus the protected payload.
pub(crate) fn assemble(envelope: &[String], content_type: &str, body: &[u8]) -> Vec<u8> {
    let mut headers = envelope.to_vec();
    headers.push("MIME-Version: 1.0".to_string());
    headers.push(format!("Content-Type: {content_type}"));
    let mut out = crlf(&headers.join("\r\n")).into_bytes();
    out.extend_from_slice(b"\r\n\r\n");
    out.extend_from_slice(body);
    out
}

/// Address-shaped substrings from a header value, lowercased.
pub(crate) fn extract_addresses(value: &str) -> Vec<String> {
    value
        .split([',', ';'])
        .filter_map(|chunk| {
            let start = chunk.rfind('<').map(|i| i + 1).unwrap_or(0);
            let end = chunk.rfind('>').unwrap_or(chunk.len());
            let candidate = chunk.get(start..end.max(start))?.trim();
            candidate.contains('@').then(|| candidate.to_lowercase())
        })
        .collect()
}

/// Unfold folded header lines (continuation lines start with whitespace).
pub(crate) fn unfold(head: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for line in head.lines() {
        if line.starts_with([' ', '\t']) {
            if let Some(last) = lines.last_mut() {
                last.push(' ');
                last.push_str(line.trim());
                continue;
            }
        }
        lines.push(line.trim_end().to_string());
    }
    lines
}

/// Split raw bytes at the blank line separating headers from body.
pub(crate) fn split_headers(raw: &[u8]) -> (&[u8], &[u8]) {
    for (index, window) in raw.windows(4).enumerate() {
        if window == b"\r\n\r\n" {
            return (&raw[..index], &raw[index + 4..]);
        }
    }
    for (index, window) in raw.windows(2).enumerate() {
        if window == b"\n\n" {
            return (&raw[..index], &raw[index + 2..]);
        }
    }
    (raw, &[])
}

/// Normalize lone LF to CRLF without doubling existing CRLF.
pub(crate) fn crlf(text: &str) -> String {
    String::from_utf8_lossy(&crlf_bytes(text.as_bytes())).into_owned()
}

fn crlf_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() + 16);
    let mut previous_cr = false;
    for &byte in bytes {
        if byte == b'\n' && !previous_cr {
            out.push(b'\r');
        }
        out.push(byte);
        previous_cr = byte == b'\r';
    }
    out
}

/// A boundary that does not appear in `content`.
pub(crate) fn unique_boundary(content: &[u8], kind: &str) -> String {
    loop {
        let boundary = format!("=-sfmail-{kind}-{:016x}", rand::random::<u64>());
        if !content
            .windows(boundary.len())
            .any(|window| window == boundary.as_bytes())
        {
            return boundary;
        }
    }
}

#[cfg(test)]
mod tests;
