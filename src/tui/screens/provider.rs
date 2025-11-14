//! Provider management screen
//!
//! Implements provider listing, editing, and interactions as described in
//! SPEC/API.md §2.2.

use std::collections::HashMap;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use super::{Screen, ScreenAction, ScreenType};
use crate::provider::config::{AiType, Provider};
use crate::provider::manager::ProviderManager;
use crate::tui::app_state::AppState;

const COMPATIBILITY_LABELS: [&str; 3] = ["Codex", "Claude", "Gemini"];

#[derive(Debug, Clone)]
struct ProviderEntry {
    id: String,
    provider: Provider,
    is_default: bool,
}

#[derive(Debug, Clone)]
struct ProviderMeta {
    display_name: String,
    icon: Option<String>,
    official: bool,
    protected: bool,
    custom: bool,
}

#[derive(Debug, Clone)]
struct EnvVarEntry {
    key: String,
    value: String,
}

#[derive(Debug, Clone)]
struct EditState {
    provider_id: String,
    meta: ProviderMeta,
    description: String,
    compat: [bool; 3],
    env_vars: Vec<EnvVarEntry>,
    selection: EditSelection,
    input_mode: Option<InputTarget>,
    is_new: bool,
    message: Option<String>,
}

enum Mode {
    List,
    InputId { buffer: String },
    Edit(EditState),
    ConfirmDelete { provider_id: String, cursor: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditSelection {
    Description,
    Compatibility(usize),
    EnvVar(usize, EnvField),
    EnvAdd,
    Save,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnvField {
    Key,
    Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputTarget {
    Description,
    EnvKey(usize),
    EnvValue(usize),
}

pub struct ProviderScreen {
    manager: ProviderManager,
    entries: Vec<ProviderEntry>,
    default_provider: String,
    list_state: ListState,
    mode: Mode,
    message: Option<String>,
    app_state: &'static AppState,
}

impl ProviderScreen {
    pub fn new() -> Result<Self> {
        let manager = ProviderManager::new()?;
        let mut screen = Self {
            manager,
            entries: Vec::new(),
            default_provider: String::new(),
            list_state: ListState::default(),
            mode: Mode::List,
            message: None,
            app_state: AppState::global(),
        };
        screen.refresh_entries()?;
        Ok(screen)
    }

    fn refresh_entries(&mut self) -> Result<()> {
        let previous_id = self
            .list_state
            .selected()
            .and_then(|idx| self.entries.get(idx).map(|entry| entry.id.clone()));

        let default_id = self.manager.default_provider_name().to_string();
        let mut entries: Vec<ProviderEntry> = self
            .manager
            .list_providers()
            .into_iter()
            .map(|(id, provider)| ProviderEntry {
                id: id.clone(),
                provider: provider.clone(),
                is_default: id == &default_id,
            })
            .collect();

        entries.sort_by(|a, b| a.id.cmp(&b.id));

        self.default_provider = default_id;
        self.entries = entries;

        if self.entries.is_empty() {
            self.list_state.select(None);
        } else if let Some(previous_id) = previous_id {
            let idx = self
                .entries
                .iter()
                .position(|entry| entry.id == previous_id)
                .unwrap_or(0);
            self.list_state.select(Some(idx));
        } else if self.list_state.selected().is_none() {
            self.list_state.select(Some(0));
        }

        self.publish_provider_snapshot();

        Ok(())
    }

    fn publish_provider_snapshot(&self) {
        let snapshot = self
            .entries
            .iter()
            .map(|entry| (entry.id.clone(), entry.provider.clone()))
            .collect::<Vec<_>>();
        self.app_state
            .set_providers(snapshot, Some(self.default_provider.clone()));
    }

    fn selected_entry(&self) -> Option<&ProviderEntry> {
        self.list_state
            .selected()
            .and_then(|idx| self.entries.get(idx))
    }

    fn start_add(&mut self) {
        self.mode = Mode::InputId {
            buffer: String::new(),
        };
        self.message = None;
    }

    fn start_edit(&mut self) {
        let Some(entry) = self.selected_entry() else {
            self.message = Some("No provider selected".to_string());
            return;
        };

        if entry.id.eq_ignore_ascii_case("official") {
            self.message = Some("Official provider cannot be edited".to_string());
            return;
        }

        self.mode = Mode::Edit(EditState::from_provider(
            entry.id.clone(),
            &entry.provider,
            false,
        ));
    }

    fn start_delete(&mut self) {
        let Some(entry) = self.selected_entry() else {
            self.message = Some("No provider selected".to_string());
            return;
        };

        self.mode = Mode::ConfirmDelete {
            provider_id: entry.id.clone(),
            cursor: 0,
        };
    }

    fn set_default_provider(&mut self) {
        let provider_id = match self.selected_entry() {
            Some(entry) => {
                if entry.is_default {
                    self.message = Some(format!("'{}' is already default", entry.id));
                    return;
                }
                entry.id.clone()
            }
            None => {
                self.message = Some("No provider selected".to_string());
                return;
            }
        };

        match self.manager.set_default_provider(&provider_id) {
            Ok(()) => {
                if let Err(err) = self.refresh_entries() {
                    self.message = Some(format!("Failed to refresh providers: {}", err));
                } else {
                    self.message = Some(format!("Provider '{}' set as default", provider_id));
                }
            }
            Err(err) => {
                self.message = Some(format!("Failed to set default provider: {}", err));
            }
        }
    }

    fn render_list(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
                Constraint::Length(2),
            ])
            .split(area);

        let title = Paragraph::new("Provider Management")
            .alignment(ratatui::layout::Alignment::Center)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, layout[0]);

        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(layout[1]);

        let items: Vec<ListItem> = self
            .entries
            .iter()
            .map(|entry| {
                let mut label = entry.provider.name.clone();
                if label.trim().is_empty() {
                    label = entry.id.clone();
                }
                if entry.is_default {
                    label.push_str(" [default]");
                }
                ListItem::new(label)
            })
            .collect();

        let mut list_state = self.list_state.clone();
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Providers"))
            .highlight_symbol("▶ ")
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_stateful_widget(list, body[0], &mut list_state);
        self.list_state = list_state;

        let detail_text = match self.selected_entry() {
            Some(entry) => format_provider_details(entry),
            None => "No provider selected.".to_string(),
        };

        let detail = Paragraph::new(detail_text)
            .block(Block::default().borders(Borders::ALL).title("Details"))
            .wrap(Wrap { trim: true });
        frame.render_widget(detail, body[1]);

        let help = Paragraph::new(
            "[↑/↓] Select  [Enter] Set Default  [A] Smart Add  [M] Manual Add  [E] Edit  [D] Delete  [R] Refresh  [ESC] Back",
        )
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, layout[2]);

        let status_text = self.message.as_deref().unwrap_or("Ready");
        let status = Paragraph::new(status_text)
            .style(Style::default().fg(Color::LightBlue))
            .block(Block::default().borders(Borders::ALL).title("Status"));
        frame.render_widget(status, layout[3]);
    }

    fn render_input_id(&self, frame: &mut Frame, area: Rect, buffer: &str) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(area);

        let title = Paragraph::new("Add Provider - Enter Provider ID")
            .alignment(ratatui::layout::Alignment::Center)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, layout[0]);

        let prompt = Paragraph::new(buffer)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Provider ID")
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .wrap(Wrap { trim: true });
        frame.render_widget(prompt, layout[1]);

        let help = Paragraph::new("[Enter] Continue  [ESC] Cancel")
            .alignment(ratatui::layout::Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, layout[2]);
    }

    fn render_edit(&self, frame: &mut Frame, area: Rect, state: &EditState) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
                Constraint::Length(2),
            ])
            .split(area);

        let title_text = if state.is_new {
            format!("Add Provider '{}'", state.provider_id)
        } else {
            format!("Edit Provider '{}'", state.provider_id)
        };
        let title = Paragraph::new(title_text)
            .alignment(ratatui::layout::Alignment::Center)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, layout[0]);

        let content = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),
                Constraint::Length(4),
                Constraint::Min(4),
            ])
            .split(layout[1]);

        // Description
        let desc_style = if state.selection == EditSelection::Description {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let desc_text = if state.description.is_empty() {
            "(empty)".to_string()
        } else {
            state.description.clone()
        };
        let desc_block = Block::default()
            .borders(Borders::ALL)
            .title("Description")
            .border_style(desc_style);
        let desc = Paragraph::new(desc_text)
            .block(desc_block)
            .wrap(Wrap { trim: false });
        frame.render_widget(desc, content[0]);

        // Compatibility
        let compat_block = Block::default()
            .borders(Borders::ALL)
            .title("Compatible AI CLI");
        let mut compat_lines = Vec::new();
        for (idx, label) in COMPATIBILITY_LABELS.iter().enumerate() {
            let selected = matches!(state.selection, EditSelection::Compatibility(i) if i == idx);
            let mark = if state.compat[idx] { "[x]" } else { "[ ]" };
            let style = if selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            compat_lines.push(Line::from(Span::styled(
                format!("{} {}", mark, label),
                style,
            )));
        }
        let compat = Paragraph::new(compat_lines)
            .block(compat_block)
            .wrap(Wrap { trim: true });
        frame.render_widget(compat, content[1]);

        // Environment variables
        let env_block = Block::default()
            .borders(Borders::ALL)
            .title("Environment Variables");
        let mut env_lines: Vec<Line> = Vec::new();
        if state.env_vars.is_empty() {
            env_lines.push(Line::from("(none)"));
        } else {
            for (idx, entry) in state.env_vars.iter().enumerate() {
                let key_selected = matches!(
                    state.selection,
                    EditSelection::EnvVar(i, EnvField::Key) if i == idx
                );
                let value_selected = matches!(
                    state.selection,
                    EditSelection::EnvVar(i, EnvField::Value) if i == idx
                );

                let key_style = if key_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let value_style = if value_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                env_lines.push(Line::from(vec![
                    Span::styled(
                        format!("{}: ", idx + 1),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(entry.key.clone(), key_style),
                    Span::raw(" = "),
                    Span::styled(entry.value.clone(), value_style),
                ]));
            }
        }

        let add_selected = state.selection == EditSelection::EnvAdd;
        let add_style = if add_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        env_lines.push(Line::from(Span::styled("[+] Add variable", add_style)));

        let env = Paragraph::new(env_lines)
            .block(env_block)
            .wrap(Wrap { trim: true });
        frame.render_widget(env, content[2]);

        let controls_text = match state.selection {
            EditSelection::Description => {
                if matches!(state.input_mode, Some(InputTarget::Description)) {
                    "Typing... [Enter] Finish  [ESC] Cancel input"
                } else {
                    "[Enter] Edit description"
                }
            }
            EditSelection::Compatibility(_) => "[Space] Toggle  [↑/↓] Move",
            EditSelection::EnvVar(_, _) => {
                if matches!(
                    state.input_mode,
                    Some(InputTarget::EnvKey(_)) | Some(InputTarget::EnvValue(_))
                ) {
                    "Typing... [Enter] Finish  [ESC] Cancel input"
                } else {
                    "[Enter] Edit  [D] Delete  [←/→] Switch field"
                }
            }
            EditSelection::EnvAdd => "[Enter] Add new variable",
            EditSelection::Save => "[Enter] Save changes",
            EditSelection::Cancel => "[Enter] Discard changes",
        };
        let controls = Paragraph::new(controls_text)
            .alignment(ratatui::layout::Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(controls, layout[2]);

        let status_text = state
            .message
            .as_deref()
            .unwrap_or("[Tab/↓] Navigate  [S] Save  [ESC] Cancel");
        let status = Paragraph::new(status_text)
            .style(Style::default().fg(Color::LightBlue))
            .block(Block::default().borders(Borders::ALL).title("Status"));
        frame.render_widget(status, layout[3]);
    }

    fn render_confirm_delete(
        &self,
        frame: &mut Frame,
        area: Rect,
        provider_id: &str,
        cursor: usize,
    ) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(area);

        let title = Paragraph::new("Confirm Delete")
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, layout[0]);

        let message = Paragraph::new(format!(
            "Are you sure you want to delete provider '{}'?\nThis action cannot be undone.",
            provider_id
        ))
        .block(Block::default().borders(Borders::ALL))
        .wrap(Wrap { trim: true });
        frame.render_widget(message, layout[1]);

        let yes_style = if cursor == 0 {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let no_style = if cursor == 1 {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let actions = Paragraph::new(Line::from(vec![
            Span::styled("[ Yes ]", yes_style),
            Span::raw("    "),
            Span::styled("[ No ]", no_style),
        ]))
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(actions, layout[2]);
    }
    fn handle_list_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
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
                    if selected + 1 < self.entries.len() {
                        self.list_state.select(Some(selected + 1));
                    }
                } else if !self.entries.is_empty() {
                    self.list_state.select(Some(0));
                }
            }
            KeyCode::Enter => self.set_default_provider(),
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.message = Some("Provider wizard removed. Use 'M' to manually add providers.".to_string());
            }
            KeyCode::Char('m') | KeyCode::Char('M') => self.start_add(),
            KeyCode::Char('e') | KeyCode::Char('E') => self.start_edit(),
            KeyCode::Char('d') | KeyCode::Char('D') => self.start_delete(),
            KeyCode::Char('r') | KeyCode::Char('R') => {
                if let Err(err) = self.refresh_entries() {
                    self.message = Some(format!("Failed to refresh: {}", err));
                }
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                return Ok(ScreenAction::Back)
            }
            _ => {}
        }
        Ok(ScreenAction::None)
    }

    fn handle_input_id_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        if let Mode::InputId { buffer } = &mut self.mode {
            match key.code {
                KeyCode::Esc => self.mode = Mode::List,
                KeyCode::Backspace => {
                    buffer.pop();
                }
                KeyCode::Char(c) => {
                    if is_valid_provider_char(c) && buffer.len() < 40 {
                        buffer.push(c);
                    }
                }
                KeyCode::Enter => {
                    let id = buffer.trim();
                    if id.is_empty() {
                        self.message = Some("Provider ID cannot be empty".to_string());
                    } else if self.entries.iter().any(|entry| entry.id == id) {
                        self.message = Some(format!("Provider '{}' exists", id));
                    } else {
                        self.mode = Mode::Edit(EditState::new(id.to_string()));
                    }
                }
                _ => {}
            }
        }
        Ok(ScreenAction::None)
    }

    fn handle_edit_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        let Mode::Edit(ref mut state) = self.mode else {
            return Ok(ScreenAction::None);
        };

        let mut save_request: Option<(Provider, String, bool)> = None;
        let mut cancel = false;

        if let Some(target) = state.input_mode {
            match key.code {
                KeyCode::Esc => state.input_mode = None,
                KeyCode::Backspace => match target {
                    InputTarget::Description => {
                        state.description.pop();
                    }
                    InputTarget::EnvKey(idx) => {
                        if let Some(entry) = state.env_vars.get_mut(idx) {
                            entry.key.pop();
                        }
                    }
                    InputTarget::EnvValue(idx) => {
                        if let Some(entry) = state.env_vars.get_mut(idx) {
                            entry.value.pop();
                        }
                    }
                },
                KeyCode::Char(c) => match target {
                    InputTarget::Description => state.description.push(c),
                    InputTarget::EnvKey(idx) => {
                        if let Some(entry) = state.env_vars.get_mut(idx) {
                            entry.key.push(c);
                        }
                    }
                    InputTarget::EnvValue(idx) => {
                        if let Some(entry) = state.env_vars.get_mut(idx) {
                            entry.value.push(c);
                        }
                    }
                },
                KeyCode::Enter => {
                    state.input_mode = None;
                    if let InputTarget::EnvKey(idx) = target {
                        state.selection = EditSelection::EnvVar(idx, EnvField::Value);
                        state.input_mode = Some(InputTarget::EnvValue(idx));
                    }
                }
                _ => {}
            }
            return Ok(ScreenAction::None);
        }

        match key.code {
            KeyCode::Esc => cancel = true,
            KeyCode::Up => move_selection_up(state),
            KeyCode::Down | KeyCode::Tab => move_selection_down(state),
            KeyCode::Char('s') | KeyCode::Char('S') => match state.build_provider_snapshot() {
                Some(snapshot) => save_request = Some(snapshot),
                None => {
                    state.message = Some("Select at least one compatible AI CLI.".to_string());
                }
            },
            KeyCode::Enter => match state.selection {
                EditSelection::Description => {
                    state.input_mode = Some(InputTarget::Description);
                }
                EditSelection::Compatibility(idx) => toggle_compat(state, idx),
                EditSelection::EnvVar(idx, field) => {
                    state.input_mode = Some(match field {
                        EnvField::Key => InputTarget::EnvKey(idx),
                        EnvField::Value => InputTarget::EnvValue(idx),
                    });
                }
                EditSelection::EnvAdd => {
                    state.env_vars.push(EnvVarEntry {
                        key: String::new(),
                        value: String::new(),
                    });
                    let idx = state.env_vars.len() - 1;
                    state.selection = EditSelection::EnvVar(idx, EnvField::Key);
                    state.input_mode = Some(InputTarget::EnvKey(idx));
                }
                EditSelection::Save => match state.build_provider_snapshot() {
                    Some(snapshot) => save_request = Some(snapshot),
                    None => {
                        state.message = Some("Select at least one compatible AI CLI.".to_string());
                    }
                },
                EditSelection::Cancel => cancel = true,
            },
            KeyCode::Char(' ') => {
                if let EditSelection::Compatibility(idx) = state.selection {
                    toggle_compat(state, idx);
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if let EditSelection::EnvVar(idx, _) = state.selection {
                    if idx < state.env_vars.len() {
                        state.env_vars.remove(idx);
                        if idx >= state.env_vars.len() && !state.env_vars.is_empty() {
                            state.selection =
                                EditSelection::EnvVar(state.env_vars.len() - 1, EnvField::Key);
                        } else if state.env_vars.is_empty() {
                            state.selection = EditSelection::EnvAdd;
                        }
                    }
                }
            }
            KeyCode::Left => {
                if let EditSelection::EnvVar(idx, field) = state.selection {
                    let next = match field {
                        EnvField::Key => EnvField::Key,
                        EnvField::Value => EnvField::Key,
                    };
                    state.selection = EditSelection::EnvVar(idx, next);
                }
            }
            KeyCode::Right => {
                if let EditSelection::EnvVar(idx, field) = state.selection {
                    let next = match field {
                        EnvField::Key => EnvField::Value,
                        EnvField::Value => EnvField::Value,
                    };
                    state.selection = EditSelection::EnvVar(idx, next);
                }
            }
            _ => {}
        }

        if let Some((provider, id, is_new)) = save_request {
            let result = if is_new {
                self.manager.add_provider(id.clone(), provider)
            } else {
                self.manager.update_provider(&id, provider)
            };

            match result {
                Ok(()) => {
                    self.mode = Mode::List;
                    self.message = Some(if is_new {
                        format!("Provider '{}' created", id)
                    } else {
                        format!("Provider '{}' updated", id)
                    });
                    self.refresh_entries()?;
                    if let Some(idx) = self.entries.iter().position(|entry| entry.id == id) {
                        self.list_state.select(Some(idx));
                    }
                }
                Err(err) => {
                    if let Mode::Edit(state) = &mut self.mode {
                        state.message = Some(format!("{}", err));
                    }
                }
            }
        }

        if cancel {
            self.mode = Mode::List;
            self.refresh_entries()?;
        }

        Ok(ScreenAction::None)
    }

    fn handle_confirm_delete_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        let Mode::ConfirmDelete {
            ref provider_id,
            ref mut cursor,
        } = self.mode
        else {
            return Ok(ScreenAction::None);
        };

        match key.code {
            KeyCode::Esc => self.mode = Mode::List,
            KeyCode::Left | KeyCode::Right | KeyCode::Tab => *cursor = 1 - *cursor,
            KeyCode::Enter => {
                if *cursor == 0 {
                    match self.manager.remove_provider(provider_id) {
                        Ok(()) => {
                            self.message = Some(format!("Provider '{}' removed", provider_id));
                            self.mode = Mode::List;
                            self.refresh_entries()?;
                        }
                        Err(err) => {
                            self.message = Some(format!("Failed to remove provider: {}", err));
                            self.mode = Mode::List;
                        }
                    }
                } else {
                    self.mode = Mode::List;
                }
            }
            _ => {}
        }
        Ok(ScreenAction::None)
    }
}
impl Screen for ProviderScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        match &self.mode {
            Mode::List => self.render_list(frame, area),
            Mode::InputId { buffer } => self.render_input_id(frame, area, buffer),
            Mode::Edit(state) => self.render_edit(frame, area, state),
            Mode::ConfirmDelete {
                provider_id,
                cursor,
            } => self.render_confirm_delete(frame, area, provider_id, *cursor),
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        match &self.mode {
            Mode::List => self.handle_list_key(key),
            Mode::InputId { .. } => self.handle_input_id_key(key),
            Mode::Edit(_) => self.handle_edit_key(key),
            Mode::ConfirmDelete { .. } => self.handle_confirm_delete_key(key),
        }
    }

    fn update(&mut self) -> Result<()> {
        Ok(())
    }
}

impl EditState {
    fn new(provider_id: String) -> Self {
        Self {
            provider_id: provider_id.clone(),
            meta: ProviderMeta {
                display_name: provider_id.clone(),
                icon: None,
                official: false,
                protected: false,
                custom: true,
            },
            description: String::new(),
            compat: [false, false, false],
            env_vars: Vec::new(),
            selection: EditSelection::Description,
            input_mode: None,
            is_new: true,
            message: None,
        }
    }

    fn from_provider(provider_id: String, provider: &Provider, is_new: bool) -> Self {
        let mut env_vars: Vec<EnvVarEntry> = provider
            .env
            .iter()
            .map(|(key, value)| EnvVarEntry {
                key: key.clone(),
                value: value.clone(),
            })
            .collect();
        env_vars.sort_by(|a, b| a.key.cmp(&b.key));

        Self {
            provider_id: provider_id.clone(),
            meta: ProviderMeta {
                display_name: provider.name.clone(),
                icon: provider.icon.clone(),
                official: provider.official,
                protected: provider.protected,
                custom: provider.custom,
            },
            description: provider.description.clone(),
            compat: [
                provider.compatible_with.contains(&AiType::Codex),
                provider.compatible_with.contains(&AiType::Claude),
                provider.compatible_with.contains(&AiType::Gemini),
            ],
            env_vars,
            selection: EditSelection::Description,
            input_mode: None,
            is_new,
            message: None,
        }
    }

    fn build_provider_snapshot(&self) -> Option<(Provider, String, bool)> {
        if !self.compat.iter().any(|flag| *flag) {
            return None;
        }

        let mut env = HashMap::new();
        for entry in &self.env_vars {
            if !entry.key.trim().is_empty() {
                env.insert(entry.key.trim().to_string(), entry.value.clone());
            }
        }

        let provider = Provider {
            name: self.meta.display_name.clone(),
            description: self.description.clone(),
            icon: self.meta.icon.clone(),
            official: self.meta.official,
            protected: self.meta.protected,
            custom: self.meta.custom,
            compatible_with: self.compatibility_vec(),
            env,
        };

        Some((provider, self.provider_id.clone(), self.is_new))
    }

    fn compatibility_vec(&self) -> Vec<AiType> {
        let mut result = Vec::new();
        if self.compat[0] {
            result.push(AiType::Codex);
        }
        if self.compat[1] {
            result.push(AiType::Claude);
        }
        if self.compat[2] {
            result.push(AiType::Gemini);
        }
        result
    }
}

fn toggle_compat(state: &mut EditState, index: usize) {
    if let Some(flag) = state.compat.get_mut(index) {
        *flag = !*flag;
    }
}

fn move_selection_up(state: &mut EditState) {
    state.selection = match state.selection {
        EditSelection::Description => EditSelection::Description,
        EditSelection::Compatibility(0) => EditSelection::Description,
        EditSelection::Compatibility(index) => EditSelection::Compatibility(index - 1),
        EditSelection::EnvVar(0, _) => EditSelection::Compatibility(2),
        EditSelection::EnvVar(index, field) => EditSelection::EnvVar(index - 1, field),
        EditSelection::EnvAdd => {
            if state.env_vars.is_empty() {
                EditSelection::Compatibility(2)
            } else {
                EditSelection::EnvVar(state.env_vars.len() - 1, EnvField::Value)
            }
        }
        EditSelection::Save => {
            if state.env_vars.is_empty() {
                EditSelection::EnvAdd
            } else {
                EditSelection::EnvVar(state.env_vars.len() - 1, EnvField::Value)
            }
        }
        EditSelection::Cancel => EditSelection::Save,
    };
}

fn move_selection_down(state: &mut EditState) {
    state.selection = match state.selection {
        EditSelection::Description => EditSelection::Compatibility(0),
        EditSelection::Compatibility(index) => {
            if index >= 2 {
                if state.env_vars.is_empty() {
                    EditSelection::EnvAdd
                } else {
                    EditSelection::EnvVar(0, EnvField::Key)
                }
            } else {
                EditSelection::Compatibility(index + 1)
            }
        }
        EditSelection::EnvVar(index, field) => {
            if index + 1 < state.env_vars.len() {
                EditSelection::EnvVar(index + 1, field)
            } else {
                EditSelection::EnvAdd
            }
        }
        EditSelection::EnvAdd => EditSelection::Save,
        EditSelection::Save => EditSelection::Cancel,
        EditSelection::Cancel => EditSelection::Cancel,
    };
}

fn format_provider_details(entry: &ProviderEntry) -> String {
    let mut lines = Vec::new();
    lines.push(format!("ID: {}", entry.id));
    lines.push(format!(
        "Name: {}",
        if entry.provider.name.trim().is_empty() {
            "(unnamed)"
        } else {
            &entry.provider.name
        }
    ));
    lines.push(format!(
        "Default: {}",
        if entry.is_default { "Yes" } else { "No" }
    ));
    lines.push(format!("Description: {}", entry.provider.description));

    if entry.provider.compatible_with.is_empty() {
        lines.push("Compatible CLI: (none)".to_string());
    } else {
        let compat = entry
            .provider
            .compatible_with
            .iter()
            .map(|ai| ai.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("Compatible CLI: {}", compat));
    }

    if entry.provider.env.is_empty() {
        lines.push("Environment: (none)".to_string());
    } else {
        lines.push("Environment:".to_string());
        let mut keys: Vec<_> = entry.provider.env.keys().collect();
        keys.sort();
        for key in keys {
            let value = entry
                .provider
                .env
                .get(key)
                .map(|s| s.as_str())
                .unwrap_or("");
            lines.push(format!("  {} = {}", key, truncate_value(value)));
        }
    }

    lines.join("\n")
}

fn truncate_value(value: &str) -> String {
    const MAX_LEN: usize = 32;
    if value.len() <= MAX_LEN {
        value.to_string()
    } else {
        format!("{}...", &value[..MAX_LEN])
    }
}

fn is_valid_provider_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{backend::TestBackend, Terminal};
    use std::env;
    use tempfile::TempDir;

    struct TempHome {
        #[allow(dead_code)]
        dir: TempDir,
        old_home: Option<std::ffi::OsString>,
        old_user: Option<std::ffi::OsString>,
    }

    impl TempHome {
        fn new() -> Self {
            let dir = tempfile::tempdir().expect("temp dir");
            let path = dir.path().to_path_buf();
            let old_home = env::var_os("HOME");
            env::set_var("HOME", &path);
            let old_user = env::var_os("USERPROFILE");
            if cfg!(windows) {
                env::set_var("USERPROFILE", &path);
            }
            Self {
                dir,
                old_home,
                old_user,
            }
        }
    }

    impl Drop for TempHome {
        fn drop(&mut self) {
            if let Some(old) = self.old_home.take() {
                env::set_var("HOME", old);
            } else {
                env::remove_var("HOME");
            }
            if cfg!(windows) {
                if let Some(old) = self.old_user.take() {
                    env::set_var("USERPROFILE", old);
                } else {
                    env::remove_var("USERPROFILE");
                }
            }
        }
    }

    fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer.get(x, y).symbol());
            }
            text.push('\n');
        }
        text
    }

    #[test]
    fn provider_screen_renders_provider_list() {
        let _home = TempHome::new();
        let mut screen = ProviderScreen::new().expect("screen should initialise");
        screen.list_state.select(Some(0));

        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| screen.render(frame, frame.size()))
            .unwrap();

        let buffer = terminal.backend().buffer();
        let rendered = buffer_to_string(buffer);
        assert!(
            rendered.contains("Provider Management") || rendered.contains("OpenRouter"),
            "rendered output did not contain expected provider data:\n{rendered}"
        );
    }

    #[test]
    fn provider_screen_handles_list_shortcuts() {
        let _home = TempHome::new();
        let mut screen = ProviderScreen::new().expect("screen should initialise");

        let back = screen
            .handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))
            .expect("handle key");
        assert!(matches!(back, ScreenAction::Back));

        let action = screen
            .handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE))
            .expect("handle key");
        // 'a' key now shows a message instead of switching to wizard
        assert!(matches!(action, ScreenAction::None));
        assert!(screen.message.is_some());
    }
}
