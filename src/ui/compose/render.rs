/* Compose editor UI — the spiffy TUI email compose screen.
Layout:
┌─────────────────────────────────────────────────────┐
│  ✉ Compose · new                        [account]  │  header bar
├───────────────────────────────────────────────────  │
│ To:      │ <input field>                            │
│ Cc:      │ <input field>                            │
│ Bcc:     │ <input field>                            │
│ Subject: │ <input field>                            │
├──────────────────────────────────────────────────── │
│                                                      │  compose body
│   (email-first multiline editor)                     │
│                                                      │
├─────────────────────────────────────────────────────│
│ -- COMPOSE --   Ctrl+f: find   Tab: next field   │  status bar
└─────────────────────────────────────────────────────┘ */

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::App;
use crate::compose::{ComposeMode, ComposeState, FocusedField};
use crate::theme::theme;

/// Render the full compose view.
use super::body::{render_body, render_compose_action_bar};
use super::overlays::{
    render_attach_prompt, render_autocomplete, render_discard_confirm, render_error,
};

pub fn render(app: &App, frame: &mut Frame) {
    let state = match &app.compose_state {
        Some(s) => s,
        None => return,
    };

    let area = frame.area();

    // ── Top-level layout: header(1) + fields(7) + body(fill) + status(1)
    let outer = Layout::vertical([
        Constraint::Length(1), // header bar
        Constraint::Length(7), // header fields (From/To/Cc/Bcc/Subject + borders)
        Constraint::Fill(1),   // editor body
        Constraint::Length(1), // status / compose bar
    ])
    .split(area);

    render_header_bar(app, state, frame, outer[0]);
    render_header_fields(state, frame, outer[1]);
    render_body(state, frame, outer[2]);
    render_compose_action_bar(state, frame, outer[3]);

    // ── Overlays ─────────────────────────────────────────────────────
    if state.confirm_discard {
        render_discard_confirm(frame, area);
    }
    if let Some(ac) = &state.autocomplete {
        render_autocomplete(ac, state.focused, frame, outer[1]);
    }
    if let Some(err) = &state.send_error {
        render_error(err, frame, area);
    }
    if let Some(input) = &state.attach_input {
        render_attach_prompt(input, frame, area);
    }
}

// ── Header bar ───────────────────────────────────────────────────────────────

fn render_header_bar(app: &App, state: &ComposeState, frame: &mut Frame, area: Rect) {
    let t = theme();

    let mode_label = match state.mode {
        ComposeMode::New => "new",
        ComposeMode::Reply => "reply",
        ComposeMode::ReplyAll => "reply-all",
        ComposeMode::Forward => "forward",
    };

    let account_label = state
        .account
        .as_deref()
        .or(app.account_name.as_deref())
        .unwrap_or("?");

    let title = format!(" \u{2709} compose \u{00B7} {mode_label} ");
    let acct = format!(" [{account_label}] ");
    let fill_len = area
        .width
        .saturating_sub(title.chars().count() as u16 + acct.chars().count() as u16)
        as usize;

    let spans = vec![
        Span::styled(title, t.header()),
        Span::styled(" ".repeat(fill_len), t.header()),
        Span::styled(acct, t.header().add_modifier(Modifier::BOLD)),
    ];

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

// ── Header fields ─────────────────────────────────────────────────────────────

fn render_header_fields(state: &ComposeState, frame: &mut Frame, area: Rect) {
    let t = theme();

    let border_style = if state.is_header_focused() {
        t.border_focused()
    } else {
        t.border()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(t.popup());
    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);

    // 5 rows: From / To / Cc / Bcc / Subject
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(inner);

    render_from_field(state, frame, rows[0]);
    render_field(state, frame, rows[1], FocusedField::To, &state.to);
    render_field(state, frame, rows[2], FocusedField::Cc, &state.cc);
    render_field(state, frame, rows[3], FocusedField::Bcc, &state.bcc);
    render_field(state, frame, rows[4], FocusedField::Subject, &state.subject);
}

fn render_from_field(state: &ComposeState, frame: &mut Frame, area: Rect) {
    let t = theme();
    let focused = state.focused == FocusedField::From;

    let label = "   From: ";
    let cols = Layout::horizontal([Constraint::Length(label.len() as u16), Constraint::Fill(1)])
        .split(area);

    frame.render_widget(Paragraph::new(label).style(t.header_label()), cols[0]);

    // Build value text: selected identity label, or "(account default)"
    let value = match state.selected_identity() {
        Some(id) => id.label(),
        None => {
            if state.from_identities.is_empty() {
                "(no identities configured)".to_string()
            } else {
                "(account default)".to_string()
            }
        }
    };

    // When focused and identities exist, show a cycle indicator: [2/3]
    let suffix = if focused && !state.from_identities.is_empty() {
        let current = state.from_idx.map(|i| i + 1).unwrap_or(0);
        let total = state.from_identities.len();
        format!("  [{}/{}]  ←/→ cycle", current, total)
    } else {
        String::new()
    };

    let display = format!("{value}{suffix}");
    let input_style = if focused {
        Style::default()
            .fg(t.foreground)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(t.foreground)
    };
    frame.render_widget(Paragraph::new(display).style(input_style), cols[1]);
}

fn render_field(
    state: &ComposeState,
    frame: &mut Frame,
    area: Rect,
    field: FocusedField,
    value: &str,
) {
    let t = theme();
    let focused = state.focused == field;
    let label = format!("{:>7}: ", field.label());

    // Split the area: label on the left, input on the right
    let cols = Layout::horizontal([Constraint::Length(label.len() as u16), Constraint::Fill(1)])
        .split(area);

    frame.render_widget(Paragraph::new(label).style(t.header_label()), cols[0]);

    let cursor = if focused { "\u{258c}" } else { "" }; // blinking block cursor
    let display = format!("{value}{cursor}");
    let input_style = if focused {
        Style::default()
            .fg(t.foreground)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(t.foreground)
    };
    frame.render_widget(Paragraph::new(display).style(input_style), cols[1]);
}

// ── Body editor ──────────────────────────────────────────────────────────────
