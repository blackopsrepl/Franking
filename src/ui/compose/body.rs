/*! Compose editor body and action bar. */

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::compose::{ComposeState, FocusedField};
use crate::theme::theme;
use crate::ui::action_bar::{
    render_action_bar_with_label, Button, ICON_ATTACH, ICON_DISCARD, ICON_DRAFT, ICON_SEND,
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
        (" COMPOSE ".to_string(), theme().mode_nav())
    };

    // Determine which button index is focused (None when a non-button field is focused).
    // Buttons order: Send(0) Draft(1) Attach(2) Discard(3)
    let focused_btn_idx = match state.focused {
        FocusedField::Send => Some(0usize),
        FocusedField::Draft => Some(1),
        FocusedField::Attach => Some(2),
        FocusedField::Discard => Some(3),
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

// ── Discard confirmation overlay ─────────────────────────────────────────────
