use franking::theme::{fallback_theme, parse_colors_toml, parse_hex_color};
use pretty_assertions::assert_eq;
use ratatui::style::Color;

#[test]
fn parse_hex_color_valid() {
    assert_eq!(
        parse_hex_color("#82FB9C").unwrap(),
        Color::Rgb(130, 251, 156)
    );
    assert_eq!(parse_hex_color("#0B0C16").unwrap(), Color::Rgb(11, 12, 22));
    assert_eq!(
        parse_hex_color("#ddf7ff").unwrap(),
        Color::Rgb(221, 247, 255)
    );
}

#[test]
fn parse_hex_color_no_hash() {
    assert_eq!(
        parse_hex_color("82FB9C").unwrap(),
        Color::Rgb(130, 251, 156)
    );
}

#[test]
fn parse_hex_color_invalid_length() {
    assert!(parse_hex_color("#FFF").is_err());
    assert!(parse_hex_color("").is_err());
}

#[test]
fn parse_hex_color_invalid_chars() {
    assert!(parse_hex_color("#ZZZZZZ").is_err());
}

#[test]
fn parse_colors_toml_fixture() {
    let content = include_str!("fixtures/colors.toml");
    let theme = parse_colors_toml(content).unwrap();

    assert_eq!(theme.accent, Color::Rgb(130, 251, 156));
    assert_eq!(theme.background, Color::Rgb(11, 12, 22));
    assert_eq!(theme.foreground, Color::Rgb(221, 247, 255));
    assert_eq!(theme.color0, Color::Rgb(11, 12, 22));
    assert_eq!(theme.color8, Color::Rgb(106, 110, 149));
}

#[test]
fn parse_colors_toml_missing_key() {
    let content = "accent = \"#82FB9C\"";
    assert!(parse_colors_toml(content).is_err());
}

#[test]
fn fallback_theme_has_correct_accent() {
    let theme = fallback_theme();
    assert_eq!(theme.accent, Color::Rgb(130, 251, 156));
    assert_eq!(theme.background, Color::Rgb(11, 12, 22));
}
