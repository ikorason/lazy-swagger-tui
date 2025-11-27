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
