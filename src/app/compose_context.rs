/*! Build contextual compose focus for the key resolver. */

use crate::compose::FocusedField;
use crate::keys::{ComposeFocus, ComposeKeyContext, EditMode};

use super::model::App;

impl App {
    pub(crate) fn compose_key_context(&self) -> ComposeKeyContext {
        self.compose_state
            .as_ref()
            .map(|cs| ComposeKeyContext {
                focus: match cs.focused {
                    FocusedField::From => ComposeFocus::From,
                    FocusedField::To
                    | FocusedField::Cc
                    | FocusedField::Bcc
                    | FocusedField::Subject => ComposeFocus::Header,
                    FocusedField::Body => ComposeFocus::Body,
                    FocusedField::Send
                    | FocusedField::Draft
                    | FocusedField::Attach
                    | FocusedField::Files
                    | FocusedField::Sign
                    | FocusedField::Encrypt
                    | FocusedField::SmimeSign
                    | FocusedField::SmimeEncrypt
                    | FocusedField::Discard => ComposeFocus::ActionBar,
                },
                edit_mode: cs.edit_mode,
                body_search_active: cs.body.is_search_active(),
                autocomplete_visible: cs.autocomplete.is_some(),
                confirm_discard_visible: cs.confirm_discard,
            })
            .unwrap_or(ComposeKeyContext {
                focus: ComposeFocus::Header,
                edit_mode: EditMode::Nav,
                body_search_active: false,
                autocomplete_visible: false,
                confirm_discard_visible: false,
            })
    }
}
