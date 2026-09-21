/* Identity add/edit form UI. */

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::App;
use crate::identity_edit::IdentityField;
use crate::keys::EditMode;
use crate::theme::theme;
use crate::ui::action_bar::{render_action_bar, Button, ICON_DISCARD, ICON_SAVE};

/// Render the identity add/edit form as a centered popup overlay.
pub fn render(app: &App, frame: &mut Frame) {
    let t = theme();
    let area = frame.area();

    let Some(ref state) = app.identity_edit_state else {
        return;
    };

    frame.render_widget(Clear, area);

    // Layout per field: 1 row label + 1 row input = 2 rows each.
    // Fields: Name, SenderName, Email, Signature, IsDefault = 5 × 2 rows.
    let popup_w = area.width.min(56);
    let popup_h = 18u16;
    let popup = Rect {
        x: area.x + area.width.saturating_sub(popup_w) / 2,
        y: area.y + area.height.saturating_sub(popup_h) / 2,
        width: popup_w,
        height: popup_h,
    };

    let title = if state.identity_id.is_some() {
        " Edit Identity "
    } else {
        " New Identity "
    };

    let block = Block::default()
        .title(Span::styled(title, t.popup_title()))
        .borders(Borders::ALL)
        .border_style(t.border_focused())
        .style(t.popup());
    frame.render_widget(block.clone(), popup);
    let inner = block.inner(popup);

    // Row 0: Account label (read-only)
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("Account: {}", state.account),
            t.dimmed(),
        )),
        Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: 1,
        },
    );

    // Rows 1–10: five fields (label + input each)
    let fields: [(IdentityField, &str, &str); 5] = [
        (IdentityField::Name, "Name   ", state.name.as_str()),
        (
            IdentityField::SenderName,
            "Sender ",
            state.display_name.as_str(),
        ),
        (IdentityField::Email, "Email  ", state.email.as_str()),
        (
            IdentityField::Signature,
            "Signat.",
            state.signature.as_str(),
        ),
        (
            IdentityField::IsDefault,
            "Default",
            if state.is_default {
                "[x] set as default"
            } else {
                "[ ] set as default"
            },
        ),
    ];

    for (i, (field, label, value)) in fields.iter().enumerate() {
        let row_y = inner.y + 1 + i as u16 * 2;
        let focused = state.focused == *field;

        // Label row
        let label_style = if focused {
            t.header_label().add_modifier(Modifier::BOLD)
        } else {
            t.header_label()
        };
        frame.render_widget(
            Paragraph::new(Span::styled(*label, label_style)),
            Rect {
                x: inner.x,
                y: row_y,
                width: inner.width,
                height: 1,
            },
        );

        // Input row — plain text with underline; no Block so text is always visible
        let cursor = if focused { "_" } else { "" };
        let display = format!("{}{}", value, cursor);
        let input_style = if focused {
            t.selected().add_modifier(Modifier::UNDERLINED)
        } else {
            t.normal().add_modifier(Modifier::UNDERLINED)
        };
        frame.render_widget(
            Paragraph::new(Span::styled(display, input_style)),
            Rect {
                x: inner.x,
                y: row_y + 1,
                width: inner.width,
                height: 1,
            },
        );
    }

    // Error bar (second-to-last row of inner) if present
    let action_bar_y = inner.y + inner.height.saturating_sub(1);
    let error_y = action_bar_y.saturating_sub(1);

    if let Some(ref err) = state.error {
        frame.render_widget(
            Paragraph::new(Span::styled(err.as_str(), t.error())).alignment(Alignment::Center),
            Rect {
                x: inner.x,
                y: error_y,
                width: inner.width,
                height: 1,
            },
        );
    }

    // ── Action bar (last row) ────────────────────────────────────────
    let action_area = Rect {
        x: inner.x,
        y: action_bar_y,
        width: inner.width,
        height: 1,
    };

    let focused_btn_idx = match state.focused {
        IdentityField::Save => Some(0usize),
        IdentityField::Cancel => Some(1),
        _ => None,
    };

    let buttons = [
        Button {
            label: &format!("{} Save", ICON_SAVE),
            focused: state.focused == IdentityField::Save,
            disabled: false,
        },
        Button {
            label: &format!("{} Cancel", ICON_DISCARD),
            focused: state.focused == IdentityField::Cancel,
            disabled: false,
        },
    ];

    render_action_bar(frame, action_area, EditMode::Nav, &buttons, focused_btn_idx);
}
