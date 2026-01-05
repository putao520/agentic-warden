use std::io::{self, stdout};
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};

use crate::auto_mode::config::ExecutionOrderConfig;
use crate::cli_type::CliType;
use crate::error::ConfigError;

#[derive(Debug)]
pub struct CliOrderScreen {
    current_order: Vec<CliType>,
    original_order: Vec<CliType>,
    selected_index: usize,
    modified: bool,
    message: Option<String>,
    list_state: ListState,
}

impl CliOrderScreen {
    pub fn new() -> Result<Self, ConfigError> {
        let current_order = ExecutionOrderConfig::get_order()?;
        let mut list_state = ListState::default();
        let selected_index = 0;
        list_state.select(Some(selected_index));

        Ok(Self {
            original_order: current_order.clone(),
            current_order,
            selected_index,
            modified: false,
            message: None,
            list_state,
        })
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(4),
            ])
            .split(area);

        let header = Paragraph::new("AI CLI Execution Order")
            .block(Block::default().borders(Borders::ALL))
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_widget(header, chunks[0]);

        let items: Vec<ListItem> = self
            .current_order
            .iter()
            .enumerate()
            .map(|(index, cli_type)| {
                let line = Line::from(vec![
                    Span::styled(
                        format!("{}. ", index + 1),
                        Style::default().fg(Color::Gray),
                    ),
                    Span::styled(
                        cli_type.display_name(),
                        Style::default().fg(Color::Yellow),
                    ),
                ]);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Current Order"))
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, chunks[1], &mut self.list_state);

        let footer_text = if let Some(message) = &self.message {
            format!(
                "{}\n\n[↑/↓] Move  [r] Reset Default  [q] Save & Quit",
                message
            )
        } else {
            "[↑/↓] Move  [r] Reset Default  [q] Save & Quit".to_string()
        };

        let footer = Paragraph::new(footer_text)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true });
        frame.render_widget(footer, chunks[2]);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<bool, ConfigError> {
        match key.code {
            KeyCode::Up => {
                if self.selected_index > 0 {
                    self.current_order.swap(self.selected_index, self.selected_index - 1);
                    self.selected_index -= 1;
                    self.modified = true;
                }
            }
            KeyCode::Down => {
                if self.selected_index + 1 < self.current_order.len() {
                    self.current_order.swap(self.selected_index, self.selected_index + 1);
                    self.selected_index += 1;
                    self.modified = true;
                }
            }
            KeyCode::Char('r') => {
                self.current_order = ExecutionOrderConfig::reset_to_default();
                self.selected_index = 0;
                self.modified = true;
            }
            KeyCode::Char('q') => {
                if let Err(err) = ExecutionOrderConfig::save_order(&self.current_order) {
                    self.message = Some(format!("Failed to save: {}", err));
                    return Ok(false);
                }
                return Ok(true);
            }
            _ => {}
        }

        self.list_state.select(Some(self.selected_index));
        Ok(false)
    }

    pub fn is_modified(&self) -> bool {
        self.modified || self.current_order != self.original_order
    }
}

pub fn run_cli_order_tui() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode().map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
    let mut stdout = stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        crossterm::terminal::Clear(ClearType::All)
    )?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut screen = CliOrderScreen::new()?;
        loop {
            terminal.draw(|frame| {
                let area = frame.size();
                screen.render(frame, area);
            })?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if screen.handle_key(key)? {
                        break;
                    }
                }
            }
        }
        Ok(())
    })();

    disable_raw_mode().ok();
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::terminal::Clear(ClearType::All)
    )
    .ok();

    result
}
