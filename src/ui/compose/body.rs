/*! Compose editor body and action bar. */

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::compose::{ComposeState, FocusedField, ACTION_BAR};
use crate::theme::theme;
use crate::ui::action_bar::{
    render_action_bar_with_label, Button, ICON_ATTACH, ICON_CHECK_OFF, ICON_CHECK_ON, ICON_DISCARD,
    ICON_DRAFT, ICON_FILES, ICON_LATER, ICON_SEND,
};

pub(super) fn render_body(state: &ComposeState, frame: &mut Frame, area: Rect) {
    let t = theme();
    let focused = state.focused == FocusedField::Body;

    let border_style = if focused {
        t.border_focused()
    } else {
        t.border()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(t.background));

    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);
    frame.render_widget(state.body.textarea(), inner);
}

// ── Action bar ────────────────────────────────────────────────────────────────

pub(super) fn render_compose_action_bar(state: &ComposeState, frame: &mut Frame, area: Rect) {
    let (status_label, status_style) = if state.body.is_search_active() {
        (
            format!(" FIND: {} ", state.body.search_query()),
            theme().mode_insert(),
        )
    } else if state.focused == FocusedField::Body {
        (" BODY ".to_string(), theme().mode_insert())
    } else {
        // Name the focused control: focus is otherwise shown only by colour,
        // which a reader who cannot distinguish it would never see.
        (
            format!(" COMPOSE · {} ", state.focused.label()),
            theme().mode_nav(),
        )
    };
    let status_label = if state.attachments.is_empty() {
        status_label
    } else {
        format!(
            "{} · {} attached ",
            status_label.trim_end(),
            state.attachments.len()
        )
    };

    let labels: Vec<String> = ACTION_BAR
        .iter()
        .map(|button| button_label(state, *button))
        .collect();
    let buttons: Vec<Button> = ACTION_BAR
        .iter()
        .zip(&labels)
        .map(|(button, label)| Button {
            label,
            focused: state.focused == *button,
            disabled: false,
        })
        .collect();

    render_action_bar_with_label(
        frame,
        area,
        &status_label,
        status_style,
        &buttons,
        state.focused.action_index(),
    );
}

/// Icon plus label for one action-bar button; toggles show their state.
fn button_label(state: &ComposeState, button: FocusedField) -> String {
    let icon = match button {
        FocusedField::Send => ICON_SEND,
        FocusedField::SendLater => ICON_LATER,
        FocusedField::Draft => ICON_DRAFT,
        FocusedField::Attach => ICON_ATTACH,
        FocusedField::Files => ICON_FILES,
        FocusedField::Sign => toggle_icon(state.sign),
        FocusedField::Encrypt => toggle_icon(state.encrypt),
        FocusedField::SmimeSign => toggle_icon(state.smime_sign),
        FocusedField::SmimeEncrypt => toggle_icon(state.smime_encrypt),
        _ => ICON_DISCARD,
    };
    format!("{icon} {}", button.label())
}

/// Checkbox glyph for a boolean toggle button.
fn toggle_icon(enabled: bool) -> &'static str {
    if enabled {
        ICON_CHECK_ON
    } else {
        ICON_CHECK_OFF
    }
}

// ── Discard confirmation overlay ─────────────────────────────────────────────
