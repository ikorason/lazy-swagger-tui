use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

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

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?; // argument errors / panics with easy to read messages
    let terminal = ratatui::init();
    let app_result = App::default().run(terminal).await;
    ratatui::restore();
    app_result
}

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
struct AppState {
    endpoints: Vec<ApiEndpoint>,
    loading: bool,
    error: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            endpoints: Vec::new(),
            loading: true, // Start as loading
            error: None,
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
        }
    }
}

impl App {
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        // Spawn background task to fetch endpoints
        self.fetch_endpoints_background();

        // Main UI loop
        while !self.should_quit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }

        Ok(())
    }

    /// Spawns a background task to fetch endpoints
    fn fetch_endpoints_background(&self) {
        let state = Arc::clone(&self.state);
        let url = self.swagger_url.clone();

        // Spawn async task that runs in background
        tokio::spawn(async move {
            match fetch_swagger(&url).await {
                Ok(endpoints) => {
                    // Update shared state with fetched endpoints
                    if let Ok(mut state) = state.write() {
                        state.endpoints = endpoints;
                        state.loading = false;
                    }
                }
                Err(e) => {
                    // Update shared state with error
                    if let Ok(mut state) = state.write() {
                        state.error = Some(e.to_string());
                        state.loading = false;
                    }
                }
            }
        });
    }

    fn draw(&mut self, frame: &mut Frame) {
        // Read from shared state
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

        // Header
        let header_text = format!("dotREST - {}", self.swagger_url);
        let header = Paragraph::new(header_text)
            .style(Style::default().fg(Color::Cyan))
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(header, main_chunks[0]);

        // Left Panel - Endpoints List
        if state.loading {
            let loading = Paragraph::new("Loading endpoints...")
                .block(Block::default().borders(Borders::ALL).title("Endpoints"));
            frame.render_widget(loading, body_chunks[0]);
        } else if let Some(ref error) = state.error {
            let error_msg = Paragraph::new(format!("Error: {}", error))
                .style(Style::default().fg(Color::Red))
                .block(Block::default().borders(Borders::ALL).title("Endpoints"));
            frame.render_widget(error_msg, body_chunks[0]);
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

        // Right Panel
        let selected_endpoint = state.endpoints.get(self.selected_index);
        let details_text = if state.loading {
            "Loading...".to_string()
        } else if let Some(endpoint) = selected_endpoint {
            let summary = endpoint
                .summary
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("No description");

            format!(
                "{} {}\n\nSummary: {}\n\n─────────────────────────\n\nPress [Enter] to execute",
                endpoint.method, endpoint.path, summary
            )
        } else {
            "No endpoint selected".to_string()
        };

        let details = Paragraph::new(details_text).block(
            Block::default()
                .title("Details & Response")
                .borders(Borders::ALL),
        );

        frame.render_widget(details, body_chunks[1]);

        // Footer
        let footer = Paragraph::new("↑↓: Navigate | Enter: Execute | q: Quit")
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("Commands"));
        frame.render_widget(footer, main_chunks[2]);
    }

    fn handle_events(&mut self) -> Result<()> {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => self.should_quit = true,
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

async fn fetch_swagger(url: &str) -> Result<Vec<ApiEndpoint>> {
    let response = reqwest::get(url).await?;
    let spec: SwaggerSpec = response.json().await?;

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

    Ok(endpoints)
}
