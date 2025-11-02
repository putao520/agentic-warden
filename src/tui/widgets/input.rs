//! Input widget for text entry

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// Text input widget
#[derive(Clone)]
pub struct InputWidget {
    /// Input label
    label: String,
    /// Current input value
    value: String,
    /// Cursor position
    cursor_position: usize,
    /// Whether the input is focused
    focused: bool,
    /// Maximum input length
    max_length: Option<usize>,
    /// Whether to mask input (for passwords)
    masked: bool,
}

impl InputWidget {
    /// Create new input widget
    pub fn new(label: String) -> Self {
        Self {
            label,
            value: String::new(),
            cursor_position: 0,
            focused: false,
            max_length: None,
            masked: false,
        }
    }

    /// Set initial value
    pub fn with_value(mut self, value: String) -> Self {
        self.cursor_position = value.len();
        self.value = value;
        self
    }

    /// Set maximum length
    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.max_length = Some(max_length);
        self
    }

    /// Set masked mode (for passwords)
    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    /// Set focus state
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Get current value
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Set value programmatically
    pub fn set_value(&mut self, value: String) {
        self.cursor_position = value.len();
        self.value = value;
    }

    /// Clear input
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor_position = 0;
    }

    /// Handle key input
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(c) => {
                if let Some(max_len) = self.max_length {
                    if self.value.len() >= max_len {
                        return true;
                    }
                }
                self.value.insert(self.cursor_position, c);
                self.cursor_position += 1;
                true
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    self.value.remove(self.cursor_position);
                }
                true
            }
            KeyCode::Delete => {
                if self.cursor_position < self.value.len() {
                    self.value.remove(self.cursor_position);
                }
                true
            }
            KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
                true
            }
            KeyCode::Right => {
                if self.cursor_position < self.value.len() {
                    self.cursor_position += 1;
                }
                true
            }
            KeyCode::Home => {
                self.cursor_position = 0;
                true
            }
            KeyCode::End => {
                self.cursor_position = self.value.len();
                true
            }
            _ => false,
        }
    }

    /// Render the input widget
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let display_value = if self.masked {
            "*".repeat(self.value.len())
        } else {
            self.value.clone()
        };

        // Add cursor if focused
        let text = if self.focused {
            let before_cursor = &display_value[..self.cursor_position];
            let cursor_char = if self.cursor_position < display_value.len() {
                &display_value[self.cursor_position..self.cursor_position + 1]
            } else {
                " "
            };
            let after_cursor = if self.cursor_position < display_value.len() {
                &display_value[self.cursor_position + 1..]
            } else {
                ""
            };

            Line::from(vec![
                Span::raw(before_cursor),
                Span::styled(
                    cursor_char,
                    Style::default().bg(Color::White).fg(Color::Black),
                ),
                Span::raw(after_cursor),
            ])
        } else {
            Line::from(display_value)
        };

        let border_style = if self.focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(self.label.clone())
            .border_style(border_style);

        let paragraph = Paragraph::new(text).block(block);
        frame.render_widget(paragraph, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn handles_typing_and_navigation() {
        let mut widget = InputWidget::new("Label".to_string());
        widget.set_focused(true);

        widget.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        widget.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
        widget.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        widget.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
        widget.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));

        assert_eq!(widget.value(), "ab");
    }

    #[test]
    fn enforces_max_length() {
        let mut widget = InputWidget::new("Label".to_string()).with_max_length(2);
        widget.set_focused(true);

        widget.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        widget.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
        widget.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));

        assert_eq!(widget.value(), "ab");
    }

    #[test]
    fn masked_render_hides_underlying_value() {
        let backend = TestBackend::new(20, 3);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| {
                let mut widget = InputWidget::new("Secret".to_string())
                    .with_value("secret".to_string())
                    .masked(true);
                widget.set_focused(false);
                let area = frame.size();
                widget.render(frame, area);
            })
            .expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let rendered: String = (1..7)
            .map(|x| {
                let cell = buffer.get(x, 1);
                cell.symbol.chars().next().unwrap_or(' ')
            })
            .collect();
        assert_eq!(rendered.trim(), "******");
    }

    #[test]
    fn typing_and_backspace_updates_value() {
        let mut input = InputWidget::new("Prompt".to_string());
        input.set_focused(true);

        input.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        input.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
        input.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));

        assert_eq!(input.value(), "a");
    }

    #[test]
    fn set_value_and_clear_resets_cursor() {
        let mut input = InputWidget::new("Prompt".to_string());
        input.set_value("hello".to_string());
        assert_eq!(input.value(), "hello");

        input.clear();
        assert_eq!(input.value(), "");
    }
}
