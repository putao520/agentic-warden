//! List selection widget

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState},
};

/// List widget for item selection
pub struct ListWidget<T> {
    /// List items
    items: Vec<T>,
    /// Current selection state
    state: ListState,
    /// List title
    title: String,
}

impl<T> ListWidget<T>
where
    T: Clone,
{
    /// Create new list widget
    pub fn new(title: String, items: Vec<T>) -> Self {
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }

        Self {
            items,
            state,
            title,
        }
    }

    /// Get currently selected item
    pub fn selected(&self) -> Option<&T> {
        self.state.selected().and_then(|i| self.items.get(i))
    }

    /// Get selected index
    pub fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    /// Set items
    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
        if self.items.is_empty() {
            self.state.select(None);
        } else if self.state.selected().is_none()
            || self.state.selected().unwrap() >= self.items.len()
        {
            self.state.select(Some(0));
        }
    }

    /// Update items
    pub fn items(&self) -> &[T] {
        &self.items
    }

    /// Handle key input
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.items.is_empty() {
            return false;
        }

        match key.code {
            KeyCode::Up => {
                let i = self.state.selected().unwrap_or(0);
                if i > 0 {
                    self.state.select(Some(i - 1));
                }
                true
            }
            KeyCode::Down => {
                let i = self.state.selected().unwrap_or(0);
                if i < self.items.len() - 1 {
                    self.state.select(Some(i + 1));
                }
                true
            }
            KeyCode::Home => {
                self.state.select(Some(0));
                true
            }
            KeyCode::End => {
                self.state.select(Some(self.items.len() - 1));
                true
            }
            _ => false,
        }
    }

    /// Render the list widget
    pub fn render<F>(&mut self, frame: &mut Frame, area: Rect, format_fn: F)
    where
        F: Fn(&T, bool) -> String,
    {
        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let is_selected = self.state.selected() == Some(i);
                let content = format_fn(item, is_selected);
                ListItem::new(Line::from(content))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(self.title.clone()),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, area, &mut self.state);
    }
}
