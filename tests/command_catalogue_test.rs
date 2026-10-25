/*! The command catalogue must agree with the actual input router. */

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use franking::keys::{commands::commands, resolve, Action, View};

#[test]
fn every_advertised_shortcut_resolves_without_collisions() {
    for view in [View::EnvelopeList, View::MessageView, View::FolderList] {
        let rows = commands(view);
        let mut keys = Vec::new();
        for row in rows {
            if let Some(key) = row.key {
                assert!(!keys.contains(&key), "{view:?}: {key:?}");
                keys.push(key);
                assert_eq!(
                    resolve(view, KeyEvent::new(key, KeyModifiers::NONE)),
                    row.action
                );
            }
        }
    }
}

#[test]
fn digits_are_unbound_and_colon_opens_actions() {
    for view in [View::EnvelopeList, View::MessageView, View::FolderList] {
        for digit in '1'..='5' {
            assert_eq!(
                resolve(
                    view,
                    KeyEvent::new(KeyCode::Char(digit), KeyModifiers::NONE)
                ),
                Action::None
            );
        }
        assert_eq!(
            resolve(view, KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE)),
            Action::OpenActions
        );
    }
}
