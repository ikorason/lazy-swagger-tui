//! Modal dialog rendering
//!
//! This module contains rendering functions for modal dialogs:
//! - URL configuration modal (Swagger URL + Base URL)
//! - Token input modal
//! - Clear confirmation modal

use crate::state::AppState;
use crate::types::UrlInputField;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

/// Render the token input modal for bearer authentication
pub fn render_token_input_modal(frame: &mut Frame, state: &AppState) {
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

    // Clear the background behind the modal
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
    let input = Paragraph::new(state.input.token_input.clone()).style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(input, chunks[1]);

    // Help text
    let help = Paragraph::new("Enter: Save  |  Ctrl+L: Clear  |  Esc: Cancel")
        .style(Style::default().fg(Color::Rgb(150, 150, 150)))
        .alignment(Alignment::Center);
    frame.render_widget(help, chunks[3]);
}

/// Render the clear token confirmation modal
pub fn render_clear_confirmation_modal(frame: &mut Frame) {
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

    // Clear the background behind the modal
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

/// Render the URL configuration modal (Swagger URL + Base URL)
pub fn render_url_input_modal(frame: &mut Frame, state: &AppState) {
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
    let desc = Paragraph::new("Swagger URL: for fetching endpoints  |  Base URL: for making API requests\nUse Tab to switch fields, Ctrl+L to clear")
        .style(Style::default().fg(Color::Gray))
        .wrap(Wrap { trim: true });
    frame.render_widget(desc, chunks[0]);

    // Determine active field styles
    let swagger_active = state.input.active_url_field == UrlInputField::SwaggerUrl;
    let base_active = state.input.active_url_field == UrlInputField::BaseUrl;

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
    let swagger_input = Paragraph::new(state.input.url_input.clone()).style(
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
    let base_input = Paragraph::new(state.input.base_url_input.clone()).style(
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
    let help = Paragraph::new(
        "Tab: Switch fields  |  Ctrl+L: Clear field  |  Enter: Confirm  |  Esc: Cancel",
    )
    .style(Style::default().fg(Color::Rgb(150, 150, 150)))
    .alignment(Alignment::Center);
    frame.render_widget(help, chunks[7]);
}

/// Render the JSON body input modal for POST/PUT/PATCH requests
pub fn render_body_input_modal(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    // Larger modal for multi-line JSON editing
    let modal_width = (area.width as f32 * 0.8).min(100.0) as u16;
    let modal_height = (area.height as f32 * 0.7).min(30.0) as u16;
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
        .title(" Edit Request Body (JSON) ")
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(Color::Rgb(30, 30, 30)).fg(Color::White));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Label
            Constraint::Min(5),    // Body content (grows)
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Help
        ])
        .split(inner);

    // Label
    let label = Paragraph::new("JSON Body:")
        .style(Style::default().fg(Color::LightGreen));
    frame.render_widget(label, chunks[0]);

    // Body input - multi-line with wrapping
    let body_text = Paragraph::new(state.input.body_input.clone())
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(body_text, chunks[1]);

    // Help text
    let help = Paragraph::new("Enter: Confirm & Format  |  Esc: Cancel  |  Ctrl+L: Clear")
        .style(Style::default().fg(Color::Rgb(150, 150, 150)))
        .alignment(Alignment::Center);
    frame.render_widget(help, chunks[3]);
}
