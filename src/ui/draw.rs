use crate::state::AppState;
use crate::types::{ApiEndpoint, AuthState, DetailsPanelFocus, LoadingState, RenderItem, ViewMode};
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
        "dotREST - {} [{}] | {}",
        swagger_url, status_text, auth_status
    );

    let header = Paragraph::new(header_text)
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(header, area);
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
            if state.endpoints.is_empty() {
                render_empty_message(frame, area);
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

fn render_flat_list(frame: &mut Frame, area: Rect, state: &AppState, list_state: &mut ListState) {
    let items: Vec<ListItem> = state
        .endpoints
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
                .title(format!("Endpoints ({})", state.endpoints.len()))
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
                    state.grouped_endpoints.len()
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
    // Get the selected endpoint (same logic as before)
    let selected_endpoint = match state.view_mode {
        ViewMode::Flat => state.endpoints.get(selected_index),
        ViewMode::Grouped => state
            .render_items
            .get(selected_index)
            .and_then(|item| match item {
                RenderItem::Endpoint { endpoint } => Some(endpoint),
                RenderItem::GroupHeader { .. } => None,
            }),
    };

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

    let Some(endpoint) = selected_endpoint else {
        let empty = Paragraph::new("No endpoint selected");
        frame.render_widget(empty, inner_area);
        return;
    };

    // Split into three sections: Details (auto), Response (main), Headers (auto)
    // The response body gets the "remaining" space
    render_three_section_layout(frame, inner_area, endpoint, state);
}

fn render_three_section_layout(
    frame: &mut Frame,
    area: Rect,
    endpoint: &ApiEndpoint,
    state: &AppState,
) {
    let sections = &state.response_sections_expanded;

    // Calculate heights for each section
    let details_height = if sections.endpoint_details {
        calculate_details_section_height(endpoint)
    } else {
        1 // Just the header
    };

    let headers_height = if sections.response_headers {
        calculate_headers_section_height(state)
    } else {
        1 // Just the header
    };

    // Calculate response height based on expanded state
    let response_constraint = if sections.response_body {
        Constraint::Min(10) // Expanded: take remaining space, min 10 lines
    } else {
        Constraint::Length(1) // Collapsed: just the header line
    };

    // Create layout: Details (fixed), Response (flexible), Headers (fixed)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(details_height),
            response_constraint,
            Constraint::Length(headers_height),
        ])
        .split(area);

    // Render each section
    render_details_section(frame, chunks[0], endpoint, sections.endpoint_details, state);
    render_response_section(frame, chunks[1], endpoint, state);
    render_headers_section(frame, chunks[2], state);
}

/// Calculate how many lines the details section needs
fn calculate_details_section_height(endpoint: &ApiEndpoint) -> u16 {
    let mut height = 1; // Header line

    if endpoint.summary.is_some() {
        height += 3; // Method line + Summary line + spacing
    } else {
        height += 2; // Method line + spacing
    }

    height
}

/// Calculate how many lines the headers section needs
fn calculate_headers_section_height(state: &AppState) -> u16 {
    let header_count = state
        .current_response
        .as_ref()
        .map(|r| r.headers.len())
        .unwrap_or(0);

    if header_count == 0 {
        return 2; // Header + "No headers" message
    }

    // Header line + headers + some padding, max 15 lines
    let height = 1 + header_count + 1;
    height.min(15) as u16
}

/// Render the endpoint details section
fn render_details_section(
    frame: &mut Frame,
    area: Rect,
    endpoint: &ApiEndpoint,
    is_expanded: bool,
    state: &AppState,
) {
    let mut lines: Vec<Line> = Vec::new();

    // Header
    let icon = if is_expanded { "▼" } else { "▶" };
    let is_focused = state.details_focus == DetailsPanelFocus::EndpointDetails;

    lines.push(Line::from(vec![Span::styled(
        format!("{} Endpoint Details", icon),
        Style::default()
            .fg(if is_focused {
                Color::Cyan
            } else {
                Color::Yellow
            })
            .add_modifier(Modifier::BOLD),
    )]));

    if is_expanded {
        let method_color = get_method_color(&endpoint.method);
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                &endpoint.method,
                Style::default()
                    .fg(method_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::raw(&endpoint.path),
        ]));

        if let Some(summary) = &endpoint.summary {
            lines.push(Line::from(vec![
                Span::raw("  Summary: "),
                Span::raw(summary),
            ]));
        }
    }

    let content = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(content, area);
}

fn render_response_section(
    frame: &mut Frame,
    area: Rect,
    endpoint: &ApiEndpoint,
    state: &AppState,
) {
    let sections = &state.response_sections_expanded;

    // Split area into: header line + content area
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header line
            Constraint::Min(0),    // Content area
        ])
        .split(area);

    // Render header (always visible)
    let is_executing = state.executing_endpoint.as_ref() == Some(&endpoint.path);
    let is_focused = state.details_focus == DetailsPanelFocus::ResponseBody;

    let icon = if sections.response_body { "▼" } else { "▶" };

    let response_title = if let Some(ref resp) = state.current_response {
        if resp.is_error {
            format!("{} Response (Error)", icon)
        } else {
            format!(
                "{} Response ({} {} - {}ms)",
                icon,
                resp.status,
                resp.status_text,
                resp.duration.as_millis()
            )
        }
    } else if is_executing {
        format!("{} Response (Executing...)", icon)
    } else {
        format!("{} Response", icon)
    };

    let header = Paragraph::new(Line::from(vec![Span::styled(
        response_title,
        Style::default()
            .fg(if is_focused {
                Color::Cyan
            } else {
                Color::Yellow
            })
            .add_modifier(Modifier::BOLD),
    )]));

    frame.render_widget(header, chunks[0]);

    // Build content lines (without header)
    let mut content_lines: Vec<Line> = Vec::new();

    if sections.response_body {
        if is_executing {
            content_lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("⏳ Executing request...", Style::default().fg(Color::Cyan)),
            ]));
        } else if let Some(ref response) = state.current_response {
            if response.is_error {
                content_lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "❌ Error",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ),
                ]));
                content_lines.push(Line::from(""));

                if let Some(ref err_msg) = response.error_message {
                    for line in err_msg.lines() {
                        content_lines.push(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(line.to_string(), Style::default().fg(Color::Red)),
                        ]));
                    }
                }
            } else {
                let formatted_body = try_format_json(&response.body);
                for line in formatted_body.lines() {
                    content_lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::raw(line.to_string()),
                    ]));
                }
            }
        } else {
            content_lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "Press [Space] to execute request",
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
    }

    // Render scrollable content
    let content = Paragraph::new(content_lines)
        .wrap(Wrap { trim: false })
        .scroll((state.response_body_scroll as u16, 0));

    frame.render_widget(content, chunks[1]);
}

fn render_headers_section(frame: &mut Frame, area: Rect, state: &AppState) {
    let sections = &state.response_sections_expanded;

    // Split area into: header line + content area
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header line
            Constraint::Min(0),    // Content area
        ])
        .split(area);

    // Render header (always visible)
    let is_focused = state.details_focus == DetailsPanelFocus::Headers;

    let icon = if sections.response_headers {
        "▼"
    } else {
        "▶"
    };

    let headers_count = state
        .current_response
        .as_ref()
        .map(|r| r.headers.len())
        .unwrap_or(0);

    let headers_title = if headers_count > 0 {
        format!("{} Headers ({})", icon, headers_count)
    } else {
        format!("{} Headers", icon)
    };

    let header = Paragraph::new(Line::from(vec![Span::styled(
        headers_title,
        Style::default()
            .fg(if is_focused {
                Color::Cyan
            } else {
                if headers_count > 0 {
                    Color::Yellow
                } else {
                    Color::DarkGray
                }
            })
            .add_modifier(Modifier::BOLD),
    )]));

    frame.render_widget(header, chunks[0]);

    // Build content lines (without header)
    let mut content_lines: Vec<Line> = Vec::new();

    if sections.response_headers {
        if let Some(ref response) = state.current_response {
            if !response.headers.is_empty() {
                let mut header_vec: Vec<_> = response.headers.iter().collect();
                header_vec.sort_by_key(|(k, _)| k.as_str());

                for (key, value) in header_vec {
                    content_lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(format!("{}: ", key), Style::default().fg(Color::Cyan)),
                        Span::raw(value.to_string()),
                    ]));
                }
            } else {
                content_lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("No headers", Style::default().fg(Color::DarkGray)),
                ]));
            }
        } else {
            content_lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("No response yet", Style::default().fg(Color::DarkGray)),
            ]));
        }
    }

    // Render scrollable content
    let content = Paragraph::new(content_lines)
        .wrap(Wrap { trim: false })
        .scroll((state.headers_scroll as u16, 0));

    frame.render_widget(content, chunks[1]);
}

pub fn render_footer(frame: &mut Frame, area: Rect, view_mode: &ViewMode) {
    let footer_text = match view_mode {
        ViewMode::Flat => {
            "Tab:Panel j/k:Nav Space:Execute/Toggle Ctrl+d/u:Scroll | G:Group ,:URL a:Auth q:Quit"
        }
        ViewMode::Grouped => {
            "Tab:Panel j/k:Nav Space:Execute/Toggle Ctrl+d/u:Scroll | G:Ungroup ,:URL a:Auth q:Quit"
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
    let mut group_names: Vec<&String> = state.grouped_endpoints.keys().collect();
    group_names.sort();

    for group_name in group_names {
        let group_endpoints = &state.grouped_endpoints[group_name];
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
