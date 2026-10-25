/*! Exercise typed Actions and the real renderer, not only the resolver. */

use crate::{app::App, keys::View};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};

fn key(app: &mut App, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
}

fn type_text(app: &mut App, text: &str) {
    for c in text.chars() {
        key(app, KeyCode::Char(c));
    }
}

fn screen(app: &mut App, width: u16, height: u16) -> Vec<String> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| crate::ui::render(app, frame))
        .unwrap();
    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| (0..width).map(|x| buffer[(x, y)].symbol()).collect())
        .collect()
}

#[test]
fn actions_filter_execute_and_escape_without_losing_reader() {
    let mut app = App::new(None);
    app.view = View::MessageView;
    app.message_content =
        Some(crate::mail::mime::parse_message(b"Subject: hello\r\n\r\nbody").unwrap());
    type_text(&mut app, ":headers");
    let rows = screen(&mut app, 80, 24);
    assert!(rows.iter().any(|row| row.contains("Show all headers")));
    key(&mut app, KeyCode::Enter);
    assert!(app.show_all_headers);
    assert!(app.actions_menu.is_none());
    assert_eq!(app.view, View::MessageView);
    assert!(app.message_content.is_some());
    type_text(&mut app, ":no such action");
    assert!(screen(&mut app, 80, 24)
        .iter()
        .any(|row| row.contains("No matching actions")));
    key(&mut app, KeyCode::Esc);
    assert_eq!(app.view, View::MessageView);
    assert!(app.actions_menu.is_none());
}

#[test]
fn footer_keeps_primary_actions_and_discovery_at_normal_widths() {
    for width in [80, 100, 120] {
        let mut app = App::new(None);
        app.set_status("Mail refreshed.");
        let rows = screen(&mut app, width, 24);
        let footer = rows.last().unwrap();
        for text in [
            "Enter Read",
            "c Compose",
            "e Archive",
            "/ Search",
            ": Actions",
            "? Help",
        ] {
            assert!(footer.contains(text), "{width}: {footer}");
        }
        app.view = View::MessageView;
        let rows = screen(&mut app, width, 24);
        let footer = rows.last().unwrap();
        for text in [
            "r Reply",
            "R Reply all",
            "e Archive",
            "Esc Back",
            ": Actions",
        ] {
            assert!(footer.contains(text), "{width}: {footer}");
        }
    }
}

#[test]
fn action_menu_does_not_capture_compose_text() {
    let mut app = App::new(None);
    app.view = View::Compose;
    type_text(&mut app, ":");
    assert!(app.actions_menu.is_none());
}

#[test]
fn local_views_do_not_offer_server_folder_mutations() {
    let mut app = App::new(None);
    app.local_view = Some(crate::app::LocalView::Starred);
    type_text(&mut app, ":mark folder");
    assert!(app.actions_menu.as_ref().unwrap().matches().is_empty());
}

#[test]
fn folder_actions_capture_the_highlighted_folder_not_the_open_mailbox() {
    use crate::mail::types::{Folder, FolderRole};
    let mut app = App::new(None);
    app.folders = vec![Folder {
        name: "Archive".into(),
        desc: None,
        role: FolderRole::Archive,
        subscribed: None,
    }];
    app.view = View::FolderList;
    type_text(&mut app, ":rename folder");
    key(&mut app, KeyCode::Enter);
    let prompt = app.folder_prompt.as_ref().unwrap();
    assert_eq!(prompt.source, "Archive");
    assert_eq!(prompt.input, "Archive");
    assert_eq!(app.current_folder, "INBOX");
}

#[test]
fn settings_display_order_matches_toggle_targets() {
    let mut app = App::new(None);
    app.open_settings();
    let before = app.page_size;
    app.settings_index = 2;
    let rows = screen(&mut app, 100, 30);
    assert!(rows.iter().any(|row| row.contains("▸ Page size")));
    app.settings_toggle();
    assert_ne!(app.page_size, before);
    assert!(!rows
        .iter()
        .any(|row| row.contains("Bypass token") || row.contains("Cover previously")));
}
