/*! Terminal-native body document.
The mail layer parses a message once and transforms the canonical body source
into this block model. The UI renders it; it never re-parses MIME or HTML. */

use regex::Regex;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::normalize_newlines;

const HTML_UNWRAPPED_WIDTH: usize = 100_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Paragraph(String),
    Quote(String),
    ListItem { marker: String, text: String },
    Code(String),
    Rule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub text: String,
    pub href: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BodyDocument {
    pub blocks: Vec<Block>,
    pub links: Vec<Link>,
}

impl BodyDocument {
    pub fn from_plain(text: &str) -> Self {
        Self {
            blocks: parse_plain(text),
            links: Vec::new(),
        }
    }

    pub fn from_html(html: &str) -> Self {
        let text = html2text::from_read(html.as_bytes(), HTML_UNWRAPPED_WIDTH)
            .map(|rendered| normalize_newlines(&rendered))
            .unwrap_or_default();
        Self {
            blocks: parse_plain(&text),
            links: extract_links(html),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// Render the canonical body as terminal text at the given width.
    pub fn render(&self, width: usize) -> String {
        let width = width.max(20);
        let mut rendered: Vec<String> = Vec::with_capacity(self.blocks.len());

        for block in &self.blocks {
            let text = match block {
                Block::Paragraph(text) => wrap(text, width, "", "").join("\n"),
                Block::Quote(text) => wrap(text, width, "> ", "> ").join("\n"),
                Block::ListItem { marker, text } => {
                    let indent = " ".repeat(marker.width() + 1);
                    wrap(text, width, &format!("{marker} "), &indent).join("\n")
                }
                Block::Code(code) => code.clone(),
                Block::Rule => "\u{2500}".repeat(width.min(40)),
            };
            if !text.trim().is_empty() {
                rendered.push(text);
            }
        }

        rendered.join("\n\n")
    }

    /// Unwrapped plain text used for search indexing and reply/forward quoting.
    pub fn to_search_text(&self) -> String {
        self.render(HTML_UNWRAPPED_WIDTH)
    }
}

fn parse_plain(text: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut paragraph: Vec<String> = Vec::new();

    let flush = |blocks: &mut Vec<Block>, paragraph: &mut Vec<String>| {
        if !paragraph.is_empty() {
            blocks.push(Block::Paragraph(paragraph.join("\n")));
            paragraph.clear();
        }
    };

    for line in text.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            flush(&mut blocks, &mut paragraph);
            continue;
        }

        if let Some(quote) = trimmed.strip_prefix('>') {
            flush(&mut blocks, &mut paragraph);
            blocks.push(Block::Quote(quote.trim_start().to_string()));
            continue;
        }

        if let Some((marker, item)) = split_list_marker(trimmed) {
            flush(&mut blocks, &mut paragraph);
            blocks.push(Block::ListItem {
                marker,
                text: item.to_string(),
            });
            continue;
        }

        paragraph.push(trimmed.to_string());
    }

    flush(&mut blocks, &mut paragraph);
    blocks
}

fn split_list_marker(line: &str) -> Option<(String, &str)> {
    for marker in ["- ", "* ", "+ "] {
        if let Some(rest) = line.strip_prefix(marker) {
            return Some((marker.trim_end().to_string(), rest));
        }
    }

    let digits = line.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 {
        let rest = &line[digits..];
        if let Some(item) = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")) {
            return Some((format!("{}.", &line[..digits]), item));
        }
    }

    None
}

fn wrap(text: &str, width: usize, initial_indent: &str, subsequent_indent: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut indent = initial_indent.to_string();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(&indent);
        } else if current.width() + 1 + word.width() <= width {
            current.push(' ');
        } else {
            lines.push(std::mem::take(&mut current));
            indent = subsequent_indent.to_string();
            current.push_str(&indent);
        }

        let mut remaining = word;
        loop {
            let available = width.saturating_sub(current.width()).max(1);
            if remaining.width() <= available {
                current.push_str(remaining);
                break;
            }
            let (head, tail) = split_at_width(remaining, available);
            current.push_str(head);
            lines.push(std::mem::take(&mut current));
            indent = subsequent_indent.to_string();
            current.push_str(&indent);
            remaining = tail;
        }
    }

    if !current.trim().is_empty() || lines.is_empty() {
        lines.push(current);
    }
    lines
}

fn split_at_width(text: &str, max: usize) -> (&str, &str) {
    let mut width = 0;
    for (idx, ch) in text.char_indices() {
        let char_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if idx > 0 && width + char_width > max {
            return (&text[..idx], &text[idx..]);
        }
        width += char_width;
    }
    (text, "")
}

fn extract_links(html: &str) -> Vec<Link> {
    let Ok(pattern) = Regex::new(r#"(?is)<a\s[^>]*href\s*=\s*["']([^"']+)["'][^>]*>(.*?)</a>"#)
    else {
        return Vec::new();
    };

    pattern
        .captures_iter(html)
        .map(|captures| {
            let href = captures
                .get(1)
                .map(|m| m.as_str().trim())
                .unwrap_or_default();
            let text = captures
                .get(2)
                .map(|m| strip_tags(m.as_str()))
                .unwrap_or_default();
            Link {
                text: if text.is_empty() {
                    href.to_string()
                } else {
                    text
                },
                href: href.to_string(),
            }
        })
        .filter(|link| !link.href.is_empty())
        .collect()
}

fn strip_tags(html: &str) -> String {
    let Ok(pattern) = Regex::new(r"(?s)<[^>]*>") else {
        return html.trim().to_string();
    };
    pattern.replace_all(html, "").trim().to_string()
}

#[cfg(test)]
mod tests {
    use unicode_width::UnicodeWidthStr;

    use super::{Block, BodyDocument};

    #[test]
    fn plain_text_blocks_preserve_quotes_and_lists() {
        let document =
            BodyDocument::from_plain("Hello world\nsecond line\n\n> quoted\n\n- one\n- two");
        assert!(matches!(document.blocks[0], Block::Paragraph(_)));
        assert!(matches!(document.blocks[1], Block::Quote(_)));
        assert!(matches!(document.blocks[2], Block::ListItem { .. }));
    }

    #[test]
    fn html_document_extracts_links_and_text() {
        let document =
            BodyDocument::from_html("<p>See <a href=\"https://example.com\">example</a>.</p>");
        assert_eq!(document.links.len(), 1);
        assert_eq!(document.links[0].href, "https://example.com");
        assert!(document.render(80).contains("example"));
    }

    #[test]
    fn wrapping_respects_width() {
        let document = BodyDocument::from_plain("one two three four five six seven eight nine ten");
        for line in document.render(20).lines() {
            assert!(line.width() <= 20, "line too wide: {line:?}");
        }
    }
}
