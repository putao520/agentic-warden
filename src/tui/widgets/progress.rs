//! Progress bar widget

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Gauge},
};

/// Progress bar widget
pub struct ProgressWidget {
    /// Progress label
    label: String,
    /// Current progress (0-100)
    progress: u16,
    /// Optional message
    message: Option<String>,
}

impl ProgressWidget {
    /// Create new progress widget
    pub fn new(label: String) -> Self {
        Self {
            label,
            progress: 0,
            message: None,
        }
    }

    /// Set progress value (0-100)
    pub fn set_progress(&mut self, progress: u16) {
        self.progress = progress.min(100);
    }

    /// Set message
    pub fn set_message(&mut self, message: String) {
        self.message = Some(message);
    }

    /// Clear message
    pub fn clear_message(&mut self) {
        self.message = None;
    }

    /// Get current progress
    pub fn progress(&self) -> u16 {
        self.progress
    }

    /// Check if complete
    pub fn is_complete(&self) -> bool {
        self.progress >= 100
    }

    /// Render the progress widget
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let label = if let Some(msg) = &self.message {
            format!("{} - {}", self.label, msg)
        } else {
            self.label.clone()
        };

        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title(label))
            .gauge_style(Style::default().fg(Color::Cyan))
            .percent(self.progress);

        frame.render_widget(gauge, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_clamps_to_hundred() {
        let mut widget = ProgressWidget::new("Sync".to_string());
        widget.set_progress(150);
        assert_eq!(widget.progress(), 100);
        assert!(widget.is_complete());
    }

    #[test]
    fn progress_message_roundtrip() {
        let mut widget = ProgressWidget::new("Sync".to_string());
        widget.set_message("Uploading".to_string());
        assert_eq!(widget.message.as_deref(), Some("Uploading"));
        widget.clear_message();
        assert!(widget.message.is_none());
    }
}
