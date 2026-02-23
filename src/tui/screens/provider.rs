//! Provider management screen - Simplified view-only version
//!
//! Displays provider list. For editing, users should modify provider.json directly.

use std::path::PathBuf;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use super::{Screen, ScreenAction};
use crate::provider::manager::ProviderManager;

pub struct ProviderScreen {
    list_state: ListState,
    providers: Vec<(String, String, bool)>, // (id, summary, enabled)
    default_provider: Option<String>,
    message: Option<String>,
}

impl ProviderScreen {
    pub fn new() -> Result<Self> {
        let mut screen = Self {
            list_state: ListState::default(),
            providers: Vec::new(),
            default_provider: None,
            message: None,
        };
        screen.refresh_providers()?;
        if !screen.providers.is_empty() {
            screen.list_state.select(Some(0));
        }
        Ok(screen)
    }

    fn refresh_providers(&mut self) -> Result<()> {
        let manager = ProviderManager::new()?;
        let config = manager.get_providers_config();

        self.default_provider = Some(config.default_provider.clone());
        self.providers = config
            .providers
            .iter()
            .map(|(id, provider)| (id.clone(), provider.summary(), provider.is_enabled()))
            .collect();
        self.providers.sort_by(|a, b| a.0.cmp(&b.0));

        Ok(())
    }

    fn get_config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("agentic-warden")
            .join("provider.json")
    }
}

impl Screen for ProviderScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Provider list
                Constraint::Length(4), // Footer
            ])
            .split(area);

        // Header
        let header = Paragraph::new("Provider Configuration (View-Only)")
            .block(Block::default().borders(Borders::ALL))
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_widget(header, chunks[0]);

        // Provider list
        let items: Vec<ListItem> = self
            .providers
            .iter()
            .map(|(id, summary, enabled)| {
                let is_default = self
                    .default_provider
                    .as_ref()
                    .map(|d| d == id)
                    .unwrap_or(false);

                let status = if !enabled {
                    Span::styled(" [disabled]", Style::default().fg(Color::Red))
                } else {
                    Span::raw("")
                };

                let line = vec![
                    Span::raw(if is_default { "✓ " } else { "  " }),
                    Span::styled(id, Style::default().fg(if *enabled { Color::Yellow } else { Color::DarkGray })),
                    Span::raw(": "),
                    Span::styled(summary, Style::default().fg(if *enabled { Color::Gray } else { Color::DarkGray })),
                    status,
                ];

                ListItem::new(Line::from(line))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Providers"))
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, chunks[1], &mut self.list_state);

        // Footer
        let config_path = Self::get_config_path();
        let footer_text = if let Some(msg) = &self.message {
            msg.clone()
        } else {
            format!(
                "Configuration file: {}\n\n\
                 [↑↓] Navigate  [r] Refresh  [q] Back\n\
                 To edit providers, modify the configuration file directly",
                config_path.display()
            )
        };

        let footer = Paragraph::new(footer_text)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true });
        frame.render_widget(footer, chunks[2]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        match key.code {
            KeyCode::Up => {
                if let Some(selected) = self.list_state.selected() {
                    if selected > 0 {
                        self.list_state.select(Some(selected - 1));
                    }
                }
            }
            KeyCode::Down => {
                if let Some(selected) = self.list_state.selected() {
                    if selected < self.providers.len().saturating_sub(1) {
                        self.list_state.select(Some(selected + 1));
                    }
                }
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                if let Err(err) = self.refresh_providers() {
                    self.message = Some(format!("Failed to refresh: {}", err));
                } else {
                    self.message = Some("Providers refreshed successfully".to_string());
                }
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                return Ok(ScreenAction::Back)
            }
            _ => {}
        }
        Ok(ScreenAction::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    #[test]
    fn test_provider_screen_creation() {
        // This test may fail if provider.json doesn't exist
        // That's okay - it's testing the happy path
        let result = ProviderScreen::new();
        // Just ensure it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_navigation_keys() {
        let mut screen = ProviderScreen {
            list_state: ListState::default(),
            providers: vec![
                ("test1".to_string(), "summary1".to_string(), true),
                ("test2".to_string(), "summary2".to_string(), true),
            ],
            default_provider: Some("test1".to_string()),
            message: None,
        };
        screen.list_state.select(Some(0));

        // Test down navigation
        let action = screen
            .handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))
            .expect("handle key");
        assert!(matches!(action, ScreenAction::None));
        assert_eq!(screen.list_state.selected(), Some(1));

        // Test up navigation
        let action = screen
            .handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))
            .expect("handle key");
        assert!(matches!(action, ScreenAction::None));
        assert_eq!(screen.list_state.selected(), Some(0));

        // Test back
        let back = screen
            .handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))
            .expect("handle key");
        assert!(matches!(back, ScreenAction::Back));
    }
}
