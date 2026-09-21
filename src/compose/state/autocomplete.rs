/*! Autocomplete popup state for compose header fields. */

use super::FocusedField;

/// Autocomplete suggestion popup state.
#[derive(Debug, Clone)]
pub struct AutocompleteState {
    /// The suggestions (name, email).
    pub suggestions: Vec<(Option<String>, String)>,
    /// Currently selected index.
    pub selected: usize,
    /// The field that triggered autocomplete.
    pub field: FocusedField,
}

impl AutocompleteState {
    pub fn new(field: FocusedField, suggestions: Vec<(Option<String>, String)>) -> Self {
        Self {
            suggestions,
            selected: 0,
            field,
        }
    }

    pub fn move_up(&mut self) {
        if self.suggestions.is_empty() {
            return;
        }
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        if self.suggestions.is_empty() {
            return;
        }
        self.selected = (self.selected + 1).min(self.suggestions.len() - 1);
    }

    /// Formatted display for the currently selected suggestion.
    pub fn current(&self) -> Option<String> {
        self.suggestions
            .get(self.selected)
            .map(|(name, email)| match name {
                Some(n) if !n.is_empty() => format!("\"{}\" <{}>", n, email),
                _ => email.clone(),
            })
    }
}
