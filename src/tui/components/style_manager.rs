//! Style Manager - Centralized style definitions for TUI components
//!
//! Eliminates repeated style definitions across screens, providing consistent
//! styling and reducing the 66+ repeated style creation calls.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Centralized style manager for consistent TUI styling
pub struct StyleManager;

impl StyleManager {
    /// Create title style (cyan, bold)
    pub fn title() -> Style {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    }

    /// Create header style (cyan, bold)
    pub fn header() -> Style {
        Self::title()
    }

    /// Create error style (light red)
    pub fn error() -> Style {
        Style::default().fg(Color::LightRed)
    }

    /// Create warning style (yellow)
    pub fn warning() -> Style {
        Style::default().fg(Color::Yellow)
    }

    /// Create success style (green)
    pub fn success() -> Style {
        Style::default().fg(Color::Green)
    }

    /// Create info style (blue)
    pub fn info() -> Style {
        Style::default().fg(Color::Blue)
    }

    /// Create muted style (gray)
    pub fn muted() -> Style {
        Style::default().fg(Color::Gray)
    }

    /// Create highlight style (yellow, bold)
    pub fn highlight() -> Style {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    }

    /// Create selected style (dark gray background)
    pub fn selected() -> Style {
        Style::default().bg(Color::DarkGray)
    }

    /// Create status style based on status string
    pub fn status(status: &str) -> Style {
        match status.to_lowercase().as_str() {
            "running" | "active" | "online" | "connected" => Self::success(),
            "failed" | "error" | "offline" | "disconnected" => Self::error(),
            "warning" | "pending" | "waiting" => Self::warning(),
            _ => Self::default(),
        }
    }

    /// Create default style
    pub fn default() -> Style {
        Style::default()
    }

    /// Create span with style
    pub fn span(text: impl Into<String>, style: Style) -> Span<'static> {
        Span::styled(text.into(), style)
    }

    /// Create span with title style
    pub fn title_span(text: impl Into<String>) -> Span<'static> {
        Self::span(text, Self::title())
    }

    /// Create span with error style
    pub fn error_span(text: impl Into<String>) -> Span<'static> {
        Self::span(text, Self::error())
    }

    /// Create span with success style
    pub fn success_span(text: impl Into<String>) -> Span<'static> {
        Self::span(text, Self::success())
    }

    /// Create span with warning style
    pub fn warning_span(text: impl Into<String>) -> Span<'static> {
        Self::span(text, Self::warning())
    }

    /// Create span with muted style
    pub fn muted_span(text: impl Into<String>) -> Span<'static> {
        Self::span(text, Self::muted())
    }

    /// Create detail line (label: value)
    pub fn detail_line(label: &str, value: String) -> Line<'static> {
        Line::from(vec![
            Self::span(format!("{}: ", label), Self::title()),
            Span::raw(value),
        ])
    }

    /// Create styled block border configuration
    pub fn block_borders() -> ratatui::widgets::Borders {
        ratatui::widgets::Borders::ALL
    }

    /// Create block without borders
    pub fn block_no_borders() -> ratatui::widgets::Borders {
        ratatui::widgets::Borders::NONE
    }
}
