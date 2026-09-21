/*! App unit tests. */

use crate::himalaya::diagnostics as himalaya_diagnostics;

#[test]
fn explain_himalaya_error_flags_maildir_as_non_auth() {
    let raw = "himalaya error: cannot open local maildir";
    let explained = himalaya_diagnostics::explain(Some("maildir"), raw);
    assert!(explained.contains("not an authentication error"));
}

#[test]
fn explain_himalaya_error_flags_keyring_failures() {
    let raw = "himalaya error: failed to talk to Secret Service keyring";
    let explained = himalaya_diagnostics::explain(Some("imap"), raw);
    assert!(explained.contains("Keyring secret missing or inaccessible"));
}

#[test]
fn explain_himalaya_error_flags_oauth_failures() {
    let raw = "himalaya error: invalid_grant while refreshing oauth token";
    let explained = himalaya_diagnostics::explain(Some("imap"), raw);
    assert!(explained.contains("OAuth credentials need reconfiguration"));
}

#[test]
fn unlock_prompt_adopts_typed_passphrase() {
    use super::App;
    use crate::keys::View;

    let mut app = App::new(None);
    app.enter_unlock_prompt();
    assert_eq!(app.view, View::PassphrasePrompt);

    for c in "secret".chars() {
        app.unlock_input(c);
    }
    app.unlock_backspace();
    app.submit_unlock();

    assert_eq!(app.view, View::MessageView);
    assert_eq!(app.crypto_passphrase, "secre");
    assert!(app.unlock_input.is_empty());
}

#[test]
fn unlock_cancel_keeps_cached_passphrase() {
    use super::App;
    use crate::keys::View;

    let mut app = App::new(None);
    app.crypto_passphrase = "cached".to_string();
    app.enter_unlock_prompt();
    app.unlock_input('x');
    app.cancel_unlock();

    assert_eq!(app.view, View::MessageView);
    assert!(app.unlock_input.is_empty());
    assert_eq!(app.crypto_passphrase, "cached");
}

#[test]
fn compose_toggles_signing_and_encryption() {
    use crate::compose::{ComposeMode, ComposeState, FocusedField};

    use super::App;

    let mut app = App::new(None);
    app.compose_state = Some(ComposeState::new(ComposeMode::New, None));

    let focused = |app: &mut App, field: FocusedField| {
        app.compose_state.as_mut().expect("compose").focused = field;
        app.compose_enter_insert();
    };

    focused(&mut app, FocusedField::Sign);
    assert!(app.compose_state.as_ref().expect("compose").sign);

    focused(&mut app, FocusedField::Encrypt);
    assert!(app.compose_state.as_ref().expect("compose").encrypt);
    focused(&mut app, FocusedField::Encrypt);
    assert!(!app.compose_state.as_ref().expect("compose").encrypt);
}

#[test]
fn trust_signer_without_a_candidate_reports_status() {
    use super::App;

    let mut app = App::new(None);
    app.trust_signer();
    assert!(app.status_message.contains("No untrusted S/MIME signer"));
}

#[test]
fn smime_status_distinguishes_trusted_from_untrusted_signers() {
    use crate::mail::smime::{SmimeSigner, SmimeVerification};

    use super::smime::signature_outcome;

    let signer = SmimeSigner {
        subject: "CN=Alice, emailAddress=alice@example.com".to_string(),
        issuer: "CN=Alice".to_string(),
        fingerprint: "ab".repeat(32),
        trusted: false,
        not_before: String::new(),
        not_after: String::new(),
        der: vec![1, 2, 3],
    };

    let untrusted = signature_outcome(SmimeVerification {
        content: Some(vec![1]),
        signers: vec![signer.clone()],
    });
    assert!(untrusted.status.contains("untrusted"));
    assert!(untrusted.status.contains("alice@example.com"));
    assert!(untrusted.untrusted.is_some());

    let trusted = signature_outcome(SmimeVerification {
        content: Some(vec![1]),
        signers: vec![SmimeSigner {
            trusted: true,
            ..signer
        }],
    });
    assert!(trusted.status.contains("valid"));
    assert!(trusted.status.contains("trusted"));
    assert!(trusted.untrusted.is_none());
}

#[test]
fn untrusted_smime_signer_is_offered_for_trust() {
    use base64::Engine;
    use openssl::hash::MessageDigest;
    use openssl::pkcs7::{Pkcs7, Pkcs7Flags};
    use openssl::pkey::PKey;
    use openssl::rsa::Rsa;
    use openssl::stack::Stack;
    use openssl::x509::{X509NameBuilder, X509};

    use super::smime::process_smime;

    let rsa = Rsa::generate(2048).unwrap();
    let key = PKey::from_rsa(rsa).unwrap();
    let mut name = X509NameBuilder::new().unwrap();
    name.append_entry_by_text("CN", "Alice").unwrap();
    name.append_entry_by_text("emailAddress", "alice@example.com")
        .unwrap();
    let name = name.build();
    let mut builder = X509::builder().unwrap();
    builder.set_version(2).unwrap();
    builder.set_subject_name(&name).unwrap();
    builder.set_issuer_name(&name).unwrap();
    builder.set_pubkey(&key).unwrap();
    builder
        .set_not_before(&openssl::asn1::Asn1Time::days_from_now(0).unwrap())
        .unwrap();
    builder
        .set_not_after(&openssl::asn1::Asn1Time::days_from_now(30).unwrap())
        .unwrap();
    builder.sign(&key, MessageDigest::sha256()).unwrap();
    let cert = builder.build();
    let serial =
        openssl::asn1::Asn1Integer::from_bn(&openssl::bn::BigNum::from_u32(7).unwrap()).unwrap();
    let _ = serial;

    let signed_part = "Content-Type: text/plain; charset=utf-8\r\n\r\nsigned body";
    let certs = Stack::new().unwrap();
    let cms = Pkcs7::sign(
        &cert,
        &key,
        &certs,
        signed_part.as_bytes(),
        Pkcs7Flags::BINARY,
    )
    .unwrap()
    .to_der()
    .unwrap();
    let encoded = base64::engine::general_purpose::STANDARD
        .encode(&cms)
        .as_bytes()
        .chunks(76)
        .map(|chunk| String::from_utf8_lossy(chunk).to_string())
        .collect::<Vec<_>>()
        .join("\r\n");

    let raw = format!(
        "MIME-Version: 1.0\r\nContent-Type: multipart/signed; micalg=sha-256; protocol=\"application/pkcs7-signature\"; boundary=\"s\"\r\n\r\n--s\r\n{signed_part}\r\n--s\r\nContent-Type: application/pkcs7-signature; name=\"smime.p7s\"\r\nContent-Transfer-Encoding: base64\r\n\r\n{encoded}\r\n--s--\r\n"
    );

    let mut document = crate::mail::mime::parse_message(raw.as_bytes()).unwrap();
    let outcome = process_smime(&mut document).expect("smime outcome");
    assert!(outcome.status.contains("press T to trust"));
    let signer = outcome.untrusted.expect("signer offered for trust");
    assert!(signer.subject.contains("CN=Alice"));
    assert!(!signer.trusted);
    assert!(!signer.der.is_empty());
}

#[test]
fn attachment_list_opens_navigates_and_closes() {
    use crate::keys::View;

    use super::App;

    let raw = b"MIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=\"m\"\r\n\r\n--m\r\nContent-Type: text/plain\r\n\r\nhello\r\n--m\r\nContent-Type: application/pdf; name=\"report.pdf\"\r\nContent-Disposition: attachment; filename=\"report.pdf\"\r\nContent-Transfer-Encoding: base64\r\n\r\nAAECAwQ=\r\n--m--\r\n";
    let document = crate::mail::mime::parse_message(raw).unwrap();

    let mut app = App::new(None);
    app.message_content = Some(document);

    app.open_attachments();
    assert_eq!(app.view, View::AttachmentList);
    assert_eq!(app.attachment_index, 0);

    app.attachment_next();
    assert_eq!(app.attachment_index, 0, "single attachment clamps");
    app.attachment_prev();
    assert_eq!(app.attachment_index, 0);

    app.close_attachments();
    assert_eq!(app.view, View::MessageView);
}

#[test]
fn attachment_actions_without_a_message_report_status() {
    use super::App;

    let mut app = App::new(None);
    app.open_attachments();
    assert_eq!(app.view, crate::keys::View::EnvelopeList);

    app.save_selected_attachment();
    assert!(app.status_message.contains("No attachment is selected"));
}

#[test]
fn folder_prompts_open_with_the_expected_input() {
    use crate::keys::View;

    use super::folders::FolderPromptKind;
    use super::App;

    let mut app = App::new(None);
    app.current_folder = "Archive".to_string();

    app.folder_prompt_new();
    assert_eq!(app.view, View::FolderPrompt);
    let prompt = app.folder_prompt.as_ref().expect("prompt");
    assert_eq!(prompt.kind, FolderPromptKind::Create);
    assert!(prompt.input.is_empty());

    app.folder_prompt_rename();
    let prompt = app.folder_prompt.as_ref().expect("prompt");
    assert_eq!(prompt.kind, FolderPromptKind::Rename);
    assert_eq!(prompt.input, "Archive");

    app.folder_prompt_delete();
    assert_eq!(
        app.folder_prompt.as_ref().expect("prompt").kind,
        FolderPromptKind::Delete
    );
}

#[test]
fn folder_prompt_edits_and_cancels() {
    use crate::keys::View;

    use super::App;

    let mut app = App::new(None);
    app.folder_prompt_new();
    for c in "Receipts".chars() {
        app.folder_prompt_input(c);
    }
    app.folder_prompt_backspace();
    assert_eq!(app.folder_prompt.as_ref().expect("prompt").input, "Receipt");

    app.cancel_folder_prompt();
    assert_eq!(app.view, View::FolderList);
    assert!(app.folder_prompt.is_none());
}

#[test]
fn deleting_a_folder_requires_confirmation() {
    use super::App;

    let mut app = App::new(None);
    app.current_folder = "Old".to_string();
    app.folder_prompt_delete();

    app.folder_prompt_input('n');
    app.folder_prompt_input('o');
    app.submit_folder_prompt();
    assert!(app.status_message.contains("not deleted"));
    assert!(!app.pending_folder_refresh);
}
