/*! Compose input context shared by the resolver and the app. */

/// Editing mode for forms that still distinguish navigation vs text entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditMode {
    Nav,
    Insert,
}

/// Coarse compose focus buckets used by contextual key resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposeFocus {
    From,
    Header,
    Body,
    ActionBar,
}

/// Runtime compose context needed to resolve keys correctly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComposeKeyContext {
    pub focus: ComposeFocus,
    pub edit_mode: EditMode,
    pub body_search_active: bool,
    pub autocomplete_visible: bool,
    pub confirm_discard_visible: bool,
}
