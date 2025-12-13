//! Screen-local rendering helpers
//!
//! **IMPORTANT**: This module contains ONLY rendering helper functions that use
//! ratatui standard components (Block, Paragraph, Gauge, Clear, etc.).
//!
//! These are NOT custom widgets - they are simple state structs + rendering functions
//! that directly use ratatui's standard components for all rendering.
//!
//! This approach complies with SPEC: "使用 ratatui 组件库的现成组件"
//! - All rendering uses Block, Paragraph, Gauge, Clear, etc.
//! - No custom rendering logic
//! - Just state management + standard component composition

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Wrap},
    Frame,
};

/// Dialog interaction result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogResult {
    None,
    Confirmed,
    Cancelled,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum DialogKind {
    Info,
    Error,
    Confirm,
}

/// Dialog state - renders using ONLY ratatui standard components
#[derive(Debug, Clone)]
pub struct DialogState {
    title: String,
    message: String,
    kind: DialogKind,
    buttons: Vec<String>,
    selected: usize,
}

impl DialogState {
    #[allow(dead_code)]
    pub fn info(title: String, message: String) -> Self {
        Self {
            title,
            message,
            kind: DialogKind::Info,
            buttons: vec!["OK".to_string()],
            selected: 0,
        }
    }

    #[allow(dead_code)]
    pub fn error(title: String, message: String) -> Self {
        Self {
            title,
            message,
            kind: DialogKind::Error,
            buttons: vec!["OK".to_string()],
            selected: 0,
        }
    }

    pub fn confirm(title: String, message: String) -> Self {
        Self {
            title,
            message,
            kind: DialogKind::Confirm,
            buttons: vec!["Confirm".to_string(), "Cancel".to_string()],
            selected: 0,
        }
    }

    /// Render using ONLY ratatui standard components: Clear, Block, Paragraph
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let width = area.width.min(70).max(30);
        let height = area.height.min(14).max(8);
        let dialog_area = Rect {
            x: area.x + (area.width.saturating_sub(width)) / 2,
            y: area.y + (area.height.saturating_sub(height)) / 2,
            width,
            height,
        };

        // Clear - ratatui standard component
        frame.render_widget(Clear, dialog_area);

        let title_style = match self.kind {
            DialogKind::Error => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            DialogKind::Confirm => Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            DialogKind::Info => Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        };

        // Block - ratatui standard component
        let block = Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(&self.title, title_style));
        frame.render_widget(block, dialog_area);

        let inner = Rect {
            x: dialog_area.x + 1,
            y: dialog_area.y + 1,
            width: dialog_area.width.saturating_sub(2),
            height: dialog_area.height.saturating_sub(2),
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(inner.height.saturating_sub(3)),
                Constraint::Length(3),
            ])
            .split(inner);

        // Paragraph - ratatui standard component
        let message_lines: Vec<Line> = self
            .message
            .lines()
            .map(|line| Line::from(line.to_string()))
            .collect();
        let paragraph = Paragraph::new(message_lines)
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);
        frame.render_widget(paragraph, chunks[0]);

        if !self.buttons.is_empty() {
            let mut spans = Vec::new();
            for (idx, label) in self.buttons.iter().enumerate() {
                if idx > 0 {
                    spans.push(Span::raw("   "));
                }
                let style = if idx == self.selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                spans.push(Span::styled(format!("[ {} ]", label), style));
            }
            // Paragraph - ratatui standard component
            let buttons = Paragraph::new(Line::from(spans)).alignment(Alignment::Center);
            frame.render_widget(buttons, chunks[1]);
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> DialogResult {
        match key.code {
            KeyCode::Left => {
                if self.buttons.len() > 1 && self.selected > 0 {
                    self.selected -= 1;
                }
                DialogResult::None
            }
            KeyCode::Right => {
                if self.buttons.len() > 1 && self.selected + 1 < self.buttons.len() {
                    self.selected += 1;
                }
                DialogResult::None
            }
            KeyCode::Tab => {
                if self.buttons.len() > 1 {
                    self.selected = (self.selected + 1) % self.buttons.len();
                }
                DialogResult::None
            }
            KeyCode::Enter => self.selection_result(),
            KeyCode::Esc => DialogResult::Cancelled,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                DialogResult::Closed
            }
            // 数字键直接选择对应的按钮
            KeyCode::Char('1') if self.buttons.len() >= 1 => {
                self.selected = 0;
                self.selection_result()
            }
            KeyCode::Char('2') if self.buttons.len() >= 2 => {
                self.selected = 1;
                self.selection_result()
            }
            _ => DialogResult::None,
        }
    }

    fn selection_result(&self) -> DialogResult {
        if self.buttons.is_empty() {
            return DialogResult::Closed;
        }
        let label = &self.buttons[self.selected];
        if self.kind == DialogKind::Confirm && label.eq_ignore_ascii_case("cancel") {
            DialogResult::Cancelled
        } else {
            DialogResult::Confirmed
        }
    }
}

/// Input state - renders using ONLY ratatui standard components
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct InputState {
    label: String,
    value: String,
    cursor: usize,
    focused: bool,
    masked: bool,
}

#[allow(dead_code)]
impl InputState {
    pub fn new(label: String) -> Self {
        Self {
            label,
            value: String::new(),
            cursor: 0,
            focused: false,
            masked: false,
        }
    }

    pub fn with_value(mut self, value: String) -> Self {
        self.cursor = value.len();
        self.value = value;
        self
    }

    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Left => {
                self.move_cursor_left();
                true
            }
            KeyCode::Right => {
                self.move_cursor_right();
                true
            }
            KeyCode::Home => {
                self.cursor = 0;
                true
            }
            KeyCode::End => {
                self.cursor = self.value.len();
                true
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    let prev = self.prev_grapheme();
                    self.value.drain(prev..self.cursor);
                    self.cursor = prev;
                }
                true
            }
            KeyCode::Delete => {
                if self.cursor < self.value.len() {
                    let next = self.next_grapheme();
                    self.value.drain(self.cursor..next);
                }
                true
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.value.insert(self.cursor, c);
                self.cursor += c.len_utf8();
                true
            }
            _ => false,
        }
    }

    /// Render using ONLY ratatui standard components: Block, Paragraph
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = self
            .label
            .lines()
            .map(|line| Line::from(line.to_string()))
            .collect();
        lines.push(Line::from(""));

        let display = if self.masked {
            if self.value.is_empty() {
                String::new()
            } else {
                "*".repeat(self.value.chars().count())
            }
        } else {
            self.value.clone()
        };
        lines.push(Line::from(vec![Span::styled(
            display,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )]));

        // Block and Paragraph - ratatui standard components
        let block = Block::default()
            .borders(Borders::ALL)
            .title(Span::styled("Input", Style::default().fg(Color::Cyan)));
        let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);

        if self.focused {
            let label_lines = self.label.lines().count() as u16;
            let max_x = area.x + area.width.saturating_sub(2);
            let mut cursor_x = area.x + 1 + self.cursor as u16;
            if cursor_x > max_x {
                cursor_x = max_x;
            }
            let mut cursor_y = area.y + 1 + label_lines;
            let max_y = area.y + area.height.saturating_sub(1);
            if cursor_y > max_y {
                cursor_y = max_y;
            }
            frame.set_cursor(cursor_x, cursor_y);
        }
    }

    fn move_cursor_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor = self.prev_grapheme();
    }

    fn move_cursor_right(&mut self) {
        if self.cursor >= self.value.len() {
            return;
        }
        self.cursor = self.next_grapheme();
    }

    fn prev_grapheme(&self) -> usize {
        self.value[..self.cursor]
            .char_indices()
            .rev()
            .next()
            .map(|(idx, _)| idx)
            .unwrap_or(0)
    }

    fn next_grapheme(&self) -> usize {
        let mut iter = self.value[self.cursor..].char_indices();
        iter.next();
        if let Some((offset, _)) = iter.next() {
            self.cursor + offset
        } else {
            self.value.len()
        }
    }
}

/// Progress state - renders using ONLY ratatui standard components
#[derive(Debug, Clone)]
pub struct ProgressState {
    title: String,
    progress: u16,
    message: Option<String>,
}

impl ProgressState {
    pub fn new(title: String) -> Self {
        Self {
            title,
            progress: 0,
            message: None,
        }
    }

    pub fn set_progress(&mut self, value: u16) {
        self.progress = value.min(100);
    }

    pub fn set_message(&mut self, message: String) {
        if message.is_empty() {
            self.message = None;
        } else {
            self.message = Some(message);
        }
    }

    pub fn clear_message(&mut self) {
        self.message = None;
    }

    /// Render using ONLY ratatui standard components: Block, Gauge, Paragraph
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Block - ratatui standard component
        let block = Block::default().borders(Borders::ALL).title(Span::styled(
            &self.title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(block, area);

        let inner = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };

        if inner.height == 0 {
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(inner);

        // Gauge - ratatui standard component
        let ratio = self.progress as f64 / 100.0;
        let gauge = Gauge::default()
            .gauge_style(
                Style::default()
                    .fg(Color::Green)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .ratio(ratio.clamp(0.0, 1.0))
            .label(format!("{:>3}% complete", self.progress));
        frame.render_widget(gauge, chunks[0]);

        // Paragraph - ratatui standard component
        let message = self.message.as_deref().unwrap_or("Waiting for updates...");
        let paragraph = Paragraph::new(message)
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);
        frame.render_widget(paragraph, chunks[1]);
    }
}
