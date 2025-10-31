//! Dialog widget for confirmations and messages

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

/// Dialog type
#[derive(Debug, Clone, PartialEq)]
pub enum DialogType {
    /// Information dialog
    Info,
    /// Warning dialog
    Warning,
    /// Error dialog
    Error,
    /// Confirmation dialog (Yes/No)
    Confirm,
}

/// Dialog result
#[derive(Debug, Clone, PartialEq)]
pub enum DialogResult {
    /// No action yet
    None,
    /// User confirmed (Yes)
    Confirmed,
    /// User cancelled (No)
    Cancelled,
    /// Dialog closed
    Closed,
}

/// Dialog widget
pub struct DialogWidget {
    /// Dialog type
    dialog_type: DialogType,
    /// Dialog title
    title: String,
    /// Dialog message
    message: String,
    /// Current selection (for confirm dialogs)
    selected_yes: bool,
}

impl DialogWidget {
    /// Create new dialog
    pub fn new(dialog_type: DialogType, title: String, message: String) -> Self {
        Self {
            dialog_type,
            title,
            message,
            selected_yes: true,
        }
    }

    /// Create info dialog
    pub fn info(title: String, message: String) -> Self {
        Self::new(DialogType::Info, title, message)
    }

    /// Create warning dialog
    pub fn warning(title: String, message: String) -> Self {
        Self::new(DialogType::Warning, title, message)
    }

    /// Create error dialog
    pub fn error(title: String, message: String) -> Self {
        Self::new(DialogType::Error, title, message)
    }

    /// Create confirm dialog
    pub fn confirm(title: String, message: String) -> Self {
        Self::new(DialogType::Confirm, title, message)
    }

    /// Handle key input
    pub fn handle_key(&mut self, key: KeyEvent) -> DialogResult {
        match key.code {
            KeyCode::Enter => {
                if self.dialog_type == DialogType::Confirm {
                    if self.selected_yes {
                        DialogResult::Confirmed
                    } else {
                        DialogResult::Cancelled
                    }
                } else {
                    DialogResult::Closed
                }
            }
            KeyCode::Esc => DialogResult::Cancelled,
            KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                if self.dialog_type == DialogType::Confirm {
                    self.selected_yes = !self.selected_yes;
                }
                DialogResult::None
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if self.dialog_type == DialogType::Confirm {
                    DialogResult::Confirmed
                } else {
                    DialogResult::None
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                if self.dialog_type == DialogType::Confirm {
                    DialogResult::Cancelled
                } else {
                    DialogResult::None
                }
            }
            _ => DialogResult::None,
        }
    }

    /// Render the dialog
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Center the dialog
        let dialog_area = Self::centered_rect(60, 40, area);

        // Clear background
        frame.render_widget(Clear, dialog_area);

        // Get dialog color
        let border_color = match self.dialog_type {
            DialogType::Info => Color::Blue,
            DialogType::Warning => Color::Yellow,
            DialogType::Error => Color::Red,
            DialogType::Confirm => Color::Cyan,
        };

        // Create block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(self.title.clone())
            .title_alignment(Alignment::Center);

        let inner_area = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        // Split inner area for message and buttons
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3)])
            .split(inner_area);

        // Render message
        let message_lines: Vec<Line> = self
            .message
            .lines()
            .map(|line| Line::from(line.to_string()))
            .collect();

        let message_paragraph = Paragraph::new(message_lines)
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);
        frame.render_widget(message_paragraph, chunks[0]);

        // Render buttons (for confirm dialog)
        if self.dialog_type == DialogType::Confirm {
            let button_text = if self.selected_yes {
                Line::from(vec![
                    Span::styled(
                        "[Y] Yes",
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled("[N] No", Style::default().fg(Color::White)),
                ])
            } else {
                Line::from(vec![
                    Span::styled("[Y] Yes", Style::default().fg(Color::White)),
                    Span::raw("  "),
                    Span::styled(
                        "[N] No",
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Red)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            };

            let buttons = Paragraph::new(button_text).alignment(Alignment::Center);
            frame.render_widget(buttons, chunks[1]);
        } else {
            let button_text = Line::from(Span::styled(
                "[Enter] OK",
                Style::default().add_modifier(Modifier::BOLD),
            ));
            let button = Paragraph::new(button_text).alignment(Alignment::Center);
            frame.render_widget(button, chunks[1]);
        }
    }

    /// Helper to create centered rect
    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }
}
