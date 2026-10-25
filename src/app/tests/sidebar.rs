/*! Local-view sidebar rows use the same viewport for rendering and hit-testing. */

use crate::app::{App, LocalView};
use crate::keys::View;
use crate::mail::{
    store,
    types::{Envelope, Folder, FolderRole, Sender},
};
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{backend::TestBackend, Terminal};

fn app(folder_count: usize) -> App {
    let mut app = App::new(Some("personal".into()));
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    app.db = Some(conn);
    app.folders = (0..folder_count)
        .map(|i| Folder {
            name: if i == 0 {
                "INBOX".into()
            } else {
                format!("Folder {i}")
            },
            role: FolderRole::Other,
            desc: None,
            subscribed: None,
        })
        .collect();
    app
}

fn cache(app: &App, account: &str, folder: &str, id: &str, flagged: bool) -> Envelope {
    let envelope = Envelope {
        id: id.into(),
        flags: if flagged {
            vec!["flagged".into()]
        } else {
            vec![]
        },
        subject: format!("{account}/{folder}/{id}"),
        sender: Sender::Plain("alice@example.org".into()),
        date: "2026-09-23T12:00:00+00:00".into(),
        message_id: Some(format!("{account}-{folder}-{id}@example.org")),
        in_reply_to: None,
        account: Some(account.into()),
        folder: Some(folder.into()),
    };
    store::upsert_envelope(
        app.db.as_ref().unwrap(),
        &store::StoredMessage::from_envelope(account, folder, &envelope),
    )
    .unwrap();
    envelope
}

#[test]
fn local_views_are_visible_sidebar_rows_without_cycling() {
    let mut app = app(1);
    app.view = View::FolderList;
    app.move_selection(1);
    assert_eq!(app.folder_index, app.folders.len());
    app.select_item();
    assert_eq!(app.local_view, Some(LocalView::ReplyLater));
    app.open_reply_later();
    assert_eq!(app.local_view, Some(LocalView::ReplyLater));
    app.view = View::FolderList;
    app.move_selection(1);
    assert_eq!(app.folder_index, app.folders.len() + 1);
    app.select_item();
    assert_eq!(app.local_view, Some(LocalView::Starred));
    let mut terminal = Terminal::new(TestBackend::new(90, 12)).unwrap();
    terminal
        .draw(|frame| crate::ui::render(&mut app, frame))
        .unwrap();
    let screen: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(screen.contains("Reply later"));
    assert!(screen.contains("Starred (cached)"));
    assert!(!screen.contains("Saved"));
    app.open_starred();
    assert_eq!(
        app.local_view,
        Some(LocalView::Starred),
        "opening Starred again must not toggle to Inbox"
    );
}

#[test]
fn starred_uses_cached_server_flags_across_folders() {
    let mut app = app(1);
    cache(&app, "personal", "INBOX", "7", true);
    cache(&app, "personal", "Archive", "7", true);
    cache(&app, "work", "INBOX", "7", true);
    cache(&app, "personal", "INBOX", "8", false);
    app.open_starred();
    assert_eq!(app.envelopes.len(), 2);
    assert!(app
        .envelopes
        .iter()
        .all(|e| e.account.as_deref() == Some("personal") && e.is_flagged()));
    assert_ne!(app.envelopes[0].folder, app.envelopes[1].folder);
    assert!(app.status_message.contains("cached"));
    app.current_folder = "All Inboxes".into();
    app.open_starred();
    assert_eq!(app.envelopes.len(), 3);
    assert_eq!(app.selected.len(), 0);
    app.page_down();
    assert_eq!(app.page, 1);
}

#[test]
fn refreshed_maildir_identity_supersedes_legacy_flag_filenames() {
    let mut app = app(1);
    cache(&app, "personal", "INBOX", "entry:2,F", true);
    cache(&app, "personal", "INBOX", "entry", true);
    app.open_starred();
    assert_eq!(app.envelopes.len(), 1);
    assert_eq!(app.envelopes[0].id, "entry");
    cache(&app, "personal", "INBOX", "entry", false);
    app.open_starred();
    assert!(
        app.envelopes.is_empty(),
        "old cached flags must not create ghost stars"
    );
}

#[test]
fn mouse_uses_rendered_sidebar_scroll_offset_and_ignores_borders() {
    let mut app = app(20);
    app.view = View::FolderList;
    app.folder_index = app.folders.len() + 1;
    let mut terminal = Terminal::new(TestBackend::new(90, 10)).unwrap();
    terminal
        .draw(|frame| crate::ui::render(&mut app, frame))
        .unwrap();
    assert_eq!(app.sidebar_offset(6), 16);
    let click = |column, row| MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    };
    app.handle_mouse(click(5, 7));
    assert_eq!(app.local_view, Some(LocalView::Starred));
    terminal
        .draw(|frame| crate::ui::render(&mut app, frame))
        .unwrap();
    let reply_row = (0..10)
        .find(|row| {
            let text: String = (0..22)
                .map(|col| terminal.backend().buffer()[(col, *row)].symbol())
                .collect();
            text.contains("Reply later")
        })
        .expect("Reply later must be visible in the rendered sidebar");
    app.handle_mouse(click(5, reply_row));
    assert_eq!(app.local_view, Some(LocalView::ReplyLater));
    app.handle_mouse(click(0, 2));
    assert_eq!(app.local_view, Some(LocalView::ReplyLater));
    app.view = View::FolderList;
    app.folder_index = 0;
    app.select_item();
    assert!(app.local_view.is_none());
}
