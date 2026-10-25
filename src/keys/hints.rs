/*! Status-bar keybinding hints per view. */

use super::view::View;

pub fn hints(view: View) -> Vec<(&'static str, &'static str)> {
    if matches!(
        view,
        View::EnvelopeList | View::MessageView | View::FolderList
    ) {
        return super::commands::commands(view)
            .into_iter()
            .filter_map(|command| command.hint.map(|hint| (command.shortcut, hint)))
            .collect();
    }
    let pairs: &[(&'static str, &'static str)] = match view {
        View::EnvelopeList | View::MessageView | View::FolderList => unreachable!(),
        View::FolderPrompt => &[("Enter", "confirm"), ("Esc", "cancel")],
        View::MessageSearch => &[("Enter", "find"), ("Esc", "cancel")],
        View::LinkList => &[("j/k", "nav"), ("Enter", "open"), ("Esc", "close")],
        View::SieveScripts => &[
            ("j/k", "nav"),
            ("Enter", "activate"),
            ("e", "edit"),
            ("n", "new"),
            ("r", "rename"),
            ("d", "delete"),
            ("x", "none active"),
            ("Esc", "close"),
        ],
        View::SieveName => &[("Enter", "create"), ("Esc", "cancel")],
        View::Keys => &[
            ("j/k", "nav"),
            ("i", "import"),
            ("g", "generate"),
            ("x", "export"),
            ("d", "delete"),
            ("Esc", "close"),
        ],
        View::KeysPrompt => &[("Enter", "confirm"), ("Esc", "cancel")],
        View::SaveSearch => &[("Enter", "save"), ("Esc", "cancel")],
        View::SavedSearches => &[
            ("j/k", "nav"),
            ("Enter", "run"),
            ("d", "delete"),
            ("Esc", "close"),
        ],
        View::SieveEdit => &[("Ctrl+s", "save"), ("Esc", "back")],
        View::Settings => &[("space", "toggle"), ("Esc", "close")],
        View::SchedulePrompt => &[("Enter", "schedule"), ("Esc", "cancel")],
        View::ResurfacePrompt => &[("Enter", "resurface"), ("Esc", "cancel")],
        View::PlacePrompt => &[("1-5", "place"), ("Esc", "cancel")],
        View::MessageNote => &[("Enter", "save"), ("Esc", "cancel")],
        View::SubjectAlias => &[("Enter", "rename"), ("Esc", "cancel")],
        View::AttachmentLibrary => &[("j/k", "nav"), ("Enter", "open message"), ("Esc", "close")],
        View::ReadTogether => &[("j/k", "scroll"), ("g/G", "top/end"), ("q/Esc", "back")],
        View::BypassPrompt => &[("Enter", "save"), ("Tab", "generate"), ("Esc", "cancel")],
        View::Clips => &[
            ("j/k", "nav"),
            ("Enter", "copy"),
            ("d", "delete"),
            ("Esc", "close"),
        ],
        View::ClipPrompt => &[("Enter", "save"), ("Esc", "cancel")],
        View::CollectionBoard => &[
            ("j/k", "nav"),
            ("Enter", "add/remove"),
            ("f", "filter"),
            ("n", "new"),
            ("Esc", "close"),
        ],
        View::CollectionName => &[("Enter", "save"), ("Esc", "cancel")],
        View::StageBoard => &[
            ("j/k", "nav"),
            ("Enter", "assign"),
            ("f", "filter"),
            ("n", "new"),
            ("u", "unstage"),
            ("Esc", "close"),
        ],
        View::StageName => &[("Enter", "save"), ("Esc", "cancel")],
        View::Snippets => &[
            ("j/k", "nav"),
            ("Enter", "insert"),
            ("s", "save body"),
            ("d", "delete"),
            ("Esc", "close"),
        ],
        View::SnippetName => &[("Enter", "save"), ("Esc", "cancel")],
        View::FocusReply => &[
            ("n/p", "next/prev"),
            ("j/k", "scroll"),
            ("r/R", "reply/all"),
            ("d", "done"),
            ("q/Esc", "close"),
        ],
        View::Outbox => &[
            ("j/k", "nav"),
            ("Enter", "send now"),
            ("d", "discard"),
            ("Esc", "close"),
        ],
        View::FilePicker => &[
            ("Enter", "open/attach"),
            ("Backspace", "up"),
            ("Esc", "cancel"),
        ],
        View::AccountEdit => &[
            ("Tab", "field"),
            ("Ctrl+d", "detect"),
            ("Enter", "save"),
            ("Esc", "cancel"),
        ],
        View::AccountList => &[
            ("j/k", "nav"),
            ("Enter", "select"),
            ("s", "default"),
            ("d", "delete"),
            ("Esc", "cancel"),
        ],
        View::Search => &[
            ("Enter", "search"),
            ("Tab", "all folders"),
            ("Esc", "cancel"),
        ],
        View::Help => &[("j/k", "scroll"), ("g/G", "top/end"), ("q/?/Esc", "close")],
        View::MovePrompt => &[("Enter", "move"), ("Esc", "cancel")],
        View::PassphrasePrompt => &[("Enter", "unlock"), ("Esc", "cancel")],
        View::AttachmentView => &[("j/k", "scroll"), ("Esc", "close")],
        View::AttachmentList => &[
            ("j/k", "nav"),
            ("Enter", "open"),
            ("v", "view"),
            ("s", "save"),
            ("S", "save to…"),
            ("Esc", "close"),
        ],
        View::Compose => &[
            ("Tab/j/k", "nav"),
            ("Enter", "insert"),
            ("Esc", "nav/discard"),
        ],
        View::Contacts => &[
            ("t", "tag filter"),
            ("j/k", "nav"),
            ("n", "new"),
            ("e", "edit"),
            ("d", "del"),
            ("/", "search"),
            ("q", "close"),
        ],
        View::ContactSearch => &[("Enter", "confirm"), ("Esc", "cancel")],
        View::ContactEdit => &[
            ("Tab/Enter", "next field"),
            ("Tab→Save→Enter", "save"),
            ("Esc", "cancel"),
        ],
        View::IdentityList => &[
            ("j/k", "nav"),
            ("n", "new"),
            ("e", "edit"),
            ("d", "delete"),
            ("s", "set default"),
            ("q/Esc", "close"),
        ],
        View::IdentityEdit => &[
            ("Tab/Enter", "next field"),
            ("Space", "toggle"),
            ("Tab→Save→Enter", "save"),
            ("Esc", "cancel"),
        ],
    };
    pairs.to_vec()
}
