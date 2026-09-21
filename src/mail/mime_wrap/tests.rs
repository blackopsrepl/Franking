use super::*;

/// Envelope headers, a plain-text entity, and a folded header to prove folding.
fn sample_message() -> Vec<u8> {
    [
        "From: Alice <alice@example.com>",
        "To: Bob <bob@example.com>, Carol <carol@example.com>",
        "Subject: Quarterly report",
        "Date: Mon, 21 Sep 2026 10:00:00 +0000",
        "Message-ID: <report@example.com>",
        "Content-Type: text/plain;",
        "\tcharset=utf-8",
        "",
        "Please review the attached numbers.",
    ]
    .join("\r\n")
    .into_bytes()
}

#[test]
fn splits_envelope_headers_from_the_mime_entity() {
    let message = split_message(&sample_message());
    assert!(message.envelope.iter().any(|h| h.starts_with("From:")));
    assert!(message.envelope.iter().any(|h| h.starts_with("To:")));
    assert!(!message
        .mime_headers
        .iter()
        .any(|h| h.to_ascii_lowercase().starts_with("from:")));
    assert_eq!(
        message.recipients(),
        vec!["bob@example.com", "carol@example.com"]
    );
}

#[test]
fn unfolds_folded_headers_into_one_line() {
    let message = split_message(&sample_message());
    assert!(
        message
            .mime_headers
            .iter()
            .any(|h| h == "Content-Type: text/plain; charset=utf-8"),
        "folded header rejoined: {:?}",
        message.mime_headers
    );
}

#[test]
fn extracts_addresses_from_display_names_and_lists() {
    assert_eq!(
        extract_addresses("Alice <alice@example.com>, Bob <BOB@example.com>"),
        vec!["alice@example.com", "bob@example.com"]
    );
    assert_eq!(
        extract_addresses("carol@example.com"),
        vec!["carol@example.com"]
    );
    assert!(extract_addresses("undisclosed-recipients:;").is_empty());
}

#[test]
fn crlf_normalization_is_idempotent() {
    assert_eq!(crlf("a\nb"), "a\r\nb");
    assert_eq!(crlf("a\r\nb"), "a\r\nb");
    assert_eq!(crlf("a\r\nb\nc"), "a\r\nb\r\nc");
}

#[test]
fn boundary_avoids_content_collisions() {
    let content = b"=-sfmail-signed-0000000000000000";
    let boundary = unique_boundary(content, "signed");
    assert!(!content
        .windows(boundary.len())
        .any(|window| window == boundary.as_bytes()));
}

#[test]
fn entity_carries_mime_headers_and_a_trailing_crlf() {
    let message = split_message(&sample_message());
    let entity = entity_bytes(&message);
    let text = String::from_utf8_lossy(&entity);
    assert!(text.starts_with("Content-Type: text/plain; charset=utf-8\r\n"));
    assert!(text.ends_with("Please review the attached numbers.\r\n"));
}
