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
fn action_bar_cycle_includes_the_pgp_and_smime_toggles() {
    use super::state::FocusedField;

    assert_eq!(FocusedField::Attach.next(), FocusedField::Files);
    assert_eq!(FocusedField::Files.next(), FocusedField::Sign);
    assert_eq!(FocusedField::Sign.next(), FocusedField::Encrypt);
    assert_eq!(FocusedField::Encrypt.next(), FocusedField::SmimeSign);
    assert_eq!(FocusedField::SmimeSign.next(), FocusedField::SmimeEncrypt);
    assert_eq!(FocusedField::SmimeEncrypt.next(), FocusedField::Discard);
    assert_eq!(FocusedField::Discard.next(), FocusedField::From);
    assert_eq!(FocusedField::Sign.prev(), FocusedField::Files);
    assert_eq!(FocusedField::Files.prev(), FocusedField::Attach);
    assert_eq!(FocusedField::Encrypt.prev(), FocusedField::Sign);
    assert_eq!(FocusedField::SmimeSign.prev(), FocusedField::Encrypt);
    assert_eq!(FocusedField::SmimeEncrypt.prev(), FocusedField::SmimeSign);
}

#[test]
fn new_messages_get_the_identity_signature() {
    use crate::compose::{populate_from_template, ComposeMode, ComposeState};
    use crate::identities::Identity;

    let mut state = ComposeState::new(ComposeMode::New, Some("acct".to_string()));
    state.from_identities = vec![Identity {
        id: 1,
        account: "acct".to_string(),
        name: Some("Work".to_string()),
        display_name: Some("Alice".to_string()),
        email: "alice@example.com".to_string(),
        signature: Some("-- \nAlice Example".to_string()),
        sent_folder: None,
        is_default: true,
    }];
    state.from_idx = Some(0);

    populate_from_template(
        &mut state,
        "To: bob@example.com\nSubject: Hi\n\nHello there",
    );
    let body = state.body.text();
    assert!(body.contains("Hello there"), "{body}");
    assert!(body.contains("-- \nAlice Example"), "{body}");
}

#[test]
fn replies_do_not_get_a_signature_appended() {
    use crate::compose::{populate_from_template, ComposeMode, ComposeState};
    use crate::identities::Identity;

    let mut state = ComposeState::new(ComposeMode::Reply, Some("acct".to_string()));
    state.from_identities = vec![Identity {
        id: 1,
        account: "acct".to_string(),
        name: None,
        display_name: None,
        email: "alice@example.com".to_string(),
        signature: Some("-- \nAlice".to_string()),
        sent_folder: None,
        is_default: true,
    }];

    populate_from_template(
        &mut state,
        "To: bob@example.com\nSubject: Re: Hi\n\n> quoted",
    );
    assert!(!state.body.text().contains("-- \nAlice"));
}

#[test]
fn sending_restores_a_signature_the_body_lost() {
    use crate::compose::{signed_body, ComposeMode, ComposeState};
    use crate::identities::Identity;

    let mut state = ComposeState::new(ComposeMode::New, Some("acct".to_string()));
    state.from_identities = vec![Identity {
        id: 1,
        account: "acct".to_string(),
        name: Some("Work".to_string()),
        display_name: Some("Alice".to_string()),
        email: "alice@example.com".to_string(),
        signature: Some("-- \nAlice Example".to_string()),
        sent_folder: None,
        is_default: true,
    }];
    state.from_idx = Some(0);

    // A body the user edited after the signature was added.
    state.body = crate::compose_editor::ComposeEditor::from_text("hello there");
    let with_signature = signed_body(&state).expect("the signature is restored");
    assert!(
        with_signature.ends_with("-- \nAlice Example"),
        "{with_signature}"
    );

    // Applying the result is idempotent.
    state.body = crate::compose_editor::ComposeEditor::from_text(&with_signature);
    assert!(
        signed_body(&state).is_none(),
        "a body that already ends with the signature is left alone"
    );
}

#[test]
fn sending_without_a_signature_changes_nothing() {
    use crate::compose::{signed_body, ComposeMode, ComposeState};

    let mut state = ComposeState::new(ComposeMode::New, Some("acct".to_string()));
    state.body = crate::compose_editor::ComposeEditor::from_text("hello");
    assert!(signed_body(&state).is_none());
}
