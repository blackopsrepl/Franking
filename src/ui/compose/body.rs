/*! Compose editor body and action bar. */

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::compose::{ComposeState, FocusedField};
use crate::theme::theme;
use crate::ui::action_bar::{
    render_action_bar_with_label, Button, ICON_ATTACH, ICON_CHECK_OFF, ICON_CHECK_ON, ICON_DISCARD,
    ICON_DRAFT, ICON_FILES, ICON_SEND,
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

    // Determine which button index is focused (None when a non-button field is focused).
    // Buttons order: Send(0) Draft(1) Attach(2) Files(3) Sign(4) Encrypt(5) Discard(6)
    let focused_btn_idx = match state.focused {
        FocusedField::Send => Some(0usize),
        FocusedField::Draft => Some(1),
        FocusedField::Attach => Some(2),
        FocusedField::Files => Some(3),
        FocusedField::Sign => Some(4),
        FocusedField::Encrypt => Some(5),
        FocusedField::Discard => Some(6),
        _ => None,
    };

    let buttons = [
        Button {
            label: &format!("{} Send", ICON_SEND),
            focused: state.focused == FocusedField::Send,
            disabled: false,
        },
        Button {
            label: &format!("{} Draft", ICON_DRAFT),
            focused: state.focused == FocusedField::Draft,
            disabled: false,
        },
        Button {
            label: &format!("{} Attach", ICON_ATTACH),
            focused: state.focused == FocusedField::Attach,
            disabled: false,
        },
        Button {
            label: &format!("{} Files", ICON_FILES),
            focused: state.focused == FocusedField::Files,
            disabled: false,
        },
        Button {
            label: &format!("{} Sign", toggle_icon(state.sign)),
            focused: state.focused == FocusedField::Sign,
            disabled: false,
        },
        Button {
            label: &format!("{} Encrypt", toggle_icon(state.encrypt)),
            focused: state.focused == FocusedField::Encrypt,
            disabled: false,
        },
        Button {
            label: &format!("{} Discard", ICON_DISCARD),
            focused: state.focused == FocusedField::Discard,
            disabled: false,
        },
    ];

    render_action_bar_with_label(
        frame,
        area,
        &status_label,
        status_style,
        &buttons,
        focused_btn_idx,
    );
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
