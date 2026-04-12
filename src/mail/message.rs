use html2text::from_read;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageDisplayMode {
    Auto,
    Plain,
    Html,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageAttachment {
    pub file_name: Option<String>,
    pub content_type: Option<String>,
    pub is_inline: bool,
    pub size: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageContent {
    pub headers: Vec<MessageHeader>,
    pub plain_body: Option<String>,
    pub html_body: Option<String>,
    pub attachments: Vec<MessageAttachment>,
    pub preferred_display: MessageDisplayMode,
}

impl MessageContent {
    pub fn header_value(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case(name))
            .map(|header| header.value.as_str())
    }

    pub fn subject(&self) -> &str {
        self.header_value("Subject").unwrap_or("")
    }

    pub fn has_plain_body(&self) -> bool {
        self.plain_body
            .as_deref()
            .map(|body| !body.trim().is_empty())
            .unwrap_or(false)
    }

    pub fn has_html_body(&self) -> bool {
        self.html_body
            .as_deref()
            .map(|body| !body.trim().is_empty())
            .unwrap_or(false)
    }

    pub fn available_display_modes(&self) -> Vec<MessageDisplayMode> {
        let mut modes = vec![MessageDisplayMode::Auto];
        if self.has_plain_body() {
            modes.push(MessageDisplayMode::Plain);
        }
        if self.has_html_body() {
            modes.push(MessageDisplayMode::Html);
        }
        modes
    }

    pub fn render_body(&self, mode: MessageDisplayMode, width: usize) -> String {
        match self.resolve_display_mode(mode) {
            MessageDisplayMode::Auto => self.render_auto_body(width),
            MessageDisplayMode::Plain => self
                .plain_body
                .as_deref()
                .map(str::to_string)
                .unwrap_or_default(),
            MessageDisplayMode::Html => self.render_html_body(width).unwrap_or_else(|| {
                self.plain_body
                    .as_deref()
                    .map(str::to_string)
                    .unwrap_or_default()
            }),
        }
    }

    pub fn resolve_display_mode(&self, requested: MessageDisplayMode) -> MessageDisplayMode {
        match requested {
            MessageDisplayMode::Auto => self.preferred_display,
            MessageDisplayMode::Plain if self.has_plain_body() => MessageDisplayMode::Plain,
            MessageDisplayMode::Html if self.has_html_body() => MessageDisplayMode::Html,
            _ => self.preferred_display,
        }
    }

    pub fn render_for_legacy_view(&self, width: usize) -> String {
        let mut out = String::new();
        for header in &self.headers {
            out.push_str(&header.name);
            out.push_str(": ");
            out.push_str(&header.value);
            out.push('\n');
        }
        out.push('\n');
        out.push_str(&self.render_body(MessageDisplayMode::Auto, width));
        out
    }

    fn render_auto_body(&self, width: usize) -> String {
        match self.preferred_display {
            MessageDisplayMode::Html => self
                .render_html_body(width)
                .or_else(|| self.plain_body.clone())
                .unwrap_or_default(),
            _ => self
                .plain_body
                .clone()
                .or_else(|| self.render_html_body(width))
                .unwrap_or_default(),
        }
    }

    fn render_html_body(&self, width: usize) -> Option<String> {
        let html = self.html_body.as_deref()?;
        let width = width.max(20);
        from_read(html.as_bytes(), width)
            .ok()
            .map(|rendered| normalize_newlines(&rendered))
    }
}

pub fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n").replace('\r', "\n")
}
