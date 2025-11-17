use crate::state::AppState;
use crate::types::{AuthState, LoadingState, RenderItem, ViewMode};
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

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!("Endpoints ({})", state.endpoints.len()))
                .borders(Borders::ALL),
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
    let mut items: Vec<ListItem> = Vec::new();
    let mut render_items: Vec<RenderItem> = Vec::new();

    let mut group_names: Vec<&String> = state.grouped_endpoints.keys().collect();
    group_names.sort();

    for group_name in group_names {
        let group_endpoints = &state.grouped_endpoints[group_name];
        let is_expanded = state.expanded_groups.contains(group_name);

        // Group header
        let icon = if is_expanded { "▼" } else { "▶" };
        let header_line = format!("{} {} ({})", icon, group_name, group_endpoints.len());

        let header_item = ListItem::new(Line::from(Span::styled(
            header_line,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        items.push(header_item);

        render_items.push(RenderItem::GroupHeader {
            name: group_name.clone(),
            count: group_endpoints.len(),
            expanded: is_expanded,
        });

        // If expanded, show endpoints
        if is_expanded {
            for endpoint in group_endpoints {
                let method_color = get_method_color(&endpoint.method);

                let line = Line::from(vec![
                    Span::raw("  "), // Indentation
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

                render_items.push(RenderItem::Endpoint {
                    endpoint: endpoint.clone(),
                });
            }
        }
    }

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(
                    "Endpoints - {} groups",
                    state.grouped_endpoints.len()
                ))
                .borders(Borders::ALL),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, area, list_state);

    // Store render_items after rendering
    // Note: This is handled in App::draw() after this function returns
}

pub fn render_details_panel(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    selected_index: usize,
) {
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

    let details_text = match &state.loading_state {
        LoadingState::Fetching | LoadingState::Parsing => "Loading...".to_string(),
        LoadingState::Error(e) => format!("Error loading endpoints:\n\n{}", e),
        _ => {
            if let Some(endpoint) = selected_endpoint {
                let summary = endpoint
                    .summary
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("No description");

                format!(
                    "{} {}\n\nSummary: {}\n\nParameters: None\n\n─────────────────────────\n\nPress [Enter] to execute request",
                    endpoint.method, endpoint.path, summary
                )
            } else {
                "No endpoint selected".to_string()
            }
        }
    };

    let details = Paragraph::new(details_text).block(
        Block::default()
            .title("Details & Response")
            .borders(Borders::ALL),
    );

    frame.render_widget(details, area);
}

pub fn render_footer(frame: &mut Frame, area: Rect, view_mode: &ViewMode) {
    let footer_text = match view_mode {
        ViewMode::Flat => {
            "↑↓: Navigate | Enter: Execute | G: Group | F5: Refresh | R: Retry | q: Quit"
        }
        ViewMode::Grouped => {
            "↑↓: Navigate | Enter: Expand/Execute | G: Ungroup | F5: Refresh | q: Quit"
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

    // Input field
    let display_token = if state.token_input.len() > 50 {
        format!(
            "{}...{}",
            &state.token_input[..20],
            &state.token_input[state.token_input.len() - 20..]
        )
    } else {
        state.token_input.clone()
    };

    let input = Paragraph::new(display_token).style(
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
