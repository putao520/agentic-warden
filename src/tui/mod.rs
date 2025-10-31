//! Terminal User Interface (TUI) module
//!
//! Provides unified TUI screens for all interactive features

pub mod app;
pub mod event;
pub mod screens;
pub mod widgets;

pub use app::TuiApp;
pub use event::{Event, EventHandler};
