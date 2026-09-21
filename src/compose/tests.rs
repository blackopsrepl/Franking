/*! Compose template unit tests. */

use super::state::{ComposeMode, ComposeState};
use super::template::{populate_from_template, reassemble_template};

#[test]
fn template_round_trip_preserves_threading_headers() {
    let raw = "To: bob@example.com\nSubject: Re: Project\nIn-Reply-To: <child@example.com>\nReferences: <root@example.com> <child@example.com>\n\nquoted body";
    let mut state = ComposeState::new(ComposeMode::Reply, Some("work".to_string()));

    populate_from_template(&mut state, raw);

    assert_eq!(state.in_reply_to.as_deref(), Some("<child@example.com>"));
    assert_eq!(
        state.references.as_deref(),
        Some("<root@example.com> <child@example.com>")
    );

    let rebuilt = reassemble_template(&state);
    assert!(rebuilt.contains("In-Reply-To: <child@example.com>"));
    assert!(rebuilt.contains("References: <root@example.com> <child@example.com>"));
}

#[test]
fn template_round_trip_preserves_attachments() {
    let raw = "To: bob@example.com\nSubject: Files\nAttachment: /tmp/a.txt\nAttachment: /tmp/b.pdf\n\nbody";
    let mut state = ComposeState::new(ComposeMode::New, None);

    populate_from_template(&mut state, raw);

    assert_eq!(
        state.attachments,
        vec!["/tmp/a.txt".to_string(), "/tmp/b.pdf".to_string()]
    );

    let rebuilt = reassemble_template(&state);
    assert!(rebuilt.contains("Attachment: /tmp/a.txt"));
    assert!(rebuilt.contains("Attachment: /tmp/b.pdf"));
}

#[test]
fn action_bar_cycle_includes_the_crypto_toggles() {
    use super::state::FocusedField;

    assert_eq!(FocusedField::Attach.next(), FocusedField::Files);
    assert_eq!(FocusedField::Files.next(), FocusedField::Sign);
    assert_eq!(FocusedField::Sign.next(), FocusedField::Encrypt);
    assert_eq!(FocusedField::Encrypt.next(), FocusedField::Discard);
    assert_eq!(FocusedField::Discard.next(), FocusedField::From);
    assert_eq!(FocusedField::Sign.prev(), FocusedField::Files);
    assert_eq!(FocusedField::Files.prev(), FocusedField::Attach);
    assert_eq!(FocusedField::Encrypt.prev(), FocusedField::Sign);
}
