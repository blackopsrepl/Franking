/* Action bar widget — a row of labelled buttons rendered with ratatui's `Tabs`
widget, with a mode pill on the left.
Used by the compose, identity-edit and contact-edit screens. */

use ratatui::prelude::*;
use ratatui::widgets::Tabs;

use crate::keys::EditMode;
use crate::theme::theme;

// ── Nerd Font icons ──────────────────────────────────────────────────────────
/// Paper plane — Send
pub const ICON_SEND: &str = "\u{f1d8}";
/// Pencil — Draft
pub const ICON_DRAFT: &str = "\u{f044}";
/// Paperclip — Attach
pub const ICON_ATTACH: &str = "\u{f0c6}";
/// × — Discard / Cancel
pub const ICON_DISCARD: &str = "\u{f00d}";
/// Floppy disk — Save
pub const ICON_SAVE: &str = "\u{f0c7}";
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
/// `focused_idx`  — index of the currently focused button (for `Tabs::select`).
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

    // ── Tabs ─────────────────────────────────────────────────────────
    let tab_lines: Vec<Line<'static>> = buttons
        .iter()
        .map(|btn| {
            let style = if btn.disabled {
                t.action_btn_disabled()
            } else if btn.focused {
                t.action_btn_focused()
            } else {
                t.action_btn()
            };
            Line::from(Span::styled(format!(" {} ", btn.label), style))
        })
        .collect();

    let focused_tab = focused_idx.unwrap_or(usize::MAX);

    // Build tabs: highlight_style applied to the selected tab by Tabs widget.
    let tabs = Tabs::new(tab_lines)
        .select(focused_tab)
        .highlight_style(t.action_btn_focused())
        .divider(Span::styled(" · ", t.dimmed()))
        .style(t.status_bar());

    frame.render_widget(tabs, cols[2]);
}
