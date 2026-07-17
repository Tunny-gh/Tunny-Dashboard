// Sidebar / panels
mod panels;
pub use panels::left_panel;
pub use panels::right_panel;
pub use panels::toolbar;

// Central drawing area
mod canvas;
pub use canvas::canvas_view;
pub use canvas::chart_cell;
pub use canvas::main_canvas;

// Chart pipeline
mod chart;
pub use chart::chart_registry;
pub use chart::poll_chart;
pub use chart::render_chart;

// Shared
pub mod help;
pub mod layout;
pub mod widget_states;
pub mod widgets;
