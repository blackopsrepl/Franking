/*! Compose overlays: discard confirmation, autocomplete, and errors. */

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::compose::{ComposeState, FocusedField};
use crate::theme::theme;

pub(super) fn render_discard_confirm(frame: &mut Frame, area: Rect) {
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

pub(super) fn render_error(err: &str, frame: &mut Frame, area: Rect) {
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

pub(super) fn render_autocomplete(
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
        | FocusedField::Files
        | FocusedField::Sign
        | FocusedField::Encrypt
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

/// Prompt for a file path to attach.
pub(super) fn render_attach_prompt(input: &str, frame: &mut Frame, area: Rect) {
    let t = theme();
    use crate::ui::util::centered_rect;

    let popup = centered_rect(60, 3, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(Span::styled(" Attach file ", t.popup_title()))
        .borders(Borders::ALL)
        .border_style(t.border_focused())
        .style(t.popup());
    frame.render_widget(block.clone(), popup);
    let inner = block.inner(popup);
    frame.render_widget(
        Paragraph::new(format!(" {input}_")).style(t.normal()),
        inner,
    );
}

/// Overlay listing the attachments queued for this message.
pub(super) fn render_attach_list(state: &ComposeState, frame: &mut Frame, area: Rect) {
    let t = theme();
    use crate::ui::util::centered_rect;

    let popup = centered_rect(70, 40, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(Span::styled(
            " Attachments · Enter/d remove · Esc close ",
            t.popup_title(),
        ))
        .borders(Borders::ALL)
        .border_style(t.border_focused())
        .style(t.popup());
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let mut lines = Vec::new();
    for (index, path) in state.attachments.iter().enumerate() {
        let marker = if index == state.attach_index {
            "▸"
        } else {
            " "
        };
        let style = if index == state.attach_index {
            t.selected()
        } else {
            t.normal()
        };
        lines.push(Line::from(Span::styled(
            format!(" {marker} {path} "),
            style,
        )));
    }
    if lines.is_empty() {
        lines.push(Line::from(Span::styled(" (no attachments)", t.dimmed())));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}
