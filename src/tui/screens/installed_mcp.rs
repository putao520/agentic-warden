//! Installed MCP screen.

use std::collections::HashMap;

use anyhow::{anyhow, Result};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use tracing::info;

use crate::commands::mcp::registry::browse::EnvInputState;
use crate::commands::mcp::registry::types::EnvVarSpec;
use crate::mcp_routing::config::McpConfigManager;

use super::render_helpers::{DialogResult, DialogState};
use super::{Screen, ScreenAction};

#[derive(Clone)]
struct InstalledMcpListItem {
    name: String,
    description: Option<String>,
    enabled: bool,
    env_var_count: usize,
    source: String,
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StatusFilter {
    All,
    Enabled,
    Disabled,
}

impl StatusFilter {
    fn next(self) -> Self {
        match self {
            StatusFilter::All => StatusFilter::Enabled,
            StatusFilter::Enabled => StatusFilter::Disabled,
            StatusFilter::Disabled => StatusFilter::All,
        }
    }

    fn matches(self, enabled: bool) -> bool {
        match self {
            StatusFilter::All => true,
            StatusFilter::Enabled => enabled,
            StatusFilter::Disabled => !enabled,
        }
    }

    fn label(self) -> &'static str {
        match self {
            StatusFilter::All => "all",
            StatusFilter::Enabled => "enabled",
            StatusFilter::Disabled => "disabled",
        }
    }
}

enum ViewMode {
    List,
    Details(usize),
    Edit(EditEnvState),
}

pub struct InstalledMcpScreen {
    items: Vec<InstalledMcpListItem>,
    filtered_items: Vec<usize>,
    list_state: ListState,
    search_query: String,
    search_mode: bool,
    status_filter: StatusFilter,
    view: ViewMode,
    message: Option<String>,
    config_path_label: String,
}

impl InstalledMcpScreen {
    pub fn new() -> Result<Self> {
        let mut screen = Self {
            items: Vec::new(),
            filtered_items: Vec::new(),
            list_state: ListState::default(),
            search_query: String::new(),
            search_mode: false,
            status_filter: StatusFilter::All,
            view: ViewMode::List,
            message: None,
            config_path_label: "~/.aiw/mcp.json".to_string(),
        };
        let _ = screen.refresh_items();
        Ok(screen)
    }

    fn refresh_items(&mut self) -> Result<()> {
        self.items.clear();
        self.message = None;

        match McpConfigManager::load() {
            Ok(manager) => {
                self.config_path_label = manager.path().display().to_string();
                let mut entries: Vec<(String, _)> = manager
                    .config()
                    .mcp_servers
                    .iter()
                    .map(|(name, cfg)| (name.clone(), cfg.clone()))
                    .collect();
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                self.items = entries
                    .into_iter()
                    .map(|(name, cfg)| InstalledMcpListItem {
                        name,
                        description: cfg.description.clone(),
                        enabled: cfg.enabled.unwrap_or(true),
                        env_var_count: cfg.env.len(),
                        source: cfg.source.clone().unwrap_or_else(|| "unknown".to_string()),
                        command: cfg.command.clone(),
                        args: cfg.args.clone(),
                        env: cfg.env.clone(),
                    })
                    .collect();

                if self.items.is_empty() {
                    self.message = Some("No installed MCP servers found".to_string());
                }
            }
            Err(err) => {
                if is_empty_config_error(&err) {
                    self.message = Some("No installed MCP servers found".to_string());
                } else {
                    self.message = Some(format!("Failed to load MCP config: {}", err));
                }
            }
        }

        self.apply_filter();
        Ok(())
    }

    fn apply_filter(&mut self) {
        let query = self.search_query.to_lowercase();
        self.filtered_items = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                let matches_query = query.is_empty()
                    || item.name.to_lowercase().contains(&query)
                    || item
                        .description
                        .as_ref()
                        .map(|desc| desc.to_lowercase().contains(&query))
                        .unwrap_or(false);
                matches_query && self.status_filter.matches(item.enabled)
            })
            .map(|(idx, _)| idx)
            .collect();

        if let Some(selected) = self.list_state.selected() {
            if selected >= self.filtered_items.len() {
                self.list_state
                    .select(if self.filtered_items.is_empty() { None } else { Some(0) });
            }
        } else if !self.filtered_items.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    fn selected_item_index(&self) -> Option<usize> {
        let selected = self.list_state.selected()?;
        self.filtered_items.get(selected).copied()
    }

    fn selected_item(&self) -> Option<&InstalledMcpListItem> {
        self.selected_item_index()
            .and_then(|idx| self.items.get(idx))
    }

    fn move_selection_up(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if selected > 0 {
                self.list_state.select(Some(selected - 1));
            }
        }
    }

    fn move_selection_down(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if selected + 1 < self.filtered_items.len() {
                self.list_state.select(Some(selected + 1));
            }
        }
    }

    fn enter_details(&mut self) {
        if let Some(idx) = self.selected_item_index() {
            self.view = ViewMode::Details(idx);
            self.search_mode = false;
        }
    }

    fn begin_edit(&mut self) -> Result<()> {
        let item = match self.selected_item().cloned() {
            Some(item) => item,
            None => {
                self.message = Some("No MCP server selected".to_string());
                return Ok(());
            }
        };

        let env_specs = build_env_specs(&item);
        let edit_state = EditEnvState::new(item.name.clone(), env_specs, item.env.clone());
        self.view = ViewMode::Edit(edit_state);
        self.search_mode = false;
        Ok(())
    }

    fn header_line(&self) -> Line<'static> {
        let title = format!(
            "Installed MCPs ({}/{})",
            self.filtered_items.len(),
            self.items.len()
        );
        let filter = format!("Filter: {}", self.status_filter.label());
        let search = if self.search_query.is_empty() {
            "Search: /".to_string()
        } else {
            format!("Search: {}", self.search_query)
        };

        Line::from(vec![
            Span::styled(title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" | "),
            Span::styled(filter, Style::default().fg(Color::Yellow)),
            Span::raw(" | "),
            Span::styled(search, Style::default().fg(Color::Gray)),
        ])
    }

    fn footer_text(&self) -> String {
        let mut lines = Vec::new();

        match &self.view {
            ViewMode::List => {
                if let Some(message) = &self.message {
                    lines.push(message.clone());
                }
                if self.search_mode {
                    lines.push(format!("Search: {}_", self.search_query));
                    lines.push("Enter: confirm  Esc: cancel".to_string());
                } else {
                    lines.push("Enter: details  e: edit  /: search  f: filter  Esc: back".to_string());
                    lines.push(format!("Config: {}", self.config_path_label));
                }
            }
            ViewMode::Details(_) => {
                if let Some(message) = &self.message {
                    lines.push(message.clone());
                }
                lines.push("Esc: back".to_string());
            }
            ViewMode::Edit(edit_state) => {
                if let Some(message) = &edit_state.message {
                    lines.push(message.clone());
                }
                if edit_state.env_input.is_complete() {
                    lines.push("s: save  Esc: cancel".to_string());
                } else {
                    lines.push("Enter: next  Backspace: delete  Esc: cancel".to_string());
                }
            }
        }

        lines.join("\n")
    }

    fn render_list(&mut self, frame: &mut Frame, area: Rect) {
        if self.items.is_empty() {
            let paragraph = Paragraph::new(
                self.message
                    .clone()
                    .unwrap_or_else(|| "No installed MCP servers found".to_string()),
            )
            .block(Block::default().borders(Borders::ALL).title("Installed MCPs"))
            .wrap(Wrap { trim: true });
            frame.render_widget(paragraph, area);
            return;
        }

        let items: Vec<ListItem> = self
            .filtered_items
            .iter()
            .filter_map(|idx| self.items.get(*idx))
            .map(|item| {
                let status = if item.enabled { "ENABLED" } else { "DISABLED" };
                let status_style = if item.enabled {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                };
                let header = Line::from(vec![
                    Span::styled(format!("{:<8}", status), status_style),
                    Span::raw(" "),
                    Span::styled(
                        item.name.clone(),
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format!("  env: {}", item.env_var_count)),
                ]);
                let desc = item
                    .description
                    .as_deref()
                    .unwrap_or("No description");
                let detail = Line::from(Span::styled(
                    desc.to_string(),
                    Style::default().fg(Color::Gray),
                ));
                ListItem::new(vec![header, detail])
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Installed MCPs"))
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol(">> ");
        frame.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn render_details(&self, frame: &mut Frame, area: Rect, idx: usize) {
        let Some(item) = self.items.get(idx) else {
            return;
        };

        let status = if item.enabled { "Enabled" } else { "Disabled" };
        let mut lines = Vec::new();
        lines.push(Line::from(vec![
            Span::styled("Name: ", Style::default().fg(Color::Cyan)),
            Span::styled(item.name.clone(), Style::default().add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Cyan)),
            Span::raw(status),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Source: ", Style::default().fg(Color::Cyan)),
            Span::raw(item.source.clone()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Command: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{} {}", item.command, item.args.join(" ")).trim().to_string()),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Environment Variables",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));

        if item.env.is_empty() {
            lines.push(Line::from("  (none)"));
        } else {
            let mut keys: Vec<_> = item.env.keys().cloned().collect();
            keys.sort();
            for key in keys {
                let value = item.env.get(&key).cloned().unwrap_or_default();
                lines.push(Line::from(format!("  {}={}", key, value)));
            }
        }

        let paragraph = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Details"))
            .wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
    }

    fn render_edit(&self, frame: &mut Frame, area: Rect, edit_state: &EditEnvState) {
        let mut lines = Vec::new();

        if edit_state.env_input.total_specs() == 0 {
            lines.push(Line::from("No environment variables configured."));
            lines.push(Line::from("Press Esc to return."));
        } else if edit_state.env_input.is_complete() {
            lines.push(Line::from(format!(
                "Editing complete for {}.",
                edit_state.server_name
            )));
            if edit_state.modified {
                lines.push(Line::from("Press 's' to save changes or Esc to cancel."));
            } else {
                lines.push(Line::from("No changes detected. Press Esc to return."));
            }
        } else if let Some(spec) = edit_state.env_input.current_spec() {
            let tag = if spec.required { "required" } else { "optional" };
            lines.push(Line::from(vec![
                Span::styled(
                    spec.name.clone(),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(" ({})", tag)),
            ]));

            if let Some(desc) = &spec.description {
                lines.push(Line::from(desc.clone()));
            }
            if let Some(default) = &spec.default {
                lines.push(Line::from(format!("Default: {}", default)));
            }

            lines.push(Line::from(""));
            lines.push(Line::from(format!(
                "Value: {}_",
                edit_state.env_input.input_buffer()
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(format!(
                "{} / {}",
                edit_state.env_input.current_index() + 1,
                edit_state.env_input.total_specs()
            )));
            lines.push(Line::from(""));
            lines.push(Line::from("Enter: next  Backspace: delete  Esc: cancel"));
            if edit_state.env_input.has_remaining_optional() {
                lines.push(Line::from("Press 'a' to skip optional variables"));
            }
        }

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Edit Env - {}", edit_state.server_name)),
            )
            .wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
    }
}

impl Screen for InstalledMcpScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(4),
            ])
            .split(area);

        let header = Paragraph::new(self.header_line())
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(header, chunks[0]);

        if matches!(&self.view, ViewMode::List) {
            self.render_list(frame, chunks[1]);
        } else if let ViewMode::Details(idx) = &self.view {
            self.render_details(frame, chunks[1], *idx);
        } else if let ViewMode::Edit(edit_state) = &self.view {
            self.render_edit(frame, chunks[1], edit_state);
        }

        let footer = Paragraph::new(self.footer_text())
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        frame.render_widget(footer, chunks[2]);

        if let ViewMode::Edit(edit_state) = &self.view {
            if let Some(dialog) = &edit_state.confirm_dialog {
                dialog.render(frame, area);
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        if matches!(&self.view, ViewMode::Edit(_)) {
            return self.handle_edit_key(key);
        }

        if matches!(&self.view, ViewMode::Details(_)) {
            if key.code == KeyCode::Esc {
                self.view = ViewMode::List;
            }
            return Ok(ScreenAction::None);
        }

        if self.search_mode {
            match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    self.search_mode = false;
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                    self.apply_filter();
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                    self.apply_filter();
                }
                _ => {}
            }
            return Ok(ScreenAction::None);
        }

        match key.code {
            KeyCode::Up => self.move_selection_up(),
            KeyCode::Down => self.move_selection_down(),
            KeyCode::Char('/') => {
                self.search_mode = true;
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.search_query.clear();
                self.apply_filter();
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.status_filter = self.status_filter.next();
                self.apply_filter();
            }
            KeyCode::Enter => self.enter_details(),
            KeyCode::Char('e') | KeyCode::Char('E') => {
                self.begin_edit()?;
            }
            KeyCode::Esc => return Ok(ScreenAction::Back),
            _ => {}
        }

        Ok(ScreenAction::None)
    }
}

impl InstalledMcpScreen {
    fn handle_edit_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        let mut exit_to_list = false;
        let mut refresh_items = false;
        let mut post_message: Option<String> = None;

        {
            let ViewMode::Edit(edit_state) = &mut self.view else {
                return Ok(ScreenAction::None);
            };

            if let Some(dialog) = edit_state.confirm_dialog.as_mut() {
                let dialog_result = match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => DialogResult::Confirmed,
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        DialogResult::Cancelled
                    }
                    _ => dialog.handle_key(key),
                };
                match dialog_result {
                    DialogResult::Confirmed => {
                        edit_state.message = None;
                        match edit_state.validate_required() {
                            Ok(()) => match edit_state.apply_changes() {
                                Ok(()) => {
                                    post_message = Some(format!(
                                        "Saved changes to {}",
                                        edit_state.server_name
                                    ));
                                    exit_to_list = true;
                                    refresh_items = true;
                                }
                                Err(err) => {
                                    edit_state.message = Some(format!("Failed to save: {}", err));
                                }
                            },
                            Err(err) => {
                                edit_state.message = Some(err.to_string());
                            }
                        }
                        edit_state.confirm_dialog = None;
                    }
                    DialogResult::Cancelled | DialogResult::Closed => {
                        edit_state.confirm_dialog = None;
                    }
                    DialogResult::None => {}
                }
            } else if edit_state.env_input.is_complete() {
                match key.code {
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        if !edit_state.modified {
                            edit_state.message = Some("No changes to save".to_string());
                        } else {
                            edit_state.confirm_dialog = Some(DialogState::confirm(
                                "Confirm Save".to_string(),
                                format!(
                                    "Save changes to {}? (y/n)",
                                    edit_state.server_name
                                ),
                            ));
                        }
                    }
                    KeyCode::Esc => {
                        exit_to_list = true;
                    }
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Enter => {
                        edit_state.env_input.next();
                        edit_state.refresh_modified();
                    }
                    KeyCode::Backspace => {
                        edit_state.env_input.pop_char();
                    }
                    KeyCode::Char(c) if c == 'a' || c == 'A' => {
                        let should_skip = edit_state
                            .env_input
                            .current_spec()
                            .map(|spec| !spec.required)
                            .unwrap_or(false);
                        if should_skip {
                            edit_state.env_input.skip_all_optional();
                            edit_state.refresh_modified();
                        } else {
                            edit_state.env_input.push_char(c);
                        }
                    }
                    KeyCode::Char(c) => {
                        edit_state.env_input.push_char(c);
                    }
                    KeyCode::Esc => {
                        exit_to_list = true;
                    }
                    _ => {}
                }
            }
        }

        if exit_to_list {
            self.view = ViewMode::List;
        }
        if refresh_items {
            self.refresh_items()?;
        }
        if let Some(message) = post_message {
            self.message = Some(message);
        }
        Ok(ScreenAction::None)
    }
}

struct EditEnvState {
    server_name: String,
    env_input: EnvInputState,
    original_values: HashMap<String, String>,
    modified: bool,
    confirm_dialog: Option<DialogState>,
    message: Option<String>,
}

impl EditEnvState {
    fn new(
        server_name: String,
        env_specs: Vec<EnvVarSpec>,
        original_values: HashMap<String, String>,
    ) -> Self {
        let mut env_input = EnvInputState::new(env_specs);
        env_input.preload_values(original_values.clone());
        let mut state = Self {
            server_name,
            env_input,
            original_values,
            modified: false,
            confirm_dialog: None,
            message: None,
        };
        state.refresh_modified();
        state
    }

    fn refresh_modified(&mut self) {
        self.modified = self.current_values() != self.original_values;
    }

    fn current_values(&self) -> HashMap<String, String> {
        self.env_input.get_values().into_iter().collect()
    }

    fn validate_required(&self) -> Result<()> {
        let values = self.current_values();
        for spec in self.env_input.env_specs() {
            if spec.required {
                let value = values.get(&spec.name).map(|v| v.trim()).unwrap_or("");
                if value.is_empty() {
                    return Err(anyhow!("Required: {}", spec.name));
                }
            }
        }
        Ok(())
    }

    fn apply_changes(&self) -> Result<()> {
        let updated_values = self.current_values();
        let mut manager = McpConfigManager::load()?;
        manager.update_server_env(&self.server_name, updated_values.clone())?;
        manager.save()?;
        log_env_changes(&self.server_name, &self.original_values, &updated_values);
        Ok(())
    }
}

fn build_env_specs(item: &InstalledMcpListItem) -> Vec<EnvVarSpec> {
    let mut keys: Vec<_> = item.env.keys().cloned().collect();
    keys.sort();
    keys.into_iter()
        .map(|name| EnvVarSpec {
            name,
            description: None,
            required: false,
            default: None,
        })
        .collect()
}

fn log_env_changes(
    server_name: &str,
    original: &HashMap<String, String>,
    updated: &HashMap<String, String>,
) {
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();

    for (key, old_value) in original {
        match updated.get(key) {
            None => removed.push(key.clone()),
            Some(new_value) if new_value != old_value => changed.push(key.clone()),
            _ => {}
        }
    }

    for key in updated.keys() {
        if !original.contains_key(key) {
            added.push(key.clone());
        }
    }

    info!(
        server = server_name,
        added = ?added,
        removed = ?removed,
        updated = ?changed,
        "Updated MCP environment variables"
    );
}

fn is_empty_config_error(err: &anyhow::Error) -> bool {
    err.to_string().contains("No MCP servers configured")
}
