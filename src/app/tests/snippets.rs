/*! Reusable compose snippets. */

use crate::app::App;
use crate::compose::{ComposeMode, ComposeState};
use crate::keys::View;

#[test]
fn a_compose_body_can_be_saved_and_reinserted_as_a_snippet() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let mut app = App::new(Some("work".into()));
    app.db = Some(conn);

    let mut cs = ComposeState::new(ComposeMode::New, Some("work".into()));
    cs.body = crate::compose_editor::ComposeEditor::from_text("Thanks for the update.");
    app.compose_state = Some(cs);
    app.view = View::Compose;

    app.begin_save_snippet();
    assert_eq!(app.view, View::SnippetName);
    app.snippets.name_input.clear();
    for c in "Ack".chars() {
        app.snippet_name_input(c);
    }
    app.submit_snippet_name();
    assert_eq!(app.view, View::Snippets);
    assert_eq!(app.snippets.items.len(), 1);
    assert_eq!(app.snippets.items[0].name, "Ack");

    // Inserting drops the text into a fresh body.
    app.compose_state = Some(ComposeState::new(ComposeMode::New, Some("work".into())));
    app.snippets.index = 0;
    app.insert_snippet();
    let body = app.compose_state.as_ref().unwrap().body.text();
    assert!(body.contains("Thanks for the update."));
}

#[test]
fn saving_an_empty_body_is_refused() {
    let mut app = App::new(Some("work".into()));
    app.compose_state = Some(ComposeState::new(ComposeMode::New, Some("work".into())));
    app.view = View::Compose;
    app.begin_save_snippet();
    assert_eq!(app.view, View::Compose);
    assert!(app.status_message.contains("body first"));
}
