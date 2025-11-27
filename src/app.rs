use crate::swagger;
use crate::types::InputMode;
use crate::ui;
use crate::ui::draw;
use crate::{config::Config, state::AppState};
use color_eyre::Result;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
    widgets::ListState,
};
use std::sync::{Arc, RwLock};
use std::time::Instant;

#[derive(Debug)]
pub struct App {
    state: Arc<RwLock<AppState>>,
    list_state: ListState,
    swagger_url: Option<String>,
    base_url: Option<String>,
    spinner_index: usize,
    last_tick: Instant,
    event_handler: ui::EventHandler,
    config: Config,
}

impl Default for App {
    fn default() -> Self {
        let mut list_state = ListState::default();
        list_state.select(None);

        // Load config
        let config = Config::load().unwrap();
        let swagger_url = config.server.swagger_url.clone();
        let base_url = config.server.base_url.clone();

        // Determine initial input mode
        let initial_input_mode = if swagger_url.is_none() {
            InputMode::EnteringUrl // Show URL modal if no config
        } else {
            InputMode::Normal
        };

        let state = AppState {
            input_mode: initial_input_mode,
            ..Default::default()
        };

        Self {
            state: Arc::new(RwLock::new(state)),
            list_state,
            swagger_url,
            base_url,
            spinner_index: 0,
            last_tick: Instant::now(),
            event_handler: ui::EventHandler::new(),
            config,
        }
    }
}

impl App {
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        // Only fetch if we have a URL
        if self.swagger_url.is_some() {
            self.fetch_endpoints_background();
        }

        // Main UI loop
        while !self.event_handler.should_quit {
            // Update spinner animation
            if self.last_tick.elapsed().as_millis() > 100 {
                self.spinner_index = (self.spinner_index + 1) % 4;
                self.last_tick = Instant::now();
            }

            terminal.draw(|frame| self.draw(frame))?;

            let state = Arc::clone(&self.state);
            let (should_fetch, url_submitted) = self.event_handler.handle_events(
                state,
                &mut self.list_state,
                self.base_url.clone(),
                self.swagger_url.clone(),
            )?;

            // If URL was submitted, save it and start fetching
            if let Some(submission) = url_submitted {
                self.swagger_url = Some(submission.swagger_url.clone());
                self.base_url = submission.base_url.clone();
                self.config
                    .set_swagger_url(submission.swagger_url, submission.base_url)?;
                self.fetch_endpoints_background();
            } else if should_fetch {
                self.fetch_endpoints_background();
            }
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        // Check if we need to initialize selection (do this before acquiring lock)
        let should_select = self.list_state.selected().is_none();

        let state = self.state.read().unwrap();

        // Create main layout: Header, Search Bar, Body, Footer
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Length(3), // Search bar
                Constraint::Min(0),    // Body
                Constraint::Length(3), // Footer
            ])
            .split(frame.area());

        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(main_chunks[2]); // Changed from [1] to [2]

        let display_url = self.swagger_url.as_deref().unwrap_or("No URL configured");

        // Render header
        ui::render_header(
            frame,
            main_chunks[0],
            display_url,
            &state.loading_state,
            state.endpoints.len(),
            &state.auth,
        );

        // Render search bar
        ui::render_search_bar(frame, main_chunks[1], &state);

        // Ensure we have a selection if items exist
        if should_select {
            let has_items = match state.view_mode {
                crate::types::ViewMode::Flat => !state.endpoints.is_empty(),
                crate::types::ViewMode::Grouped => !state.render_items.is_empty(),
            };

            if has_items {
                self.list_state.select(Some(0));
            }
        }

        // Render left panel (endpoints list)
        ui::render_endpoints_panel(
            frame,
            body_chunks[0],
            &state,
            self.spinner_index,
            &mut self.list_state,
        );

        // After rendering grouped view, update render_items if needed
        if matches!(state.view_mode, crate::types::ViewMode::Grouped) {
            drop(state); // Release read lock
            let render_items = ui::build_grouped_render_items(&self.state.read().unwrap());
            let mut state_write = self.state.write().unwrap();
            state_write.render_items = render_items;
            drop(state_write);

            // Re-acquire read lock for remaining rendering
            let state = self.state.read().unwrap();

            // Render right panel (details)
            ui::render_details_panel(
                frame,
                body_chunks[1],
                &state,
                self.event_handler.selected_index,
            );

            // Render footer
            ui::render_footer(frame, main_chunks[3], &state.view_mode);
        } else {
            // In flat mode, just render remaining panels
            ui::render_details_panel(
                frame,
                body_chunks[1],
                &state,
                self.event_handler.selected_index,
            );

            ui::render_footer(frame, main_chunks[3], &state.view_mode);
        }

        // Render modals LAST - after everything else (re-borrow state if needed)
        let state = self.state.read().unwrap();
        match state.input_mode {
            InputMode::EnteringUrl => {
                draw::render_url_input_modal(frame, &state);
            }
            InputMode::EnteringToken => {
                draw::render_token_input_modal(frame, &state);
            }
            InputMode::ConfirmClearToken => {
                draw::render_clear_confirmation_modal(frame);
            }
            InputMode::Normal | InputMode::Searching => {}
        }
    }

    fn fetch_endpoints_background(&self) {
        if let Some(url) = &self.swagger_url {
            swagger::fetch_endpoints_background(Arc::clone(&self.state), url.clone());
        }
    }
}
