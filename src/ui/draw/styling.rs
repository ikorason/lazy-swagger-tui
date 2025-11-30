//! Styling utilities and color schemes
//!
//! This module contains color helpers and style constants used throughout the UI.
//!
//! The app uses the terminal's default colors (Color::Reset) for text and backgrounds
//! to respect the user's terminal theme, while using semantic colors (Green, Red, etc.)
//! for syntax highlighting and status indicators.

use ratatui::style::Color;

/// Get the color for an HTTP method
pub fn get_method_color(method: &str) -> Color {
    match method {
        "GET" => Color::Green,
        "POST" => Color::Blue,
        "PUT" => Color::Yellow,
        "DELETE" => Color::Red,
        "PATCH" => Color::Cyan,
        _ => Color::Reset, // Use terminal default
    }
}

/// Get the default foreground color (uses terminal theme)
pub fn default_fg() -> Color {
    Color::Reset
}

/// Get the default background color (uses terminal theme)
pub fn default_bg() -> Color {
    Color::Reset
}

/// Get a dimmed/muted text color for help text and labels
pub fn muted_fg() -> Color {
    Color::DarkGray
}

/// Get the border color for focused panels
pub fn focused_border() -> Color {
    Color::Cyan
}

/// Get the border color for unfocused panels
pub fn unfocused_border() -> Color {
    Color::DarkGray
}
