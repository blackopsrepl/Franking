/*! Theme module wiring. */

mod loader;
mod palette;

pub use loader::{fallback_theme, parse_colors_toml, parse_hex_color, theme};
pub use palette::Theme;
