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
fn retired_workflows_have_no_shortcut_or_action_menu_entry() {
    for view in [View::EnvelopeList, View::MessageView, View::FolderList] {
        let rows = commands(view);
        for row in rows {
            assert!(!matches!(
                row.action,
                Action::CycleTriageLane
                    | Action::RouteSender(_)
                    | Action::RouteMessage(_)
                    | Action::OpenStages
                    | Action::OpenCollections
                    | Action::MergeConversation
                    | Action::OpenClips
                    | Action::OpenClipPrompt
                    | Action::OpenSubjectAlias
                    | Action::CycleBundle
                    | Action::OpenReadTogether
                    | Action::OpenFocusReply
                    | Action::OpenResurface
                    | Action::ToggleLoudConversation
                    | Action::ToggleCoverReveal
                    | Action::ToggleMarker(franking::db::message_markers::Marker::Saved)
                    | Action::OpenFollowup(franking::db::message_markers::Marker::Saved)
            ));
        }
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
