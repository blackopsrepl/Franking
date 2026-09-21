use ratatui::style::{Color, Modifier, Style};

mod loader;

pub use loader::{fallback_theme, parse_colors_toml, parse_hex_color, theme};

/// The resolved color palette used across all UI modules.
#[derive(Debug, Clone)]
pub struct Theme {
    pub accent: Color,
    pub background: Color,
    pub foreground: Color,
    pub cursor: Color,
    pub selection_fg: Color,
    pub selection_bg: Color,
    pub color0: Color,
    pub color1: Color,
    pub color2: Color,
    pub color3: Color,
    pub color4: Color,
    pub color5: Color,
    pub color6: Color,
    pub color7: Color,
    pub color8: Color,
    pub color9: Color,
    pub color10: Color,
    pub color11: Color,
    pub color12: Color,
    pub color13: Color,
    pub color14: Color,
    pub color15: Color,
}

impl Theme {
    // ── Semantic styles ─────────────────────────────────────────────

    /// Title bar / header background.
    pub fn header(&self) -> Style {
        Style::default().fg(self.background).bg(self.accent)
    }

    /// Selected row in a list or table.
    pub fn selected(&self) -> Style {
        Style::default()
            .fg(self.selection_fg)
            .bg(self.selection_bg)
            .add_modifier(Modifier::BOLD)
    }

    /// Unread / unseen envelope.
    pub fn unread(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Flagged / starred envelope.
    pub fn flagged(&self) -> Style {
        Style::default()
            .fg(self.color3)
            .add_modifier(Modifier::BOLD)
    }

    /// Active folder in the sidebar.
    pub fn folder_active(&self) -> Style {
        Style::default()
            .fg(self.background)
            .bg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Inactive folder.
    pub fn folder_inactive(&self) -> Style {
        Style::default().fg(self.color7)
    }

    /// Status bar background.
    pub fn status_bar(&self) -> Style {
        Style::default().fg(self.foreground).bg(self.color0)
    }

    /// Status bar key hint.
    pub fn status_key(&self) -> Style {
        Style::default()
            .fg(self.background)
            .bg(self.color4)
            .add_modifier(Modifier::BOLD)
    }

    /// Status bar description text.
    pub fn status_desc(&self) -> Style {
        Style::default().fg(self.color8)
    }

    /// Border lines.
    pub fn border(&self) -> Style {
        Style::default().fg(self.color8)
    }

    /// Border lines when focused.
    pub fn border_focused(&self) -> Style {
        Style::default().fg(self.accent)
    }

    /// Dimmed / secondary text.
    pub fn dimmed(&self) -> Style {
        Style::default().fg(self.color8)
    }

    /// Normal text.
    pub fn normal(&self) -> Style {
        Style::default().fg(self.foreground)
    }

    /// Error text.
    pub fn error(&self) -> Style {
        Style::default()
            .fg(Color::Rgb(224, 108, 117))
            .add_modifier(Modifier::BOLD)
    }

    /// Accent-colored text.
    pub fn accent_style(&self) -> Style {
        Style::default().fg(self.accent)
    }

    /// Message header labels (From:, To:, Subject:, etc.).
    pub fn header_label(&self) -> Style {
        Style::default()
            .fg(self.color4)
            .add_modifier(Modifier::BOLD)
    }

    /// Message header values.
    pub fn header_value(&self) -> Style {
        Style::default().fg(self.foreground)
    }

    /// Popup / overlay background.
    pub fn popup(&self) -> Style {
        Style::default().fg(self.foreground).bg(self.color0)
    }

    /// Popup title.
    pub fn popup_title(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Search input.
    pub fn search_input(&self) -> Style {
        Style::default().fg(self.foreground).bg(self.color0)
    }

    /// Loading spinner.
    pub fn spinner(&self) -> Style {
        Style::default()
            .fg(self.color6)
            .add_modifier(Modifier::BOLD)
    }

    // ── Action bar / modal editing ───────────────────────────────────

    /// Unfocused action button in the action bar.
    pub fn action_btn(&self) -> Style {
        Style::default().fg(self.color7)
    }

    /// Focused (selected) action button — inverted with accent.
    pub fn action_btn_focused(&self) -> Style {
        Style::default()
            .fg(self.background)
            .bg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Disabled action button (Draft / Attach that aren't wired up yet).
    pub fn action_btn_disabled(&self) -> Style {
        Style::default()
            .fg(self.color8)
            .add_modifier(Modifier::DIM | Modifier::ITALIC)
    }

    /// Mode pill for Nav mode.
    pub fn mode_nav(&self) -> Style {
        Style::default()
            .fg(self.background)
            .bg(self.color8)
            .add_modifier(Modifier::BOLD)
    }

    /// Mode pill for Insert mode.
    pub fn mode_insert(&self) -> Style {
        Style::default()
            .fg(self.background)
            .bg(self.color4)
            .add_modifier(Modifier::BOLD)
    }
}
