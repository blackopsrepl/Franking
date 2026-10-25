/* Action bar widget — a row of labelled buttons with a mode pill on the left.
When the row is wider than the terminal it scrolls to keep the focused button
visible and marks the elided side.
Used by the compose, identity-edit and contact-edit screens. */

use ratatui::prelude::*;
use unicode_width::UnicodeWidthStr;

use crate::keys::EditMode;
use crate::theme::theme;

// ── Nerd Font icons ──────────────────────────────────────────────────────────
/// Paper plane — Send
pub const ICON_SEND: &str = "\u{f1d8}";
/// Clock — Send later
pub const ICON_LATER: &str = "\u{f017}";
/// Pencil — Draft
pub const ICON_DRAFT: &str = "\u{f044}";
/// Paperclip — Attach
pub const ICON_ATTACH: &str = "\u{f0c6}";
/// × — Discard / Cancel
pub const ICON_DISCARD: &str = "\u{f00d}";
/// Floppy disk — Save
pub const ICON_SAVE: &str = "\u{f0c7}";
/// Folder — attachment list
pub const ICON_FILES: &str = "\u{f07b}";
/// Checked box — toggled on
pub const ICON_CHECK_ON: &str = "\u{f046}";
/// Empty box — toggled off
pub const ICON_CHECK_OFF: &str = "\u{f096}";

// ── Button descriptors ───────────────────────────────────────────────────────

/// A single action-bar button.
pub struct Button<'a> {
    /// Display label (including icon).
    pub label: &'a str,
    /// Whether this button is currently focused.
    pub focused: bool,
    /// Whether this button is disabled (grayed out, italic, no interaction).
    pub disabled: bool,
}

/// Render an action bar with a mode pill and a set of buttons.
///
/// Layout (left → right):
///   `[ -- NAV -- │  Send ·  Draft ·  Attach ·  Discard ]`
///
/// `area`         — the 1-row area to render into.
/// `edit_mode`    — Nav or Insert (used for the mode pill).
/// `buttons`      — ordered list of buttons.
/// `focused_idx`  — index of the currently focused button, kept on screen.
pub fn render_action_bar(
    frame: &mut Frame,
    area: Rect,
    edit_mode: EditMode,
    buttons: &[Button<'_>],
    focused_idx: Option<usize>,
) {
    let t = theme();

    // ── Mode pill ────────────────────────────────────────────────────
    let (mode_text, mode_style) = match edit_mode {
        EditMode::Nav => (" -- NAV -- ", t.mode_nav()),
        EditMode::Insert => (" -- INSERT -- ", t.mode_insert()),
    };
    render_action_bar_with_label(frame, area, mode_text, mode_style, buttons, focused_idx);
}

pub fn render_action_bar_with_label(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    label_style: Style,
    buttons: &[Button<'_>],
    focused_idx: Option<usize>,
) {
    let t = theme();
    let pill_width = label.chars().count() as u16;
    let sep_width = 1u16; // │ separator

    // ── Split area: pill | separator | tabs ──────────────────────────
    let cols = Layout::horizontal([
        Constraint::Length(pill_width),
        Constraint::Length(sep_width),
        Constraint::Fill(1),
    ])
    .split(area);

    // Mode pill
    frame.render_widget(
        ratatui::widgets::Paragraph::new(label).style(label_style),
        cols[0],
    );

    // Separator
    frame.render_widget(
        ratatui::widgets::Paragraph::new("│").style(t.dimmed()),
        cols[1],
    );

    // ── Buttons ──────────────────────────────────────────────────────
    let labels: Vec<String> = buttons
        .iter()
        .map(|btn| format!(" {} ", btn.label))
        .collect();
    let widths: Vec<usize> = labels.iter().map(|label| label.width()).collect();
    let (start, end) = visible_window(&widths, focused_idx, cols[2].width as usize);
    let mut spans = Vec::new();
    if start > 0 {
        spans.push(Span::styled(MORE_LEFT, t.dimmed()));
    }
    for index in start..end {
        if index > start {
            spans.push(Span::styled(DIVIDER, t.dimmed()));
        }
        let btn = &buttons[index];
        let style = if btn.disabled {
            t.action_btn_disabled()
        } else if btn.focused || focused_idx == Some(index) {
            t.action_btn_focused()
        } else {
            t.action_btn()
        };
        spans.push(Span::styled(labels[index].clone(), style));
    }
    if end < buttons.len() {
        spans.push(Span::styled(MORE_RIGHT, t.dimmed()));
    }
    frame.render_widget(
        ratatui::widgets::Paragraph::new(Line::from(spans)).style(t.status_bar()),
        cols[2],
    );
}

const DIVIDER: &str = " · ";
const MORE_LEFT: &str = "‹ ";
const MORE_RIGHT: &str = " ›";

/// The contiguous run of buttons `[start, end)` that fits `available` columns
/// and always contains the focused button. Elided sides get a ‹ or › marker.
fn visible_window(widths: &[usize], focused: Option<usize>, available: usize) -> (usize, usize) {
    let span = |start: usize, end: usize| -> usize {
        let buttons: usize = widths[start..end].iter().sum();
        let dividers = (end - start).saturating_sub(1) * DIVIDER.width();
        let markers = if start > 0 { MORE_LEFT.width() } else { 0 }
            + if end < widths.len() {
                MORE_RIGHT.width()
            } else {
                0
            };
        buttons + dividers + markers
    };
    if span(0, widths.len()) <= available {
        return (0, widths.len());
    }
    let anchor = focused.unwrap_or(0).min(widths.len().saturating_sub(1));
    let mut start = 0;
    while start < anchor && span(start, anchor + 1) > available {
        start += 1;
    }
    let mut end = anchor + 1;
    while end < widths.len() && span(start, end + 1) <= available {
        end += 1;
    }
    (start, end.max(start + 1).min(widths.len()))
}
