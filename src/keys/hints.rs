/*! Status-bar keybinding hints per view. */

use super::action::View;

pub fn hints(view: View) -> Vec<(&'static str, &'static str)> {
    match view {
        View::EnvelopeList => vec![
            ("j/k", "nav"),
            ("Enter", "read"),
            ("c", "compose"),
            ("d", "del"),
            ("m", "move"),
            ("!", "flag"),
            ("N", "read"),
            ("S", "sync"),
            ("A", "all read"),
            ("t", "thread"),
            ("/", "search"),
            ("Tab", "folders"),
            ("Ctrl+b", "contacts"),
            ("?", "help"),
        ],
        View::MessageView => vec![
            ("j/k", "scroll"),
            ("r", "reply"),
            ("R", "all"),
            ("f", "fwd"),
            ("d", "del"),
            ("a", "attach"),
            ("N", "read"),
            ("P", "unlock"),
            ("T", "trust"),
            ("q", "back"),
            ("?", "help"),
        ],
        View::FolderList => vec![
            ("j/k", "nav"),
            ("Enter", "select"),
            ("Tab", "emails"),
            ("?", "help"),
        ],
        View::AccountList => vec![("j/k", "nav"), ("Enter", "select"), ("Esc", "cancel")],
        View::Search => vec![("Enter", "search"), ("Esc", "cancel")],
        View::Help => vec![("j/k", "scroll"), ("q/?/Esc", "close")],
        View::MovePrompt => vec![("Enter", "move"), ("Esc", "cancel")],
        View::PassphrasePrompt => vec![("Enter", "unlock"), ("Esc", "cancel")],
        View::Compose => vec![
            ("Tab/j/k", "nav"),
            ("Enter", "insert"),
            ("Esc", "nav/discard"),
        ],
        View::Contacts => vec![
            ("j/k", "nav"),
            ("n", "new"),
            ("e", "edit"),
            ("d", "del"),
            ("/", "search"),
            ("q", "close"),
        ],
        View::ContactSearch => vec![("Enter", "confirm"), ("Esc", "cancel")],
        View::ContactEdit => vec![
            ("Tab/Enter", "next field"),
            ("Tab→Save→Enter", "save"),
            ("Esc", "cancel"),
        ],
        View::IdentityList => vec![
            ("j/k", "nav"),
            ("n", "new"),
            ("e", "edit"),
            ("d", "delete"),
            ("s", "set default"),
            ("q/Esc", "close"),
        ],
        View::IdentityEdit => vec![
            ("Tab/Enter", "next field"),
            ("Space", "toggle"),
            ("Tab→Save→Enter", "save"),
            ("Esc", "cancel"),
        ],
    }
}
