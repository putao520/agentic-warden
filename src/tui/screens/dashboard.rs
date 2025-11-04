//! Simplified Dashboard Screen

use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};
use std::collections::HashMap;

use super::{Screen, ScreenAction, ScreenType};

pub struct DashboardScreen {
    providers: Vec<crate::core::models::Provider>,
    selected_menu: usize,
    menu_items: Vec<String>,
}

impl DashboardScreen {
    pub fn new() -> Result<Self> {
        let menu_items = vec![
            "1. System Status".to_string(),
            "2. Provider Management".to_string(),
            "3. Push Files".to_string(),
            "4. Pull Files".to_string(),
            "5. OAuth Configuration".to_string(),
            "6. Exit".to_string(),
        ];

        Ok(Self {
            providers: Vec::new(),
            selected_menu: 0,
            menu_items,
        })
    }

    fn render_menu(&self, frame: &mut Frame, area: Rect) {
        let lines: Vec<String> = self.menu_items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let marker = if i == self.selected_menu { "▶ " } else { "  " };
                format!("{}{}", marker, item)
            })
            .collect();

        let paragraph = ratatui::widgets::Paragraph::new(lines.join("\n"))
            .block(ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title("🚀 Agentic Warden - Dashboard"));
        frame.render_widget(paragraph, area);
    }

    fn render_info(&self, frame: &mut Frame, area: Rect) {
        let info_text = format!(
            "Providers: {}\n\nUse ↑↓ to navigate, Enter to select",
            self.providers.len()
        );

        let paragraph = ratatui::widgets::Paragraph::new(info_text)
            .block(ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title("System Info"));
        frame.render_widget(paragraph, area);
    }
}

impl Screen for DashboardScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(12),
                ratatui::layout::Constraint::Min(0),
            ])
            .split(area);

        self.render_menu(frame, chunks[0]);
        self.render_info(frame, chunks[1]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Up => {
                if self.selected_menu > 0 {
                    self.selected_menu -= 1;
                }
                Ok(ScreenAction::None)
            }
            KeyCode::Down => {
                if self.selected_menu < self.menu_items.len() - 1 {
                    self.selected_menu += 1;
                }
                Ok(ScreenAction::None)
            }
            KeyCode::Enter => {
                match self.selected_menu {
                    0 => Ok(ScreenAction::SwitchTo(ScreenType::Status)),
                    1 => Ok(ScreenAction::SwitchTo(ScreenType::Provider)),
                    2 => Ok(ScreenAction::SwitchTo(ScreenType::Push(vec![]))),
                    3 => Ok(ScreenAction::SwitchTo(ScreenType::Pull)),
                    4 => Ok(ScreenAction::SwitchTo(ScreenType::OAuth)),
                    5 => Ok(ScreenAction::Quit),
                    _ => Ok(ScreenAction::None),
                }
            }
            KeyCode::Char('q') => Ok(ScreenAction::Quit),
            KeyCode::Esc => Ok(ScreenAction::Quit),
            KeyCode::Char('1') => Ok(ScreenAction::SwitchTo(ScreenType::Status)),
            KeyCode::Char('2') => Ok(ScreenAction::SwitchTo(ScreenType::Provider)),
            KeyCode::Char('3') => Ok(ScreenAction::SwitchTo(ScreenType::Push(vec![]))),
            KeyCode::Char('4') => Ok(ScreenAction::SwitchTo(ScreenType::Pull)),
            KeyCode::Char('5') => Ok(ScreenAction::SwitchTo(ScreenType::OAuth)),
            _ => Ok(ScreenAction::None),
        }
    }

    fn update(&mut self) -> Result<()> {
        // In a real implementation, this would refresh provider data
        Ok(())
    }
}