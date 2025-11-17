use crate::state::AppState;
use crate::swagger;
use crate::ui;
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
    swagger_url: String,
    spinner_index: usize,
    last_tick: Instant,
    event_handler: ui::EventHandler,
}

impl Default for App {
    fn default() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            state: Arc::new(RwLock::new(AppState::default())),
            list_state,
            swagger_url: "http://localhost:50002/swagger/v2/swagger.json".to_string(),
            spinner_index: 0,
            last_tick: Instant::now(),
            event_handler: ui::EventHandler::new(),
        }
    }
}

impl App {
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        // Start initial fetch
        self.fetch_endpoints_background();

        // Main UI loop
        while !self.event_handler.should_quit {
            // Update spinner animation
            if self.last_tick.elapsed().as_millis() > 100 {
                self.spinner_index = (self.spinner_index + 1) % 4;
                self.last_tick = Instant::now();
            }

            terminal.draw(|frame| self.draw(frame))?;

            // Clone what we need for the event handler
            let state = Arc::clone(&self.state);
            let should_fetch = self
                .event_handler
                .handle_events(state, &mut self.list_state)?;

            // If fetch was requested, do it
            if should_fetch {
                self.fetch_endpoints_background();
            }
        }

        Ok(())
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

        // Render header
        ui::render_header(
            frame,
            main_chunks[0],
            &self.swagger_url,
            &state.loading_state,
            state.endpoints.len(),
        );

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
            ui::render_footer(frame, main_chunks[2], &state.view_mode);
        } else {
            // In flat mode, just render remaining panels
            ui::render_details_panel(
                frame,
                body_chunks[1],
                &state,
                self.event_handler.selected_index,
            );

            ui::render_footer(frame, main_chunks[2], &state.view_mode);
        }
    }

    fn fetch_endpoints_background(&self) {
        swagger::fetch_endpoints_background(Arc::clone(&self.state), self.swagger_url.clone());
    }
}
