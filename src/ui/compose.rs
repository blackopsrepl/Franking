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
use crate::ui::action_bar::{
    render_action_bar_with_label, Button, ICON_ATTACH, ICON_DISCARD, ICON_DRAFT, ICON_SEND,
};

/// Render the full compose view.
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

fn render_body(state: &ComposeState, frame: &mut Frame, area: Rect) {
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

fn render_compose_action_bar(state: &ComposeState, frame: &mut Frame, area: Rect) {
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
            label: &format!("{} Attach (soon)", ICON_ATTACH),
            focused: state.focused == FocusedField::Attach,
            disabled: true,
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

fn render_discard_confirm(frame: &mut Frame, area: Rect) {
    let t = theme();
    use crate::ui::util::centered_rect;

    let popup = centered_rect(44, 5, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(Span::styled(" Discard message? ", t.popup_title()))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(t.error())
        .style(t.popup());
    frame.render_widget(block.clone(), popup);
    let inner = block.inner(popup);
    frame.render_widget(
        Paragraph::new("y  discard  ·  n / Esc  keep editing")
            .style(t.normal())
            .alignment(Alignment::Center),
        inner,
    );
}

// ── Error overlay ─────────────────────────────────────────────────────────────

fn render_error(err: &str, frame: &mut Frame, area: Rect) {
    let t = theme();
    use crate::ui::util::centered_rect;

    let popup = centered_rect(60, 5, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(Span::styled(" Send failed ", t.popup_title()))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(t.error())
        .style(t.popup());
    frame.render_widget(block.clone(), popup);
    let inner = block.inner(popup);
    frame.render_widget(
        Paragraph::new(err)
            .style(t.error())
            .alignment(Alignment::Center),
        inner,
    );
}

// ── Autocomplete popup ────────────────────────────────────────────────────────

fn render_autocomplete(
    ac: &crate::compose::AutocompleteState,
    focused: FocusedField,
    frame: &mut Frame,
    fields_area: Rect,
) {
    if ac.suggestions.is_empty() {
        return;
    }

    let t = theme();

    // Determine Y position: below the relevant field (0-indexed inside border)
    // Row 0 = From, Row 1 = To, Row 2 = Cc, Row 3 = Bcc, Row 4 = Subject
    let field_row = match focused {
        FocusedField::From
        | FocusedField::Subject
        | FocusedField::Body
        | FocusedField::Send
        | FocusedField::Draft
        | FocusedField::Attach
        | FocusedField::Discard => return,
        FocusedField::To => 1u16,
        FocusedField::Cc => 2,
        FocusedField::Bcc => 3,
    };

    let label_width: u16 = 9; // "  To:   " etc.
    let popup_width = fields_area.width.saturating_sub(label_width + 2);
    let max_items = ac.suggestions.len().min(6) as u16;
    let popup_rect = Rect {
        x: fields_area.x + label_width,
        y: fields_area.y + 1 + field_row, // below the field (inside the border)
        width: popup_width,
        height: max_items + 2,
    };

    frame.render_widget(Clear, popup_rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.border_focused())
        .style(t.popup());

    let items: Vec<ListItem> = ac
        .suggestions
        .iter()
        .enumerate()
        .map(|(i, (name, email))| {
            let text = match name {
                Some(n) if !n.is_empty() => format!("{} <{}>", n, email),
                _ => email.clone(),
            };
            let style = if i == ac.selected {
                t.selected()
            } else {
                t.normal()
            };
            ListItem::new(text).style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, popup_rect);
}
