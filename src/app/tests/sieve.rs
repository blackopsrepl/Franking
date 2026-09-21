/*! App unit tests: Sieve filters. */

use crate::mail::sieve::SieveScript;
use crate::mail::MailError;

use super::super::App;

#[test]
fn sieve_scripts_are_stored_and_the_index_clamps() {
    let mut app = App::new(None);
    app.sieve.index = 5;
    app.handle_sieve_scripts(Ok(vec![
        SieveScript {
            name: "main".to_string(),
            active: true,
        },
        SieveScript {
            name: "backup".to_string(),
            active: false,
        },
    ]));
    assert_eq!(app.sieve.scripts.len(), 2);
    assert_eq!(app.sieve.index, 1);
    assert!(!app.loading);
}

#[test]
fn sieve_errors_are_surfaced() {
    let mut app = App::new(None);
    app.handle_sieve_scripts(Err(MailError::other("boom")));
    assert!(app.status_is_error);
    assert!(app.status_message.contains("boom"));
}

#[test]
fn new_sieve_script_opens_the_editor_with_a_template() {
    use crate::keys::View;

    let mut app = App::new(None);
    app.sieve_new();
    assert_eq!(app.view, View::SieveName);
    assert_eq!(app.sieve.name, "solverforge");

    app.sieve.name.clear();
    app.sieve_name_submit();
    assert!(app.status_is_error, "empty name is rejected");
    assert_eq!(app.view, View::SieveName);

    for c in "bot".chars() {
        app.sieve_name_input(c);
    }
    app.sieve_name_submit();
    assert_eq!(app.view, View::SieveEdit);
    assert_eq!(app.sieve.editor_name, "bot");
    assert!(app
        .sieve
        .editor
        .as_ref()
        .expect("editor")
        .text()
        .contains("fileinto"));
}

#[test]
fn deleting_a_sieve_script_requires_two_presses() {
    let mut app = App::new(None);
    app.sieve.scripts = vec![SieveScript {
        name: "main".to_string(),
        active: true,
    }];

    app.sieve_delete();
    assert_eq!(app.sieve.pending_delete.as_deref(), Some("main"));
    assert!(app.status_message.contains("Press d again"));
}
