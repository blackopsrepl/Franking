//! Tests for the account form's value types.

use super::ConnectionSecurity;

#[test]
fn a_port_suggests_its_usual_security() {
    assert_eq!(ConnectionSecurity::for_port(993), ConnectionSecurity::Tls);
    assert_eq!(ConnectionSecurity::for_port(465), ConnectionSecurity::Tls);
    // Submission, cleartext IMAP, and the local test ports.
    assert_eq!(
        ConnectionSecurity::for_port(143),
        ConnectionSecurity::StartTls
    );
    assert_eq!(
        ConnectionSecurity::for_port(587),
        ConnectionSecurity::StartTls
    );
    assert_eq!(
        ConnectionSecurity::for_port(1153),
        ConnectionSecurity::StartTls
    );
}

#[test]
fn security_cycles_through_every_choice() {
    let mut security = ConnectionSecurity::default();
    let mut seen = vec![security];
    for _ in 0..3 {
        security = security.next();
        seen.push(security);
    }
    assert_eq!(
        security,
        ConnectionSecurity::default(),
        "returns to the start"
    );
    assert!(seen.contains(&ConnectionSecurity::StartTls));
    assert!(seen.contains(&ConnectionSecurity::Plain));
}

#[test]
fn stored_names_round_trip() {
    for security in [
        ConnectionSecurity::Tls,
        ConnectionSecurity::StartTls,
        ConnectionSecurity::Plain,
    ] {
        assert_eq!(ConnectionSecurity::parse(Some(security.as_str())), security);
    }
    assert_eq!(
        ConnectionSecurity::parse(None),
        ConnectionSecurity::Tls,
        "an unset value is the secure default"
    );
}
