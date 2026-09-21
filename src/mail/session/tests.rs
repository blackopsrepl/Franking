use super::{Capabilities, Security};

#[test]
fn capabilities_map_known_extensions() {
    let capabilities = Capabilities::from_names([
        "IMAP4rev1",
        "IDLE",
        "MOVE",
        "UIDPLUS",
        "CONDSTORE",
        "SPECIAL-USE",
        "SORT",
        "THREAD",
        "UTF8=ACCEPT",
        "COMPRESS=DEFLATE",
        "AUTH=PLAIN",
        "AUTH=XOAUTH2",
    ]);

    assert!(capabilities.imap4rev1);
    assert!(capabilities.idle);
    assert!(capabilities.move_);
    assert!(capabilities.uidplus);
    assert!(capabilities.condstore);
    assert!(capabilities.special_use);
    assert!(capabilities.sort);
    assert!(capabilities.thread);
    assert!(capabilities.utf8_accept);
    assert!(capabilities.compress_deflate);
    assert!(capabilities.auth_plain);
    assert!(capabilities.auth_xoauth2);
    assert!(!capabilities.qresync);
    assert!(capabilities.supports("idle"));
}

#[test]
fn security_normalizes_aliases() {
    assert_eq!(Security::normalize(Some("imaps"), "plain"), Security::Tls);
    assert_eq!(Security::normalize(Some("ssl"), "plain"), Security::Tls);
    assert_eq!(
        Security::normalize(Some("starttls"), "tls"),
        Security::StartTls
    );
    assert_eq!(Security::normalize(Some("none"), "tls"), Security::Plain);
    assert_eq!(Security::normalize(None, "tls"), Security::Tls);
}
