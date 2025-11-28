//! Reusable UI components
//!
//! This module contains shared UI components used throughout the application:
//! - Header (title, status, auth)
//! - Footer (command help)
//! - Search bar
//! - Loading spinners
//! - Error/empty state messages

use crate::state::AppState;
use crate::types::{AuthState, InputMode, LoadingState, ViewMode};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

/// Render the application header with status and auth info
pub fn render_header(
    frame: &mut Frame,
    area: Rect,
    swagger_url: &str,
    loading_state: &LoadingState,
    endpoints_count: usize,
    auth_state: &AuthState,
) {
    let status_text = match loading_state {
        LoadingState::Idle => "Idle".to_string(),
        LoadingState::Fetching => "Fetching...".to_string(),
        LoadingState::Parsing => "Parsing...".to_string(),
        LoadingState::Complete => format!("{endpoints_count} endpoints loaded"),
        LoadingState::Error(_) => "Error".to_string(),
    };

    let auth_status = get_auth_status_text(auth_state);

    let header_text = format!("lazy swagger tui - {swagger_url} [{status_text}] | {auth_status}",);

    let header = Paragraph::new(header_text)
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(header, area);
}

/// Render the search bar with active filter indication
pub fn render_search_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let is_active = matches!(state.input.mode, InputMode::Searching);

    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else if !state.search.query.is_empty() {
        Style::default().fg(Color::Green) // Show filter is active
    } else {
        Style::default().fg(Color::DarkGray)
    };

    // Show match count if filtering
    let title = if !state.search.query.is_empty() {
        let count = state.search.filtered_endpoints.len();
        let total = state.data.endpoints.len();
        format!(" Search [{count}/{total}] ")
    } else {
        " Search (/) ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title);

    let search_text = if is_active {
        format!("{}_", state.search.query) // Show cursor
    } else {
        state.search.query.clone()
    };

    let paragraph = Paragraph::new(search_text).block(block);

    frame.render_widget(paragraph, area);
}

/// Render the footer with command help
pub fn render_footer(frame: &mut Frame, area: Rect, view_mode: &ViewMode) {
    let footer_text = match view_mode {
        ViewMode::Flat => {
            "Tab:Panel j/k/↑/↓:Nav Space:Execute/Toggle | g:Group ,:URL a:Auth q:Quit"
        }
        ViewMode::Grouped => {
            "Tab:Panel j/k/↑/↓:Nav Space:Execute/Toggle | g:Ungroup ,:URL a:Auth q:Quit"
        }
    };

    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Commands"));

    frame.render_widget(footer, area);
}

/// Render loading spinner animation
pub fn render_loading_spinner(
    frame: &mut Frame,
    area: Rect,
    loading_state: &LoadingState,
    spinner_index: usize,
) {
    let spinner = ["⠋", "⠙", "⠹", "⠸"];
    let progress_text = match loading_state {
        LoadingState::Fetching => "Fetching swagger.json",
        LoadingState::Parsing => "Parsing endpoints",
        _ => "",
    };

    let loading_text = format!(
        "{} {}\n\nPlease wait...",
        spinner[spinner_index], progress_text
    );

    let loading = Paragraph::new(loading_text)
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("[1] Endpoints"),
        );

    frame.render_widget(loading, area);
}

/// Render error message with retry instructions
pub fn render_error_message(frame: &mut Frame, area: Rect, error: &str, retry_count: u32) {
    let retry_text = if retry_count > 0 {
        format!("\n\nRetry attempt: {retry_count}")
    } else {
        String::new()
    };

    let error_msg = format!("❌ {error}{retry_text}\n\nPress [R] to retry\nPress [F5] to refresh",);

    let error_widget = Paragraph::new(error_msg)
        .style(Style::default().fg(Color::Red))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("[1] Endpoints"),
        );

    frame.render_widget(error_widget, area);
}

/// Render empty state message
pub fn render_empty_message(frame: &mut Frame, area: Rect) {
    let empty = Paragraph::new("No endpoints found\n\nPress [F5] to refresh").block(
        Block::default()
            .borders(Borders::ALL)
            .title("[1] Endpoints"),
    );

    frame.render_widget(empty, area);
}

/// Render no search results message
pub fn render_no_search_results(frame: &mut Frame, area: Rect) {
    let empty = Paragraph::new("No matching endpoints\n\nPress [Esc] or [Ctrl+L] to clear search")
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("[1] Search Results"),
        );

    frame.render_widget(empty, area);
}

/// Get authentication status display text
fn get_auth_status_text(auth: &AuthState) -> String {
    if auth.is_authenticated() {
        let display = auth.get_masked_display();
        format!("🔒 {display} | 'a':edit 'A':clear")
    } else {
        "🔓 Not authenticated | 'a':set token".to_string()
    }
}
