//! Styling utilities and color schemes
//!
//! This module contains color helpers and style constants used throughout the UI.

use ratatui::style::Color;

/// Get the color for an HTTP method
pub fn get_method_color(method: &str) -> Color {
    match method {
        "GET" => Color::Green,
        "POST" => Color::Blue,
        "PUT" => Color::Yellow,
        "DELETE" => Color::Red,
        "PATCH" => Color::Cyan,
        _ => Color::White,
    }
}

/// Method column width for consistent formatting
#[allow(dead_code)]
pub const METHOD_COLUMN_WIDTH: usize = 7;

/// Scroll lines per action (Ctrl+U / Ctrl+D)
#[allow(dead_code)]
pub const SCROLL_LINES_PER_ACTION: usize = 5;
