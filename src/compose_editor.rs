use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_textarea::TextArea;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ComposeEditorResult {
    pub handled: bool,
    pub text_modified: bool,
}

#[derive(Clone, Debug)]
pub struct ComposeEditor {
    textarea: TextArea<'static>,
    search_active: bool,
    search_query: String,
}

impl Default for ComposeEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl ComposeEditor {
    pub fn new() -> Self {
        let mut textarea = TextArea::default();
        textarea.set_max_histories(1000);
        Self {
            textarea,
            search_active: false,
            search_query: String::new(),
        }
    }

    pub fn from_text(text: &str) -> Self {
        let mut editor = if text.is_empty() {
            Self::new()
        } else {
            let mut textarea = TextArea::from(text.lines());
            textarea.set_max_histories(1000);
            Self {
                textarea,
                search_active: false,
                search_query: String::new(),
            }
        };
        editor.sync_search();
        editor
    }

    pub fn textarea(&self) -> &TextArea<'static> {
        &self.textarea
    }

    pub fn cursor(&self) -> (usize, usize) {
        self.textarea.cursor()
    }

    pub fn is_empty(&self) -> bool {
        self.text().trim().is_empty()
    }

    pub fn text(&self) -> String {
        self.textarea.lines().join("\n")
    }

    /// Insert text at the cursor, used to drop in a snippet.
    pub fn insert_str(&mut self, text: &str) {
        self.textarea.insert_str(text);
    }

    pub fn is_search_active(&self) -> bool {
        self.search_active
    }

    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    pub fn clear_search(&mut self) {
        self.search_active = false;
        self.search_query.clear();
        self.sync_search();
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> ComposeEditorResult {
        if self.search_active {
            return self.handle_search_key(key);
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('f') {
            self.search_active = true;
            self.search_query.clear();
            self.sync_search();
            return ComposeEditorResult {
                handled: true,
                text_modified: false,
            };
        }

        if key.code == KeyCode::F(3) {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                self.textarea.search_back(false);
            } else {
                self.textarea.search_forward(false);
            }
            return ComposeEditorResult {
                handled: true,
                text_modified: false,
            };
        }

        let modified = self.textarea.input(key);
        ComposeEditorResult {
            handled: true,
            text_modified: modified,
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> ComposeEditorResult {
        match key.code {
            KeyCode::Esc => {
                self.clear_search();
            }
            KeyCode::Enter | KeyCode::F(3) => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.textarea.search_back(false);
                } else {
                    self.textarea.search_forward(false);
                }
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.sync_search();
                if !self.search_query.is_empty() {
                    self.textarea.search_forward(false);
                }
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.search_query.push(c);
                self.sync_search();
                self.textarea.search_forward(false);
            }
            _ => {}
        }

        ComposeEditorResult {
            handled: true,
            text_modified: false,
        }
    }

    fn sync_search(&mut self) {
        let pattern = if self.search_query.is_empty() {
            String::new()
        } else {
            regex::escape(&self.search_query)
        };
        let _ = self.textarea.set_search_pattern(pattern);
    }
}
