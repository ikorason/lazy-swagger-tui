use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

#[derive(Debug, Clone)]
struct ApiEndpoint {
    method: String,
    path: String,
    summary: Option<String>,
}

#[derive(Deserialize)]
struct SwaggerSpec {
    paths: HashMap<String, PathItem>,
}

#[derive(Deserialize)]
struct PathItem {
    get: Option<Operation>,
    post: Option<Operation>,
    put: Option<Operation>,
    delete: Option<Operation>,
    patch: Option<Operation>,
}

#[derive(Deserialize)]
struct Operation {
    summary: Option<String>,
}

#[derive(Debug, Clone)]
enum LoadingState {
    Idle,
    Fetching,
    Parsing,
    Complete,
    Error(String),
}

#[derive(Debug, Clone)]
struct AppState {
    endpoints: Vec<ApiEndpoint>,
    loading_state: LoadingState,
    retry_count: u32,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            endpoints: Vec::new(),
            loading_state: LoadingState::Idle,
            retry_count: 0,
        }
    }
}

#[derive(Debug)]
struct App {
    should_quit: bool,
    state: Arc<RwLock<AppState>>,
    selected_index: usize,
    list_state: ListState,
    swagger_url: String,
    spinner_index: usize,
    last_tick: Instant,
}

impl Default for App {
    fn default() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            should_quit: false,
            state: Arc::new(RwLock::new(AppState::default())),
            selected_index: 0,
            list_state,
            swagger_url: "http://localhost:50002/swagger/v2/swagger.json".to_string(),
            spinner_index: 0,
            last_tick: Instant::now(),
        }
    }
}

impl App {
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        // Start initial fetch
        self.fetch_endpoints_background();

        // Main UI loop
        while !self.should_quit {
            // Update spinner animation
            if self.last_tick.elapsed().as_millis() > 100 {
                self.spinner_index = (self.spinner_index + 1) % 4;
                self.last_tick = Instant::now();
            }

            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }

        Ok(())
    }

    /// Spawns a background task to fetch endpoints
    fn fetch_endpoints_background(&self) {
        let state = Arc::clone(&self.state);
        let url = self.swagger_url.clone();

        // Set loading state
        if let Ok(mut s) = state.write() {
            s.loading_state = LoadingState::Fetching;
        }

        tokio::spawn(async move {
            // Step 1: Fetching
            match reqwest::get(&url).await {
                Ok(response) => {
                    // Step 2: Parsing
                    if let Ok(mut s) = state.write() {
                        s.loading_state = LoadingState::Parsing;
                    }

                    match response.json::<SwaggerSpec>().await {
                        Ok(spec) => {
                            // Parse endpoints
                            let endpoints = parse_swagger_spec(spec);

                            // Step 3: Complete
                            if let Ok(mut s) = state.write() {
                                s.endpoints = endpoints;
                                s.loading_state = LoadingState::Complete;
                                s.retry_count = 0;
                            }
                        }
                        Err(e) => {
                            if let Ok(mut s) = state.write() {
                                s.loading_state =
                                    LoadingState::Error(format!("Parse error: {}", e));
                            }
                        }
                    }
                }
                Err(e) => {
                    if let Ok(mut s) = state.write() {
                        s.loading_state = LoadingState::Error(format!("Network error: {}", e));
                    }
                }
            }
        });
    }

    /// Retry fetching with exponential backoff
    fn retry_fetch(&self) {
        let state = Arc::clone(&self.state);

        // Increment retry count
        if let Ok(mut s) = state.write() {
            s.retry_count += 1;
        }

        self.fetch_endpoints_background();
    }

    fn draw(&mut self, frame: &mut Frame) {
        let state = self.state.read().unwrap();

        // Create main layout: Header, Body, Footer
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(frame.area());

        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(main_chunks[1]);

        // Header with status
        let status_text = match &state.loading_state {
            LoadingState::Idle => "Idle".to_string(),
            LoadingState::Fetching => "Fetching...".to_string(),
            LoadingState::Parsing => "Parsing...".to_string(),
            LoadingState::Complete => format!("{} endpoints loaded", state.endpoints.len()),
            LoadingState::Error(_) => "Error".to_string(),
        };
        let header_text = format!("dotREST - {} [{}]", self.swagger_url, status_text);
        let header = Paragraph::new(header_text)
            .style(Style::default().fg(Color::Cyan))
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(header, main_chunks[0]);

        // Left Panel - Endpoints List with spinner
        match &state.loading_state {
            LoadingState::Fetching | LoadingState::Parsing => {
                let spinner = ["⠋", "⠙", "⠹", "⠸"];
                let progress_text = match &state.loading_state {
                    LoadingState::Fetching => "Fetching swagger.json",
                    LoadingState::Parsing => "Parsing endpoints",
                    _ => "",
                };
                let loading_text = format!(
                    "{} {}\n\nPlease wait...",
                    spinner[self.spinner_index], progress_text
                );
                let loading = Paragraph::new(loading_text)
                    .style(Style::default().fg(Color::Yellow))
                    .block(Block::default().borders(Borders::ALL).title("Endpoints"));
                frame.render_widget(loading, body_chunks[0]);
            }
            LoadingState::Error(error) => {
                let retry_text = if state.retry_count > 0 {
                    format!("\n\nRetry attempt: {}", state.retry_count)
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
                frame.render_widget(error_widget, body_chunks[0]);
            }
            LoadingState::Complete | LoadingState::Idle => {
                if state.endpoints.is_empty() {
                    let empty = Paragraph::new("No endpoints found\n\nPress [F5] to refresh")
                        .block(Block::default().borders(Borders::ALL).title("Endpoints"));
                    frame.render_widget(empty, body_chunks[0]);
                } else {
                    let items: Vec<ListItem> = state
                        .endpoints
                        .iter()
                        .map(|endpoint| {
                            let method_color = match endpoint.method.as_str() {
                                "GET" => Color::Green,
                                "POST" => Color::Blue,
                                "PUT" => Color::Yellow,
                                "DELETE" => Color::Red,
                                "PATCH" => Color::Cyan,
                                _ => Color::White,
                            };

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

                    frame.render_stateful_widget(list, body_chunks[0], &mut self.list_state);
                }
            }
        }

        // Right Panel
        let selected_endpoint = state.endpoints.get(self.selected_index);
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

        frame.render_widget(details, body_chunks[1]);

        // Footer with more commands
        let footer =
            Paragraph::new("↑↓: Navigate | Enter: Execute | F5: Refresh | R: Retry | q: Quit")
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL).title("Commands"));
        frame.render_widget(footer, main_chunks[2]);
    }

    fn handle_events(&mut self) -> Result<()> {
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => self.should_quit = true,
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        // Retry on error
                        let state = self.state.read().unwrap();
                        if matches!(state.loading_state, LoadingState::Error(_)) {
                            drop(state); // Release read lock
                            self.retry_fetch();
                        }
                    }
                    KeyCode::F(5) => {
                        // Refresh - refetch endpoints
                        self.fetch_endpoints_background();
                    }
                    KeyCode::Up => {
                        if self.selected_index > 0 {
                            self.selected_index -= 1;
                            self.list_state.select(Some(self.selected_index));
                        }
                    }
                    KeyCode::Down => {
                        let state = self.state.read().unwrap();
                        if self.selected_index < state.endpoints.len().saturating_sub(1) {
                            self.selected_index += 1;
                            self.list_state.select(Some(self.selected_index));
                        }
                    }
                    KeyCode::Enter => {
                        // TODO: Execute request in background
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

fn parse_swagger_spec(spec: SwaggerSpec) -> Vec<ApiEndpoint> {
    let mut endpoints = Vec::new();

    for (path, path_item) in spec.paths {
        if let Some(op) = &path_item.get {
            endpoints.push(ApiEndpoint {
                method: "GET".to_string(),
                path: path.clone(),
                summary: op.summary.clone(),
            });
        }
        if let Some(op) = &path_item.post {
            endpoints.push(ApiEndpoint {
                method: "POST".to_string(),
                path: path.clone(),
                summary: op.summary.clone(),
            });
        }
        if let Some(op) = &path_item.put {
            endpoints.push(ApiEndpoint {
                method: "PUT".to_string(),
                path: path.clone(),
                summary: op.summary.clone(),
            });
        }
        if let Some(op) = &path_item.delete {
            endpoints.push(ApiEndpoint {
                method: "DELETE".to_string(),
                path: path.clone(),
                summary: op.summary.clone(),
            });
        }
        if let Some(op) = &path_item.patch {
            endpoints.push(ApiEndpoint {
                method: "PATCH".to_string(),
                path: path.clone(),
                summary: op.summary.clone(),
            });
        }
    }

    endpoints
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let app_result = App::default().run(terminal).await;
    ratatui::restore();
    app_result
}
