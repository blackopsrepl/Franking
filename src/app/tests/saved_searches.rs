/*! Saved-search overlay tests. */

use crate::db::saved_searches::{self as store, SavedSearch};
use crate::keys::View;

use super::super::App;

/// An app with a working local database.
fn app() -> App {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let mut app = App::new(None);
    app.db = Some(conn);
    app
}

#[test]
fn the_active_query_can_be_saved_under_a_name_and_re_run() {
    let mut app = app();
    app.active_query = Some("subject quarterly".to_string());
    app.search_all_folders = true;

    app.begin_save_search();
    assert_eq!(app.view, View::SaveSearch);
    assert_eq!(app.saved_searches.name_input, "subject quarterly");

    // The name is seeded from the query, so typing appends; start over.
    app.saved_searches.name_input.clear();
    for c in "Quarterly".chars() {
        app.save_search_input(c);
    }
    app.submit_save_search();

    assert_eq!(app.view, View::Search, "back to the prompt");
    let stored = store::list(app.db.as_ref().unwrap()).unwrap();
    assert_eq!(
        stored,
        vec![SavedSearch {
            name: "Quarterly".to_string(),
            query: "subject quarterly".to_string(),
            all_folders: true,
        }]
    );

    // Running it restores the query and its scope.
    app.active_query = None;
    app.search_all_folders = false;
    app.open_saved_searches();
    assert_eq!(app.view, View::SavedSearches);
    app.saved_searches.index = 0;
    app.run_saved_search();
    assert_eq!(app.view, View::EnvelopeList);
    assert_eq!(app.active_query.as_deref(), Some("subject quarterly"));
    assert!(app.search_all_folders);
}

#[test]
fn saving_without_a_search_explains_why() {
    let mut app = app();
    app.active_query = None;
    app.begin_save_search();
    assert_eq!(app.view, View::EnvelopeList, "no prompt opens");
    assert!(app.status_message.contains("Run a search"));
}

#[test]
fn an_empty_name_is_refused() {
    let mut app = app();
    app.active_query = Some("from alice".to_string());
    app.begin_save_search();
    app.saved_searches.name_input.clear();
    app.submit_save_search();
    assert!(app.status_message.contains("A name is required"));
    assert_eq!(app.view, View::SaveSearch, "the prompt stays open");
    assert!(store::list(app.db.as_ref().unwrap()).unwrap().is_empty());
}

#[test]
fn deleting_a_saved_search_requires_confirmation() {
    let mut app = app();
    store::save(
        app.db.as_ref().unwrap(),
        &SavedSearch {
            name: "Old".to_string(),
            query: "from bob".to_string(),
            all_folders: false,
        },
    )
    .unwrap();

    app.open_saved_searches();
    assert_eq!(app.saved_searches.searches.len(), 1);
    app.delete_saved_search();
    assert!(app.status_message.contains("Press d again"));
    assert_eq!(app.saved_searches.searches.len(), 1);

    app.delete_saved_search();
    assert!(app.saved_searches.searches.is_empty());
    assert!(store::list(app.db.as_ref().unwrap()).unwrap().is_empty());
}
