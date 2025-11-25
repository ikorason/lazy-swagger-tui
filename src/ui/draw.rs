use std::collections::HashMap;

use crate::state::AppState;
use crate::types::{
    ApiEndpoint, AuthState, DetailTab, InputMode, LoadingState, Parameter, RenderItem,
    RequestEditMode, ViewMode,
};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::widgets::Wrap;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

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
        LoadingState::Complete => format!("{} endpoints loaded", endpoints_count),
        LoadingState::Error(_) => "Error".to_string(),
    };

    let auth_status = get_auth_status_text(auth_state);

    let header_text = format!(
        "lazy swagger tui - {} [{}] | {}",
        swagger_url, status_text, auth_status
    );

    let header = Paragraph::new(header_text)
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(header, area);
}

pub fn render_search_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let is_active = matches!(state.input_mode, InputMode::Searching);

    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else if !state.search_query.is_empty() {
        Style::default().fg(Color::Green) // Show filter is active
    } else {
        Style::default().fg(Color::DarkGray)
    };

    // Show match count if filtering
    let title = if !state.search_query.is_empty() {
        let count = state.filtered_endpoints.len();
        let total = state.endpoints.len();
        format!(" Search [{}/{}] ", count, total)
    } else {
        " Search (/) ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title);

    let search_text = if is_active {
        format!("{}_", state.search_query) // Show cursor
    } else {
        state.search_query.clone()
    };

    let paragraph = Paragraph::new(search_text).block(block);

    frame.render_widget(paragraph, area);
}

fn get_auth_status_text(auth: &AuthState) -> String {
    if auth.is_authenticated() {
        let display = auth.get_masked_display();
        format!("🔒 {} | 'a':edit 'A':clear", display)
    } else {
        "🔓 Not authenticated | 'a':set token".to_string()
    }
}

pub fn render_endpoints_panel(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    spinner_index: usize,
    list_state: &mut ListState,
) {
    match &state.loading_state {
        LoadingState::Fetching | LoadingState::Parsing => {
            render_loading_spinner(frame, area, &state.loading_state, spinner_index);
        }
        LoadingState::Error(error) => {
            render_error_message(frame, area, error, state.retry_count);
        }
        LoadingState::Complete | LoadingState::Idle => {
            if state.active_endpoints().is_empty() {
                if !state.search_query.is_empty() {
                    // Searching but no results
                    render_no_search_results(frame, area);
                } else {
                    // No endpoints loaded
                    render_empty_message(frame, area);
                }
            } else {
                match &state.view_mode {
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

fn render_loading_spinner(
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
        .block(Block::default().borders(Borders::ALL).title("Endpoints"));

    frame.render_widget(loading, area);
}

fn render_error_message(frame: &mut Frame, area: Rect, error: &str, retry_count: u32) {
    let retry_text = if retry_count > 0 {
        format!("\n\nRetry attempt: {}", retry_count)
    } else {
        String::new()
    };

    let error_msg = format!(
        "❌ {}{}\n\nPress [R] to retry\nPress [F5] to refresh",
        error, retry_text
    );

    let error_widget = Paragraph::new(error_msg)
        .style(Style::default().fg(Color::Red))
        .block(Block::default().borders(Borders::ALL).title("Endpoints"));

    frame.render_widget(error_widget, area);
}

fn render_empty_message(frame: &mut Frame, area: Rect) {
    let empty = Paragraph::new("No endpoints found\n\nPress [F5] to refresh")
        .block(Block::default().borders(Borders::ALL).title("Endpoints"));

    frame.render_widget(empty, area);
}

fn render_no_search_results(frame: &mut Frame, area: Rect) {
    let empty = Paragraph::new("No matching endpoints\n\nPress [Esc] or [Ctrl+L] to clear search")
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Search Results"));

    frame.render_widget(empty, area);
}

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
    use crate::types::PanelFocus;
    let border_color = if state.panel_focus == PanelFocus::EndpointsList {
        Color::Cyan
    } else {
        Color::White
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!("Endpoints ({})", state.active_endpoints().len()))
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

fn render_grouped_list(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    list_state: &mut ListState,
) {
    let mut items = Vec::new();

    for item in &state.render_items {
        match item {
            RenderItem::GroupHeader {
                name,
                count,
                expanded,
            } => {
                let icon = if *expanded { "▼" } else { "▶" };
                let line = Line::from(vec![Span::styled(
                    format!("{} {} ({})", icon, name, count),
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
    use crate::types::PanelFocus;
    let border_color = if state.panel_focus == PanelFocus::EndpointsList {
        Color::Cyan
    } else {
        Color::White
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(
                    "Endpoints - {} groups",
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

pub fn render_details_panel(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    selected_index: usize,
) {
    // Get the selected endpoint
    let selected_endpoint = state.get_selected_endpoint(selected_index);

    // Determine border color based on panel focus
    use crate::types::PanelFocus;
    let border_color = if state.panel_focus == PanelFocus::Details {
        Color::Cyan
    } else {
        Color::White
    };

    // Create the main block
    let block = Block::default()
        .title("Details & Response")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // Handle loading/error states
    match &state.loading_state {
        LoadingState::Fetching | LoadingState::Parsing => {
            let loading =
                Paragraph::new("Loading endpoints...").style(Style::default().fg(Color::Yellow));
            frame.render_widget(loading, inner_area);
            return;
        }
        LoadingState::Error(e) => {
            let error = Paragraph::new(format!("Error loading endpoints:\n\n{}", e))
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
        match state.active_detail_tab {
            DetailTab::Endpoint => render_endpoint_tab(frame, chunks[1], endpoint),
            DetailTab::Request => render_request_tab(frame, chunks[1], endpoint, state),
            DetailTab::Headers => render_headers_tab(frame, chunks[1], state),
            DetailTab::Response => render_response_tab(frame, chunks[1], endpoint, state),
        }
    } else {
        // No endpoint selected
        let empty =
            Paragraph::new("No endpoint selected").style(Style::default().fg(Color::DarkGray));
        frame.render_widget(empty, chunks[1]);
    }
}

/// Render the tab bar showing [ Endpoint ] [ Headers ] [ Response ]
fn render_tab_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let active_tab = &state.active_detail_tab;

    // Check if request is executing
    let is_executing = state.executing_endpoint.is_some();

    // Build tab labels with highlighting
    let endpoint_style = if *active_tab == DetailTab::Endpoint {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let request_style = if *active_tab == DetailTab::Request {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let headers_style = if *active_tab == DetailTab::Headers {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
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
        Style::default().fg(Color::White)
    };

    let tabs = Line::from(vec![
        Span::styled("[ ", Style::default().fg(Color::DarkGray)),
        Span::styled("Endpoint", endpoint_style),
        Span::styled(" ] [ ", Style::default().fg(Color::DarkGray)),
        Span::styled("Request", request_style), // NEW
        Span::styled(" ] [ ", Style::default().fg(Color::DarkGray)),
        Span::styled("Headers", headers_style),
        Span::styled(" ] [ ", Style::default().fg(Color::DarkGray)),
        Span::styled(response_label, response_style),
        Span::styled(" ]", Style::default().fg(Color::DarkGray)),
    ]);

    let tab_bar = Paragraph::new(tabs);
    frame.render_widget(tab_bar, area);
}

/// Render the Endpoint tab content
fn render_endpoint_tab(frame: &mut Frame, area: Rect, endpoint: &ApiEndpoint) {
    let mut lines: Vec<Line> = Vec::new();

    let method_color = get_method_color(&endpoint.method);

    lines.push(Line::from(vec![
        Span::styled(
            &endpoint.method,
            Style::default()
                .fg(method_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::raw(&endpoint.path),
    ]));

    lines.push(Line::from("")); // Empty line

    if let Some(summary) = &endpoint.summary {
        lines.push(Line::from(vec![
            Span::styled("Summary: ", Style::default().fg(Color::Cyan)),
            Span::raw(summary),
        ]));
    }

    if !endpoint.tags.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Tags: ", Style::default().fg(Color::Cyan)),
            Span::raw(endpoint.tags.join(", ")),
        ]));
    }

    let content = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White));

    frame.render_widget(content, area);
}

/// Render the Request tab content (parameters, etc.)
fn render_request_tab(frame: &mut Frame, area: Rect, endpoint: &ApiEndpoint, state: &AppState) {
    let mut lines: Vec<Line> = Vec::new();

    // Get path and query parameters for this endpoint
    let path_params: Vec<&Parameter> = endpoint.path_params();
    let query_params: Vec<&Parameter> = endpoint.query_params();

    // Check if there are ANY parameters at all
    if path_params.is_empty() && query_params.is_empty() {
        lines.push(Line::from(Span::styled(
            "No parameters defined for this endpoint",
            Style::default().fg(Color::DarkGray),
        )));

        let content = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(content, area);
        return;
    }

    // Get request config for this endpoint
    let config = state.request_configs.get(&endpoint.path);

    let total_path_params = path_params.len();

    // ===== SECTION 1: Path Parameters =====
    if !path_params.is_empty() {
        lines.push(Line::from(Span::styled(
            "Path Parameters:",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from("")); // Empty line

        // Display each path parameter
        for (idx, param) in path_params.iter().enumerate() {
            let global_idx = idx; // Path params come first
            let is_selected = state.selected_param_index == global_idx;

            let current_value =
                if let RequestEditMode::Editing(editing_param_name) = &state.request_edit_mode {
                    if editing_param_name == &param.name {
                        state.param_edit_buffer.as_str()
                    } else {
                        config
                            .and_then(|c| c.path_params.get(&param.name))
                            .map(|s| s.as_str())
                            .unwrap_or("")
                    }
                } else {
                    config
                        .and_then(|c| c.path_params.get(&param.name))
                        .map(|s| s.as_str())
                        .unwrap_or("")
                };

            let is_editing = matches!(
                &state.request_edit_mode,
                RequestEditMode::Editing(name) if name == &param.name
            );

            let line = build_param_line(
                param,
                current_value,
                is_selected,
                is_editing,
                true, // is_path_param
            );
            lines.push(line);
        }

        lines.push(Line::from("")); // Empty line after path params
    }

    // ===== SECTION 2: Query Parameters =====
    if !query_params.is_empty() {
        lines.push(Line::from(Span::styled(
            "Query Parameters:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from("")); // Empty line

        // Display each query parameter
        for (idx, param) in query_params.iter().enumerate() {
            let global_idx = total_path_params + idx; // Offset by path param count
            let is_selected = state.selected_param_index == global_idx;

            let current_value =
                if let RequestEditMode::Editing(editing_param_name) = &state.request_edit_mode {
                    if editing_param_name == &param.name {
                        state.param_edit_buffer.as_str()
                    } else {
                        config
                            .and_then(|c| c.query_params.get(&param.name))
                            .map(|s| s.as_str())
                            .unwrap_or("")
                    }
                } else {
                    config
                        .and_then(|c| c.query_params.get(&param.name))
                        .map(|s| s.as_str())
                        .unwrap_or("")
                };

            let is_editing = matches!(
                &state.request_edit_mode,
                RequestEditMode::Editing(name) if name == &param.name
            );

            let line = build_param_line(
                param,
                current_value,
                is_selected,
                is_editing,
                false, // is_path_param
            );
            lines.push(line);
        }

        lines.push(Line::from("")); // Empty line after query params
    }

    // ===== SECTION 3: URL Preview =====
    lines.push(Line::from(Span::styled(
        "Preview URL:",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));

    // Build preview URL with both path and query params
    let preview_url = if let Some(config) = config {
        build_preview_url(&endpoint.path, &config.path_params, &config.query_params)
    } else {
        endpoint.path.clone()
    };

    lines.push(Line::from(Span::styled(
        preview_url,
        Style::default().fg(Color::Yellow),
    )));

    // ===== SECTION 4: Help Text =====
    lines.push(Line::from("")); // Empty line

    let help_text = match &state.request_edit_mode {
        RequestEditMode::Viewing => "j/k/↑/↓: Navigate  |  e: Edit parameter  |  Space: Execute",
        RequestEditMode::Editing(_) => "Type to edit  |  Enter: Confirm  |  Esc: Cancel",
    };

    lines.push(Line::from(Span::styled(
        help_text,
        Style::default().fg(Color::DarkGray),
    )));

    let content = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(content, area);
}

fn build_preview_url(
    path_template: &str,
    path_params: &HashMap<String, String>,
    query_params: &HashMap<String, String>,
) -> String {
    // Step 1: Substitute path parameters
    let mut path = path_template.to_string();

    for (key, value) in path_params {
        let placeholder = format!("{{{}}}", key);
        if path.contains(&placeholder) {
            // Show placeholder if value is empty, otherwise substitute
            if value.is_empty() {
                // Keep the placeholder visible
            } else {
                path = path.replace(&placeholder, value);
            }
        }
    }

    // Step 2: Add query parameters
    let non_empty_params: Vec<String> = query_params
        .iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();

    if non_empty_params.is_empty() {
        path
    } else {
        format!("{}?{}", path, non_empty_params.join("&"))
    }
}

/// Helper function to build a single parameter line with styling
fn build_param_line(
    param: &Parameter,
    current_value: &str,
    is_selected: bool,
    is_editing: bool,
    is_path_param: bool,
) -> Line<'static> {
    // Build type info string (e.g., "integer/int32" or "boolean")
    let type_info = if let Some(schema) = &param.schema {
        let type_str = schema.param_type.as_deref().unwrap_or("unknown");
        if let Some(format) = &schema.format {
            format!("{}/{}", type_str, format)
        } else {
            type_str.to_string()
        }
    } else {
        "unknown".to_string()
    };

    // Build required indicator
    let required_str = if param.required.unwrap_or(false) {
        "*"
    } else {
        ""
    };

    // Selection indicator
    let indicator = if is_selected { "→ " } else { "  " };

    // Value display - show cursor if editing
    let value_display = if is_editing {
        format!("[{}▊]", current_value) // Show cursor
    } else if current_value.is_empty() {
        "[_____]".to_string() // Empty placeholder
    } else {
        format!("[{}]", current_value)
    };

    // Build the line with appropriate styling
    let indicator_style = if is_selected {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let name_style = if is_selected {
        Style::default()
            .fg(if is_path_param {
                Color::Magenta
            } else {
                Color::Cyan
            })
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(if is_path_param {
            Color::Rgb(180, 100, 180) // Dimmed magenta
        } else {
            Color::White
        })
    };

    let value_style = if is_editing {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else if is_selected {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Gray)
    };

    let meta_style = Style::default().fg(Color::DarkGray);

    Line::from(vec![
        Span::styled(indicator, indicator_style),
        Span::styled(format!("{}{}: ", param.name, required_str), name_style),
        Span::styled(value_display, value_style),
        Span::raw("  "),
        Span::styled(format!("({})", type_info), meta_style),
    ])
}

/// Render the Headers tab content
fn render_headers_tab(frame: &mut Frame, area: Rect, state: &AppState) {
    let mut lines: Vec<Line> = Vec::new();

    if let Some(ref response) = state.current_response {
        if !response.headers.is_empty() {
            let mut header_vec: Vec<_> = response.headers.iter().collect();
            header_vec.sort_by_key(|(k, _)| k.as_str());

            for (key, value) in header_vec {
                lines.push(Line::from(vec![
                    Span::styled(format!("{}: ", key), Style::default().fg(Color::Cyan)),
                    Span::raw(value.to_string()),
                ]));
            }
        } else {
            lines.push(Line::from(Span::styled(
                "No headers",
                Style::default().fg(Color::DarkGray),
            )));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "No response yet",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let content = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((state.headers_scroll as u16, 0));

    frame.render_widget(content, area);
}

/// Render the Response tab content
fn render_response_tab(frame: &mut Frame, area: Rect, endpoint: &ApiEndpoint, state: &AppState) {
    let mut lines: Vec<Line> = Vec::new();

    let is_executing = state.executing_endpoint.as_ref() == Some(&endpoint.path);

    if is_executing {
        lines.push(Line::from(vec![Span::styled(
            "⏳ Executing request...",
            Style::default().fg(Color::Cyan),
        )]));
    } else if let Some(ref response) = state.current_response {
        if response.is_error {
            lines.push(Line::from(vec![Span::styled(
                "❌ Error",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(""));

            if let Some(ref err_msg) = response.error_message {
                for line in err_msg.lines() {
                    lines.push(Line::from(Span::styled(
                        line.to_string(),
                        Style::default().fg(Color::Red),
                    )));
                }
            }
        } else {
            // Show status line
            lines.push(Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!("{} {}", response.status, response.status_text),
                    Style::default().fg(Color::Green),
                ),
                Span::raw("  "),
                Span::styled("Duration: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{}ms", response.duration.as_millis())),
            ]));
            lines.push(Line::from("")); // Empty line

            // Show formatted body
            let formatted_body = try_format_json(&response.body);
            for line in formatted_body.lines() {
                lines.push(Line::from(line.to_string()));
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            "Press [Space] to execute request",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let content = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((state.response_body_scroll as u16, 0));

    frame.render_widget(content, area);
}

pub fn render_footer(frame: &mut Frame, area: Rect, view_mode: &ViewMode) {
    let footer_text = match view_mode {
        ViewMode::Flat => {
            "Tab:Panel j/k/↑/↓:Nav Space:Execute/Toggle Ctrl+d/u:Scroll | g:Group ,:URL a:Auth q:Quit"
        }
        ViewMode::Grouped => {
            "Tab:Panel j/k/↑/↓:Nav Space:Execute/Toggle Ctrl+d/u:Scroll | g:Ungroup ,:URL a:Auth q:Quit"
        }
    };

    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Commands"));

    frame.render_widget(footer, area);
}

fn get_method_color(method: &str) -> Color {
    match method {
        "GET" => Color::Green,
        "POST" => Color::Blue,
        "PUT" => Color::Yellow,
        "DELETE" => Color::Red,
        "PATCH" => Color::Cyan,
        _ => Color::White,
    }
}

/// Helper to build render_items for grouped view
/// Returns the render_items that should be stored in state
pub fn build_grouped_render_items(state: &AppState) -> Vec<RenderItem> {
    let mut render_items = Vec::new();
    let grouped = state.active_grouped_endpoints();
    let mut group_names: Vec<&String> = grouped.keys().collect();
    group_names.sort();

    for group_name in group_names {
        let group_endpoints = &grouped[group_name];
        let is_expanded = state.expanded_groups.contains(group_name);

        render_items.push(RenderItem::GroupHeader {
            name: group_name.clone(),
            count: group_endpoints.len(),
            expanded: is_expanded,
        });

        if is_expanded {
            for endpoint in group_endpoints {
                render_items.push(RenderItem::Endpoint {
                    endpoint: endpoint.clone(),
                });
            }
        }
    }

    render_items
}

pub fn render_token_input_modal(frame: &mut Frame, state: &AppState) {
    use ratatui::widgets::Clear; // Add this import at the top of the file

    let area = frame.area();

    let modal_width = (area.width as f32 * 0.6).min(80.0) as u16;
    let modal_height = 7;
    let modal_x = (area.width.saturating_sub(modal_width)) / 2;
    let modal_y = (area.height.saturating_sub(modal_height)) / 2;

    let modal_area = Rect {
        x: modal_x,
        y: modal_y,
        width: modal_width,
        height: modal_height,
    };

    // Clear the background behind the modal - THIS IS THE FIX
    frame.render_widget(Clear, modal_area);

    // Create modal block
    let block = Block::default()
        .title(" Enter Bearer Token ")
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(Color::Rgb(30, 30, 30)).fg(Color::White));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    // Label
    let label = Paragraph::new("Token:").style(Style::default().fg(Color::LightCyan));
    frame.render_widget(label, chunks[0]);

    // Input field - show full token while editing
    let input = Paragraph::new(state.token_input.clone()).style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(input, chunks[1]);

    // Help text
    let help = Paragraph::new("Enter: Save  |  Esc: Cancel")
        .style(Style::default().fg(Color::Rgb(150, 150, 150)))
        .alignment(Alignment::Center);
    frame.render_widget(help, chunks[3]);
}

pub fn render_clear_confirmation_modal(frame: &mut Frame) {
    use ratatui::widgets::Clear; // Add this import at the top of the file

    let area = frame.area();

    let modal_width = (area.width as f32 * 0.5).min(60.0) as u16;
    let modal_height = 7;
    let modal_x = (area.width.saturating_sub(modal_width)) / 2;
    let modal_y = (area.height.saturating_sub(modal_height)) / 2;

    let modal_area = Rect {
        x: modal_x,
        y: modal_y,
        width: modal_width,
        height: modal_height,
    };

    // Clear the background behind the modal - THIS IS THE FIX
    frame.render_widget(Clear, modal_area);

    // Create modal block
    let block = Block::default()
        .title(" Clear Token? ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(Color::Rgb(30, 30, 30)).fg(Color::White));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    // Message
    let message = Paragraph::new("This will remove your authentication token.\nYou will need to re-enter it to make authenticated requests.")
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(message, chunks[0]);

    // Actions
    let actions = Paragraph::new("[Y] Yes, clear it  |  [N] Cancel")
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center);
    frame.render_widget(actions, chunks[2]);
}

pub fn render_url_input_modal(frame: &mut Frame, state: &AppState) {
    use ratatui::widgets::Clear;

    let area = frame.area();

    let modal_width = (area.width as f32 * 0.7).min(90.0) as u16;
    let modal_height = 12;
    let modal_x = (area.width.saturating_sub(modal_width)) / 2;
    let modal_y = (area.height.saturating_sub(modal_height)) / 2;

    let modal_area = Rect {
        x: modal_x,
        y: modal_y,
        width: modal_width,
        height: modal_height,
    };

    frame.render_widget(Clear, modal_area);

    let block = Block::default()
        .title(" Configure API URLs ")
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(Color::Rgb(30, 30, 30)).fg(Color::White));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Description
            Constraint::Length(1), // Swagger label
            Constraint::Length(1), // Swagger input
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Base URL label
            Constraint::Length(1), // Base URL input
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Help
        ])
        .split(inner);

    // Description
    let desc = Paragraph::new("Enter your Swagger/OpenAPI spec URL. The base URL will be auto-detected.\nExample: http://localhost:5000/swagger/v1/swagger.json")
        .style(Style::default().fg(Color::Gray))
        .wrap(Wrap { trim: true });
    frame.render_widget(desc, chunks[0]);

    // Determine active field styles
    use crate::types::UrlInputField;
    let swagger_active = state.active_url_field == UrlInputField::SwaggerUrl;
    let base_active = state.active_url_field == UrlInputField::BaseUrl;

    // Swagger URL label (with indicator if active)
    let swagger_label_text = if swagger_active {
        "► Swagger URL:"
    } else {
        "  Swagger URL:"
    };
    let swagger_label =
        Paragraph::new(swagger_label_text).style(Style::default().fg(if swagger_active {
            Color::Yellow
        } else {
            Color::LightCyan
        }));
    frame.render_widget(swagger_label, chunks[1]);

    // Swagger URL input (highlighted if active)
    let swagger_input = Paragraph::new(state.url_input.clone()).style(
        Style::default()
            .fg(if swagger_active {
                Color::Yellow
            } else {
                Color::Gray
            })
            .add_modifier(if swagger_active {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
    );
    frame.render_widget(swagger_input, chunks[2]);

    // Base URL label (with indicator if active)
    let base_label_text = if base_active {
        "► API Base URL:"
    } else {
        "  API Base URL:"
    };
    let base_label = Paragraph::new(base_label_text).style(Style::default().fg(if base_active {
        Color::Yellow
    } else {
        Color::LightCyan
    }));
    frame.render_widget(base_label, chunks[4]);

    // Base URL input (highlighted if active)
    let base_input = Paragraph::new(state.base_url_input.clone()).style(
        Style::default()
            .fg(if base_active {
                Color::Yellow
            } else {
                Color::Gray
            })
            .add_modifier(if base_active {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
    );
    frame.render_widget(base_input, chunks[5]);

    // Help text
    let help = Paragraph::new("Tab: Switch fields  |  Enter: Confirm  |  Esc: Cancel")
        .style(Style::default().fg(Color::Rgb(150, 150, 150)))
        .alignment(Alignment::Center);
    frame.render_widget(help, chunks[7]);
}

/// Attempts to pretty-print JSON, returns original string if not valid JSON
fn try_format_json(body: &str) -> String {
    // Try to parse as JSON
    match serde_json::from_str::<serde_json::Value>(body) {
        Ok(json) => {
            // Successfully parsed, pretty-print it
            serde_json::to_string_pretty(&json).unwrap_or_else(|_| body.to_string())
        }
        Err(_) => {
            // Not valid JSON, return as-is
            body.to_string()
        }
    }
}
