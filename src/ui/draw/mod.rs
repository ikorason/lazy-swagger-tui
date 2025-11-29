//! UI drawing module
//!
//! This module is organized into focused submodules:
//! - `components`: Reusable UI components (header, footer, search bar, spinners)
//! - `modals`: Modal dialogs (URL input, token input, confirmation)
//! - `panels`: Main panels (endpoints list, details panel)
//! - `tabs`: Detail tabs (endpoint, request, headers, response)
//! - `styling`: Color schemes and style constants

mod components;
mod modals;
mod panels;
mod styling;
mod tabs;

// Re-export public API to maintain compatibility
pub use components::{render_footer, render_header, render_search_bar};
pub use modals::{
    render_body_input_modal, render_clear_confirmation_modal, render_token_input_modal,
    render_url_input_modal,
};
pub use panels::{render_details_panel, render_endpoints_panel};
pub use tabs::try_format_json;
