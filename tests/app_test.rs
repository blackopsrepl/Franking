use franking::keys::View;

// These tests cover the state machine logic directly without invoking
// network or backend I/O.
// We construct an App and manipulate its state directly.

use franking::app::App;

#[test]
fn app_initial_state() {
    let app = App::new(None);
    assert!(app.running);
    assert_eq!(app.view, View::EnvelopeList);
    assert_eq!(app.current_folder, "INBOX");
    assert_eq!(app.page, 1);
    assert!(app.envelopes.is_empty());
    assert!(app.folders.is_empty());
    assert!(app.accounts.is_empty());
    assert!(app.pending_open_command.is_none());
}

#[test]
fn app_with_account_flag() {
    let app = App::new(Some("work".to_string()));
    assert_eq!(app.account_name, Some("work".to_string()));
}

#[test]
fn app_set_status() {
    let mut app = App::new(None);
    app.set_status("test message");
    assert_eq!(app.status_message, "test message");
}

#[test]
fn app_tick_increments_counter() {
    let mut app = App::new(None);
    assert_eq!(app.tick_count, 0);
    app.tick();
    assert_eq!(app.tick_count, 1);
    app.tick();
    assert_eq!(app.tick_count, 2);
}

#[test]
fn app_search_query_manipulation() {
    let mut app = App::new(None);
    app.search_query.push('f');
    app.search_query.push('o');
    app.search_query.push('o');
    assert_eq!(app.search_query, "foo");
    app.search_query.pop();
    assert_eq!(app.search_query, "fo");
}

#[test]
fn app_move_target_manipulation() {
    let mut app = App::new(None);
    app.move_target.push('S');
    app.move_target.push('e');
    app.move_target.push('n');
    app.move_target.push('t');
    assert_eq!(app.move_target, "Sent");
}

#[test]
fn app_selected_envelope_empty() {
    let app = App::new(None);
    assert!(app.selected_envelope_id().is_none());
    assert!(app.selected_envelope().is_none());
}

#[test]
fn app_help_scroll() {
    let mut app = App::new(None);
    assert_eq!(app.help_scroll, 0);
    app.help_scroll = 5;
    assert_eq!(app.help_scroll, 5);
}
