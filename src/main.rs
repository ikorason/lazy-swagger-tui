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
use std::io::Write;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use std::{
    collections::{HashMap, HashSet},
    fs::OpenOptions,
};

#[derive(Debug, Clone)]
struct ApiEndpoint {
    method: String,
    path: String,
    summary: Option<String>,
    tags: Vec<String>,
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
    tags: Option<Vec<String>>,
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
enum RenderItem {
    GroupHeader {
        name: String,
        count: usize,
        expanded: bool,
    },
    Endpoint {
        endpoint: ApiEndpoint,
    },
}

#[derive(Debug, Clone)]
struct AppState {
    endpoints: Vec<ApiEndpoint>, // original flat list
    loading_state: LoadingState,
    retry_count: u32,
    grouped_endpoints: HashMap<String, Vec<ApiEndpoint>>, // Grouped by tag/controllers
    view_mode: ViewMode,
    expanded_groups: HashSet<String>, // which groups are expanded
    render_items: Vec<RenderItem>,    // flattened list for rendering
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            endpoints: Vec::new(),
            loading_state: LoadingState::Idle,
            retry_count: 0,
            grouped_endpoints: HashMap::new(),
            view_mode: ViewMode::Grouped,
            expanded_groups: HashSet::new(),
            render_items: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ViewMode {
    Flat,    // show all endpoints in a flat list
    Grouped, // show grouped by controllers/tags with expandable
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

                            let mut grouped: HashMap<String, Vec<ApiEndpoint>> = HashMap::new();

                            for endpoint in &endpoints {
                                if endpoint.tags.is_empty() {
                                    // no tags? add to 'Other' group
                                    grouped
                                        .entry("Other".to_string())
                                        .or_default()
                                        .push(endpoint.clone());
                                } else {
                                    // has tags? add to each tag group
                                    for tag in &endpoint.tags {
                                        grouped
                                            .entry(tag.clone())
                                            .or_default()
                                            .push(endpoint.clone());
                                    }
                                }
                            }

                            // Step 3: Complete
                            if let Ok(mut s) = state.write() {
                                s.endpoints = endpoints;
                                s.grouped_endpoints = grouped;
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
                    match &state.view_mode {
                        ViewMode::Flat => {
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

                            frame.render_stateful_widget(
                                list,
                                body_chunks[0],
                                &mut self.list_state,
                            );
                        }

                        ViewMode::Grouped => {
                            let mut items: Vec<ListItem> = Vec::new();
                            let mut render_items: Vec<RenderItem> = Vec::new();

                            let mut group_names: Vec<&String> =
                                state.grouped_endpoints.keys().collect();
                            group_names.sort();

                            for group_name in group_names {
                                let group_endpoints = &state.grouped_endpoints[group_name];
                                let is_expanded = state.expanded_groups.contains(group_name);

                                // create group header line
                                let icon = if is_expanded { "▼" } else { "▶" };
                                let header_line =
                                    format!("{} {} ({})", icon, group_name, group_endpoints.len());

                                // style the group header
                                let header_item = ListItem::new(Line::from(Span::styled(
                                    header_line,
                                    Style::default()
                                        .fg(Color::Yellow)
                                        .add_modifier(Modifier::BOLD),
                                )));
                                items.push(header_item);

                                // track that this line is a group header
                                render_items.push(RenderItem::GroupHeader {
                                    name: group_name.clone(),
                                    count: group_endpoints.len(),
                                    expanded: is_expanded,
                                });

                                // If expanded, add endpoints under this group
                                if is_expanded {
                                    for endpoint in group_endpoints {
                                        let method_color = match endpoint.method.as_str() {
                                            "GET" => Color::Green,
                                            "POST" => Color::Blue,
                                            "PUT" => Color::Yellow,
                                            "DELETE" => Color::Red,
                                            "PATCH" => Color::Cyan,
                                            _ => Color::White,
                                        };

                                        // indent endpoints under group (use spaces)
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

                            // render the list
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

                            frame.render_stateful_widget(
                                list,
                                body_chunks[0],
                                &mut self.list_state,
                            );

                            drop(state); // release read lock
                            let mut state_write = self.state.write().unwrap(); // get write lock
                            state_write.render_items = render_items;
                            drop(state_write); // release write lock immediately
                        }
                    }
                }
            }
        }

        // re-acquire state for details panel (in case it was dropped in grouped rendering)
        let state = self.state.read().unwrap();

        // Right Panel - Get the correct endpoint based on view mode
        let selected_endpoint = match state.view_mode {
            ViewMode::Flat => {
                // In flat mode, use endpoints array directly
                state.endpoints.get(self.selected_index)
            }
            ViewMode::Grouped => {
                // In grouped mode, extract endpoint from render_items
                state
                    .render_items
                    .get(self.selected_index)
                    .and_then(|item| match item {
                        RenderItem::Endpoint { endpoint } => Some(endpoint),
                        RenderItem::GroupHeader { .. } => None,
                    })
            }
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

        frame.render_widget(details, body_chunks[1]);

        let footer_text = match state.view_mode {
            ViewMode::Flat => {
                "↑↓: Navigate | Enter: Execute | G: Group | F5: Refresh | R: Retry | q: Quit"
            }
            ViewMode::Grouped => {
                "↑↓: Navigate | Enter: Expand/Execute | G: Ungroup | F5: Refresh | q: Quit"
            }
        };

        // Footer with more commands
        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("Commands"));

        frame.render_widget(footer, main_chunks[2]);
    }

    /// Helper function to count visible items in current view mode
    fn count_visible_items(&self, state: &AppState) -> usize {
        match state.view_mode {
            ViewMode::Flat => state.endpoints.len(),
            ViewMode::Grouped => {
                let mut count = 0;
                let mut group_names: Vec<&String> = state.grouped_endpoints.keys().collect();
                group_names.sort();

                for group_name in group_names {
                    count += 1; // Group header
                    if state.expanded_groups.contains(group_name) {
                        let endpoints = &state.grouped_endpoints[group_name];
                        count += endpoints.len(); // Endpoints in expanded group
                    }
                }
                count
            }
        }
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
                    KeyCode::Char('g') | KeyCode::Char('G') => {
                        let mut state = self.state.write().unwrap();

                        // Toggle view mode
                        state.view_mode = match state.view_mode {
                            ViewMode::Flat => ViewMode::Grouped,
                            ViewMode::Grouped => ViewMode::Flat,
                        };

                        // Reset selection to top
                        self.selected_index = 0;
                        self.list_state.select(Some(0));

                        log_debug(&format!("Switched to {:?} mode", state.view_mode));
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

                        // Use render_items length in grouped mode, endpoints length in flat mode
                        let max_index = match state.view_mode {
                            ViewMode::Flat => state.endpoints.len().saturating_sub(1),
                            ViewMode::Grouped => state.render_items.len().saturating_sub(1),
                        };

                        if self.selected_index < max_index {
                            self.selected_index += 1;
                            self.list_state.select(Some(self.selected_index));
                        }
                    }
                    KeyCode::Enter => {
                        let state = self.state.read().unwrap();

                        // Check what view mode we're in
                        if state.view_mode == ViewMode::Flat {
                            // In flat mode: Execute request
                            // TODO: Execute request for selected endpoint
                            log_debug("Execute request in flat mode");
                        } else {
                            // In grouped mode: Check if we're on a group header or endpoint
                            if let Some(item) = state.render_items.get(self.selected_index) {
                                match item {
                                    RenderItem::GroupHeader { name, .. } => {
                                        let groupd_name = name.clone();

                                        drop(state); // Release read lock
                                        let mut state = self.state.write().unwrap();

                                        if state.expanded_groups.contains(&groupd_name) {
                                            state.expanded_groups.remove(&groupd_name);
                                            log_debug(&format!("Collapsed group: {}", groupd_name));
                                        } else {
                                            state.expanded_groups.insert(groupd_name.clone());
                                            log_debug(&format!("Expanded group: {}", groupd_name));
                                        }

                                        // Validate selection is still in bounds
                                        drop(state);
                                        let state = self.state.read().unwrap();

                                        let visible_count = self.count_visible_items(&state);
                                        if self.selected_index >= visible_count {
                                            self.selected_index = visible_count.saturating_sub(1);
                                            self.list_state.select(Some(self.selected_index));
                                        }
                                    }
                                    RenderItem::Endpoint { endpoint } => {
                                        // Execute request for this endpoint
                                        log_debug(&format!(
                                            "Execute: {} {}",
                                            endpoint.method, endpoint.path
                                        ));
                                        // TODO: Actually execute request
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

fn parse_swagger_spec(spec: SwaggerSpec) -> Vec<ApiEndpoint> {
    let mut endpoints: Vec<ApiEndpoint> = Vec::new();

    for (path, path_item) in spec.paths {
        if let Some(op) = &path_item.get {
            endpoints.push(ApiEndpoint {
                method: "GET".to_string(),
                path: path.clone(),
                summary: op.summary.clone(),
                tags: op.tags.clone().unwrap_or_default(),
            });
        }
        if let Some(op) = &path_item.post {
            endpoints.push(ApiEndpoint {
                method: "POST".to_string(),
                path: path.clone(),
                summary: op.summary.clone(),
                tags: op.tags.clone().unwrap_or_default(),
            });
        }
        if let Some(op) = &path_item.put {
            endpoints.push(ApiEndpoint {
                method: "PUT".to_string(),
                path: path.clone(),
                summary: op.summary.clone(),
                tags: op.tags.clone().unwrap_or_default(),
            });
        }
        if let Some(op) = &path_item.delete {
            endpoints.push(ApiEndpoint {
                method: "DELETE".to_string(),
                path: path.clone(),
                summary: op.summary.clone(),
                tags: op.tags.clone().unwrap_or_default(),
            });
        }
        if let Some(op) = &path_item.patch {
            endpoints.push(ApiEndpoint {
                method: "PATCH".to_string(),
                path: path.clone(),
                summary: op.summary.clone(),
                tags: op.tags.clone().unwrap_or_default(),
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

fn log_debug(msg: &str) {
    let _ = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/dotrest.log")
        .and_then(|mut f| writeln!(f, "{}", msg));
}
