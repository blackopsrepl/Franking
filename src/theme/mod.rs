use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use ratatui::style::{Color, Modifier, Style};

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

// ── Singleton ───────────────────────────────────────────────────────

static THEME: OnceLock<Theme> = OnceLock::new();

/// Returns the global theme, loading it once on first access.
pub fn theme() -> &'static Theme {
    THEME.get_or_init(|| load_theme().unwrap_or_else(|_| fallback_theme()))
}

// ── Loading ─────────────────────────────────────────────────────────

fn colors_toml_path() -> Option<PathBuf> {
    // Standard SolverForge location
    let home = dirs::home_dir()?;
    let path = home.join(".local/share/solverforge/default/theme/colors.toml");
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

fn load_theme() -> anyhow::Result<Theme> {
    let path = colors_toml_path().ok_or_else(|| anyhow::anyhow!("colors.toml not found"))?;
    let content = std::fs::read_to_string(&path)?;
    parse_colors_toml(&content)
}

/// Parse a SolverForge colors.toml into a Theme.
pub fn parse_colors_toml(content: &str) -> anyhow::Result<Theme> {
    let table: HashMap<String, String> = toml::from_str(content)?;

    let get = |key: &str| -> anyhow::Result<Color> {
        let hex = table
            .get(key)
            .ok_or_else(|| anyhow::anyhow!("missing color: {key}"))?;
        parse_hex_color(hex)
    };

    Ok(Theme {
        accent: get("accent")?,
        background: get("background")?,
        foreground: get("foreground")?,
        cursor: get("cursor")?,
        selection_fg: get("selection_foreground")?,
        selection_bg: get("selection_background")?,
        color0: get("color0")?,
        color1: get("color1")?,
        color2: get("color2")?,
        color3: get("color3")?,
        color4: get("color4")?,
        color5: get("color5")?,
        color6: get("color6")?,
        color7: get("color7")?,
        color8: get("color8")?,
        color9: get("color9")?,
        color10: get("color10")?,
        color11: get("color11")?,
        color12: get("color12")?,
        color13: get("color13")?,
        color14: get("color14")?,
        color15: get("color15")?,
    })
}

/// Parse `#RRGGBB` into `Color::Rgb`.
pub fn parse_hex_color(hex: &str) -> anyhow::Result<Color> {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 {
        anyhow::bail!("invalid hex color length: {hex}");
    }
    let r = u8::from_str_radix(&hex[0..2], 16)?;
    let g = u8::from_str_radix(&hex[2..4], 16)?;
    let b = u8::from_str_radix(&hex[4..6], 16)?;
    Ok(Color::Rgb(r, g, b))
}

/// Hardcoded hackerman palette — used when colors.toml is unavailable.
pub fn fallback_theme() -> Theme {
    Theme {
        accent: Color::Rgb(130, 251, 156),       // #82FB9C
        background: Color::Rgb(11, 12, 22),      // #0B0C16
        foreground: Color::Rgb(221, 247, 255),   // #ddf7ff
        cursor: Color::Rgb(221, 247, 255),       // #ddf7ff
        selection_fg: Color::Rgb(11, 12, 22),    // #0B0C16
        selection_bg: Color::Rgb(221, 247, 255), // #ddf7ff
        color0: Color::Rgb(11, 12, 22),          // #0B0C16
        color1: Color::Rgb(80, 248, 114),        // #50f872
        color2: Color::Rgb(79, 232, 143),        // #4fe88f
        color3: Color::Rgb(80, 247, 212),        // #50f7d4
        color4: Color::Rgb(130, 157, 212),       // #829dd4
        color5: Color::Rgb(134, 167, 223),       // #86a7df
        color6: Color::Rgb(124, 248, 247),       // #7cf8f7
        color7: Color::Rgb(133, 225, 251),       // #85E1FB
        color8: Color::Rgb(106, 110, 149),       // #6a6e95
        color9: Color::Rgb(133, 255, 157),       // #85ff9d
        color10: Color::Rgb(156, 247, 194),      // #9cf7c2
        color11: Color::Rgb(164, 255, 236),      // #a4ffec
        color12: Color::Rgb(196, 210, 237),      // #c4d2ed
        color13: Color::Rgb(205, 219, 244),      // #cddbf4
        color14: Color::Rgb(209, 255, 254),      // #d1fffe
        color15: Color::Rgb(221, 247, 255),      // #ddf7ff
    }
}
