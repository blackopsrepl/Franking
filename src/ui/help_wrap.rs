/*! Wrap styled help rows explicitly so scroll limits match visible rows. */

use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthChar;

pub(super) fn wrap_lines(lines: Vec<Line<'_>>, width: usize) -> Vec<Line<'static>> {
    let width = width.max(1);
    let mut wrapped = Vec::new();
    for line in lines {
        let mut row = Vec::new();
        let mut used = 0;
        for span in line.spans {
            let mut part = String::new();
            for ch in span.content.chars() {
                let cells = ch.width().unwrap_or(0);
                if used + cells > width && used > 0 {
                    if !part.is_empty() {
                        row.push(Span::styled(std::mem::take(&mut part), span.style));
                    }
                    wrapped.push(Line::from(std::mem::take(&mut row)));
                    used = 0;
                }
                part.push(ch);
                used += cells;
            }
            if !part.is_empty() {
                row.push(Span::styled(part, span.style));
            }
        }
        wrapped.push(Line::from(row));
    }
    wrapped
}

#[cfg(test)]
mod tests {
    use super::wrap_lines;
    use ratatui::text::Line;

    #[test]
    fn narrow_help_keeps_every_character_and_has_exact_row_count() {
        let rows = wrap_lines(vec![Line::from("1234567890")], 4);
        assert_eq!(rows.len(), 3);
        let text = rows
            .iter()
            .flat_map(|row| row.spans.iter())
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert_eq!(text, "1234567890");
    }
}
