use ratatui::prelude::*;

/// Create a centered rectangle within `area`, sized by width percentage and a
/// fixed number of rows.
///
/// Small dialogs size by rows: `centered_rect` takes percentages, so passing a
/// row count there collapses the dialog to one row and drops its body text.
pub fn centered_rows(percent_x: u16, rows: u16, area: Rect) -> Rect {
    let width = area.width.saturating_mul(percent_x.min(100)) / 100;
    let height = rows.min(area.height);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

/// Create a centered rectangle within `area`, sized by percentage.
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
