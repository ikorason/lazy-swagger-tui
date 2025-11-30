//! Main panel rendering
//!
//! This module contains rendering functions for the two main panels:
//! - Endpoints panel (left side) - flat or grouped list
//! - Details panel (right side) - tabs with endpoint details

use super::components::{
    render_empty_message, render_error_message, render_loading_spinner, render_no_search_results,
};
use super::{styling, tabs::*};
use crate::state::AppState;
use crate::types::{DetailTab, LoadingState, PanelFocus, RenderItem, ViewMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use styling::get_method_color;

/// Render the left panel with endpoint list (flat or grouped)
pub fn render_endpoints_panel(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    spinner_index: usize,
    list_state: &mut ListState,
) {
    match &state.data.loading_state {
        LoadingState::Fetching | LoadingState::Parsing => {
            render_loading_spinner(frame, area, &state.data.loading_state, spinner_index);
        }
        LoadingState::Error(error) => {
            render_error_message(frame, area, error, state.data.retry_count);
        }
        LoadingState::Complete | LoadingState::Idle => {
            if state.active_endpoints().is_empty() {
                if !state.search.query.is_empty() {
                    // Searching but no results
                    render_no_search_results(frame, area);
                } else {
                    // No endpoints loaded
                    render_empty_message(frame, area);
                }
            } else {
                match &state.ui.view_mode {
                    ViewMode::Flat => {
                        render_flat_list(frame, area, state, list_state);
                    }
                    ViewMode::Grouped => {
                        render_grouped_list(frame, area, state, list_state);
                    }
                }
            }
        }
    }
}

/// Render the right panel with endpoint details and tabs
pub fn render_details_panel(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    selected_index: usize,
) {
    // Get the selected endpoint
    let selected_endpoint = state.get_selected_endpoint(selected_index);

    // Determine border color based on panel focus
    let border_color = if state.ui.panel_focus == PanelFocus::Details {
        styling::focused_border()
    } else {
        styling::unfocused_border()
    };

    // Create the main block
    let block = Block::default()
        .title("[2] Details & Response")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // Handle loading/error states
    match &state.data.loading_state {
        LoadingState::Fetching | LoadingState::Parsing => {
            let loading =
                Paragraph::new("Loading endpoints...").style(Style::default().fg(Color::Yellow));
            frame.render_widget(loading, inner_area);
            return;
        }
        LoadingState::Error(e) => {
            let error = Paragraph::new(format!("Error loading endpoints:\n\n{e}"))
                .style(Style::default().fg(Color::Red));
            frame.render_widget(error, inner_area);
            return;
        }
        _ => {}
    }

    // Split into: Tab bar (1 line) + Content area (rest)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Tab bar
            Constraint::Min(0),    // Content area
        ])
        .split(inner_area);

    // Render tab bar
    render_tab_bar(frame, chunks[0], state);

    // Render active tab content
    if let Some(endpoint) = selected_endpoint {
        match state.ui.active_detail_tab {
            DetailTab::Endpoint => render_endpoint_tab(frame, chunks[1], &endpoint),
            DetailTab::Request => render_request_tab(frame, chunks[1], &endpoint, state),
            DetailTab::Headers => render_headers_tab(frame, chunks[1], state),
            DetailTab::Response => render_response_tab(frame, chunks[1], &endpoint, state),
        }
    } else {
        // No endpoint selected
        let empty =
            Paragraph::new("No endpoint selected").style(Style::default().fg(Color::DarkGray));
        frame.render_widget(empty, chunks[1]);
    }
}

// ============================================================================
// Private Helper Functions
// ============================================================================

/// Render flat endpoint list
fn render_flat_list(frame: &mut Frame, area: Rect, state: &AppState, list_state: &mut ListState) {
    let items: Vec<ListItem> = state
        .active_endpoints()
        .iter()
        .map(|endpoint| {
            let method_color = get_method_color(&endpoint.method);

            let line = Line::from(vec![
                Span::styled(
                    format!("{:7}", endpoint.method),
                    Style::default()
                        .fg(method_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::raw(&endpoint.path),
            ]);

            ListItem::new(line)
        })
        .collect();

    // Determine border color based on panel focus
    let border_color = if state.ui.panel_focus == PanelFocus::EndpointsList {
        styling::focused_border()
    } else {
        styling::unfocused_border()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(
                    "[1] Endpoints ({})",
                    state.active_endpoints().len()
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, area, list_state);
}

/// Render grouped endpoint list (with expandable groups)
fn render_grouped_list(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    list_state: &mut ListState,
) {
    let mut items = Vec::new();
    let render_items = state.get_render_items();

    for item in &render_items {
        match item {
            RenderItem::GroupHeader {
                name,
                count,
                expanded,
            } => {
                let icon = if *expanded { "▼" } else { "▶" };
                let line = Line::from(vec![Span::styled(
                    format!("{icon} {name} ({count})"),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )]);
                items.push(ListItem::new(line));
            }
            RenderItem::Endpoint { endpoint } => {
                let method_color = get_method_color(&endpoint.method);

                let line = Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("{:7}", endpoint.method),
                        Style::default()
                            .fg(method_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::raw(&endpoint.path),
                ]);

                items.push(ListItem::new(line));
            }
        }
    }

    // Determine border color based on panel focus
    let border_color = if state.ui.panel_focus == PanelFocus::EndpointsList {
        styling::focused_border()
    } else {
        styling::unfocused_border()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(
                    "[1] Endpoints - {} groups",
                    state.active_grouped_endpoints().len()
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, area, list_state);
}

/// Render the tab bar showing [ Endpoint ] [ Request ] [ Headers ] [ Response ]
fn render_tab_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let active_tab = &state.ui.active_detail_tab;

    // Check if request is executing
    let is_executing = state.request.executing_endpoint.is_some();

    // Build tab labels with highlighting
    let endpoint_style = if *active_tab == DetailTab::Endpoint {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(styling::default_fg())
    };

    let request_style = if *active_tab == DetailTab::Request {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(styling::default_fg())
    };

    let headers_style = if *active_tab == DetailTab::Headers {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(styling::default_fg())
    };

    let response_label = if is_executing {
        "Response (...)"
    } else {
        "Response"
    };

    let response_style = if *active_tab == DetailTab::Response {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(styling::default_fg())
    };

    let tabs = Line::from(vec![
        Span::styled("[ ", Style::default().fg(Color::DarkGray)),
        Span::styled("Endpoint", endpoint_style),
        Span::styled(" ] [ ", Style::default().fg(Color::DarkGray)),
        Span::styled("Request", request_style),
        Span::styled(" ] [ ", Style::default().fg(Color::DarkGray)),
        Span::styled("Headers", headers_style),
        Span::styled(" ] [ ", Style::default().fg(Color::DarkGray)),
        Span::styled(response_label, response_style),
        Span::styled(" ]", Style::default().fg(Color::DarkGray)),
    ]);

    let tab_bar = Paragraph::new(tabs);
    frame.render_widget(tab_bar, area);
}
