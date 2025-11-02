//! Provider management screen

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::collections::HashMap;

use super::{Screen, ScreenAction, ScreenType};
use crate::provider::env_mapping::{EnvVarMapping, get_env_vars_for_ai_type};
use crate::provider::{AiType, Provider, ProviderManager};
use crate::tui::widgets::{DialogResult, DialogWidget, InputWidget, ListWidget};

/// Environment variable definition for TUI
#[derive(Debug, Clone)]
struct EnvVarDef {
    pub key: String,
    pub description: String,
    pub required: bool,
    pub sensitive: bool,
}

impl From<EnvVarMapping> for EnvVarDef {
    fn from(mapping: EnvVarMapping) -> Self {
        let key_upper = mapping.key.to_ascii_uppercase();
        let sensitive = ["KEY", "SECRET", "TOKEN", "PASSWORD"]
            .iter()
            .any(|marker| key_upper.contains(marker));
        EnvVarDef {
            key: mapping.key.to_string(),
            description: mapping.description.to_string(),
            required: mapping.required,
            sensitive,
        }
    }
}

#[derive(Clone)]
struct ProviderListEntry {
    id: String,
    label: String,
    is_default: bool,
}

fn mask_sensitive_value(key: &str, value: &str) -> String {
    let upper_key = key.to_ascii_uppercase();
    let is_sensitive = ["KEY", "SECRET", "TOKEN", "PASSWORD"]
        .iter()
        .any(|marker| upper_key.contains(marker));

    if !is_sensitive {
        return value.to_string();
    }

    if value.is_empty() {
        "(not set)".to_string()
    } else {
        let prefix: String = value.chars().take(4).collect();
        format!("{prefix}***")
    }
}

/// Provider screen mode
enum ProviderMode {
    List,
    AddNameInput,
    AddDescriptionInput {
        name: String,
    },
    AddSelectTypes {
        name: String,
        description: String,
    },
    AddEnvInput {
        name: String,
        description: String,
        compatible_types: Vec<AiType>,
        env_defs: Vec<(AiType, EnvVarDef)>,
        env_vars: HashMap<String, String>,
        current_idx: usize,
    },
    DeleteConfirm {
        provider_id: String,
        dialog: DialogWidget,
    },
    Dialog(DialogWidget),
}

/// Provider list screen
pub struct ProviderScreen {
    provider_manager: ProviderManager,
    list_widget: ListWidget<ProviderListEntry>,
    mode: ProviderMode,
    input_widget: InputWidget,
    types_selected: Vec<bool>, // For multi-select AI types
    pending_focus_id: Option<String>,
}

impl ProviderScreen {
    pub fn new() -> Result<Self> {
        let provider_manager = ProviderManager::new()?;
        let entries = Self::build_provider_entries(&provider_manager);
        let list_widget = ListWidget::new("Providers".to_string(), entries);

        Ok(Self {
            provider_manager,
            list_widget,
            mode: ProviderMode::List,
            input_widget: InputWidget::new("Input".to_string()),
            types_selected: vec![false, false, false], // codex, claude, gemini
            pending_focus_id: None,
        })
    }

    fn refresh_list(&mut self) -> Result<()> {
        let entries = Self::build_provider_entries(&self.provider_manager);
        let target_focus = self.pending_focus_id.take();
        let selected_id =
            target_focus.or_else(|| self.list_widget.selected().map(|entry| entry.id.clone()));

        self.list_widget.set_items(entries);

        if self.list_widget.items().is_empty() {
            self.list_widget.select(None);
        } else if let Some(id) = selected_id {
            if let Some(index) = self
                .list_widget
                .items()
                .iter()
                .position(|entry| entry.id == id)
            {
                self.list_widget.select(Some(index));
            }
        } else if self.list_widget.selected_index().is_none() {
            self.list_widget.select(Some(0));
        }

        Ok(())
    }

    fn get_all_env_vars_for_types(types: &[AiType]) -> Vec<(AiType, EnvVarDef)> {
        let mut all_vars = Vec::new();
        for ai_type in types {
            let vars = get_env_vars_for_ai_type(ai_type.clone());
            for var in vars {
                all_vars.push((ai_type.clone(), var.into()));
            }
        }
        all_vars
    }

    fn collect_selected_types(flags: &[bool]) -> Vec<AiType> {
        let mut types = Vec::new();
        if flags.get(0).copied().unwrap_or(false) {
            types.push(AiType::Codex);
        }
        if flags.get(1).copied().unwrap_or(false) {
            types.push(AiType::Claude);
        }
        if flags.get(2).copied().unwrap_or(false) {
            types.push(AiType::Gemini);
        }
        types
    }

    fn build_provider_entries(manager: &ProviderManager) -> Vec<ProviderListEntry> {
        let default_id = manager.default_provider_name().to_string();

        manager
            .list_providers()
            .into_iter()
            .map(|(id, provider)| {
                let label = if provider.name.is_empty() {
                    id.clone()
                } else if provider.name == *id {
                    provider.name.clone()
                } else {
                    format!("{} ({})", provider.name, id)
                };

                ProviderListEntry {
                    id: id.clone(),
                    label,
                    is_default: *id == default_id,
                }
            })
            .collect()
    }

    fn format_provider_detail(&self, provider_id: &str) -> Option<String> {
        let provider = self.provider_manager.get_provider(provider_id).ok()?;

        let mut lines = Vec::new();
        lines.push(format!("Provider ID: {}", provider_id));
        lines.push(format!("Display Name: {}", provider.name));
        if !provider.description.is_empty() {
            lines.push(format!("Description: {}", provider.description));
        }

        if !provider.compatible_with.is_empty() {
            let compat = provider
                .compatible_with
                .iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("Compatible CLI: {}", compat));
        }

        if !provider.support_modes.is_empty() {
            lines.push(String::from("Support Modes:"));
            for mode in &provider.support_modes {
                lines.push(format!(
                    "  - {} ({})",
                    mode.name,
                    mode.mode_type.to_string()
                ));
            }
        }

        lines.push(String::new());
        lines.push("Environment Variables:".to_string());

        for ai_type in &[AiType::Codex, AiType::Claude, AiType::Gemini] {
            if provider.compatible_with.contains(ai_type) {
                lines.push(format!("  {}:", ai_type));
                let mappings = get_env_vars_for_ai_type(ai_type.clone());
                for mapping in mappings {
                    let value = provider
                        .env
                        .get(mapping.key)
                        .map(|val| mask_sensitive_value(mapping.key, val))
                        .unwrap_or_else(|| "(not set)".to_string());
                    let required = if mapping.required { " (required)" } else { "" };
                    lines.push(format!("    {} = {}{}", mapping.key, value, required));
                }
            }
        }

        Some(lines.join("\n"))
    }

    fn reset_to_list(&mut self) {
        self.mode = ProviderMode::List;
        self.input_widget = InputWidget::new("Input".to_string());
        self.types_selected = vec![false, false, false];
    }

    fn begin_add_flow(&mut self) {
        self.input_widget = InputWidget::new("Provider ID".to_string());
        self.input_widget.set_focused(true);
        self.types_selected = vec![false, false, false];
        self.mode = ProviderMode::AddNameInput;
    }

    fn env_prompt_label(env_def: &(AiType, EnvVarDef)) -> String {
        let (ai_type, var) = env_def;
        let requirement = if var.required { "required" } else { "optional" };
        format!(
            "{} [{} | {}]\n{}",
            var.key, ai_type, requirement, var.description
        )
    }

    fn build_env_input_widget(
        env_defs: &[(AiType, EnvVarDef)],
        current_idx: usize,
        env_vars: &HashMap<String, String>,
    ) -> InputWidget {
        let env_def = &env_defs[current_idx];
        let label = Self::env_prompt_label(env_def);
        let existing = env_vars.get(&env_def.1.key).cloned().unwrap_or_default();
        let mut input = InputWidget::new(label)
            .with_value(existing)
            .masked(env_def.1.sensitive);
        input.set_focused(true);
        input
    }

    fn finish_provider_creation(
        &mut self,
        provider_id: String,
        description: String,
        compatible_types: Vec<AiType>,
        env_vars: HashMap<String, String>,
    ) -> Result<()> {
        let provider = Provider {
            name: provider_id.clone(),
            description,
            icon: None,
            official: false,
            protected: false,
            custom: true,
            support_modes: vec![],
            compatible_with: compatible_types,
            validation_endpoint: None,
            category: None,
            website: None,
            regions: vec![],
            env: env_vars,
        };

        match self
            .provider_manager
            .add_provider(provider_id.clone(), provider)
        {
            Ok(()) => {
                self.pending_focus_id = Some(provider_id.clone());
                self.refresh_list()?;
                self.mode = ProviderMode::Dialog(DialogWidget::info(
                    "Success".to_string(),
                    format!("Provider '{}' added successfully", provider_id),
                ));
            }
            Err(err) => {
                self.mode = ProviderMode::Dialog(DialogWidget::error(
                    "Error".to_string(),
                    format!("Failed to add provider: {}", err),
                ));
            }
        }
        Ok(())
    }
}

impl Screen for ProviderScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        match &self.mode {
            ProviderMode::List => {
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(0),
                        Constraint::Length(3),
                    ])
                    .split(area);

                let title = Paragraph::new("Provider Management")
                    .style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL));
                frame.render_widget(title, layout[0]);

                let content = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
                    .split(layout[1]);

                let selected_id = self.list_widget.selected().map(|entry| entry.id.clone());

                self.list_widget.render(frame, content[0], |entry, _| {
                    let default_marker = if entry.is_default { " [default]" } else { "" };
                    format!("{}{}", entry.label, default_marker)
                });

                let detail_text = selected_id
                    .and_then(|id| self.format_provider_detail(&id))
                    .unwrap_or_else(|| {
                        "Select a provider to see configuration details.".to_string()
                    });

                let detail = Paragraph::new(detail_text).wrap(Wrap { trim: true }).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Provider Details"),
                );
                frame.render_widget(detail, content[1]);

                let help = Paragraph::new(
                    "[Enter] Set Default  [A] Add  [E] Edit  [D] Delete  [R] Refresh  [ESC] Back",
                )
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
                frame.render_widget(help, layout[2]);
            }
            ProviderMode::AddNameInput | ProviderMode::AddDescriptionInput { .. } => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(10),
                        Constraint::Length(3),
                    ])
                    .split(area);

                let title_text = match &self.mode {
                    ProviderMode::AddNameInput => "Add Provider - Enter Name",
                    ProviderMode::AddDescriptionInput { .. } => "Add Provider - Enter Description",
                    _ => "",
                };

                let title = Paragraph::new(title_text)
                    .style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL));
                frame.render_widget(title, chunks[0]);

                self.input_widget.render(frame, chunks[1]);

                let help = Paragraph::new("[Enter] Continue  [ESC] Cancel")
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL));
                frame.render_widget(help, chunks[2]);
            }
            ProviderMode::AddSelectTypes { .. } => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(10),
                        Constraint::Length(3),
                    ])
                    .split(area);

                let title = Paragraph::new("Add Provider - Select Compatible AI Types")
                    .style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL));
                frame.render_widget(title, chunks[0]);

                let types = vec![
                    format!("[{}] Codex", if self.types_selected[0] { "X" } else { " " }),
                    format!(
                        "[{}] Claude",
                        if self.types_selected[1] { "X" } else { " " }
                    ),
                    format!(
                        "[{}] Gemini",
                        if self.types_selected[2] { "X" } else { " " }
                    ),
                ];
                let types_text = types.join("\n");
                let content = Paragraph::new(types_text)
                    .block(Block::default().borders(Borders::ALL).title("AI Types"));
                frame.render_widget(content, chunks[1]);

                let help = Paragraph::new(
                    "[Space] Toggle  [1/2/3] Quick select  [Enter] Continue  [ESC] Cancel",
                )
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
                frame.render_widget(help, chunks[2]);
            }
            ProviderMode::AddEnvInput {
                name,
                env_defs,
                current_idx,
                ..
            } => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(10),
                        Constraint::Length(3),
                    ])
                    .split(area);

                let title =
                    Paragraph::new(format!("Add Provider '{}' - Environment Variables", name))
                        .style(
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )
                        .alignment(Alignment::Center)
                        .block(Block::default().borders(Borders::ALL));
                frame.render_widget(title, chunks[0]);

                let total = env_defs.len();

                if *current_idx < total {
                    let (ai_type, var) = &env_defs[*current_idx];
                    let progress =
                        format!("Variable {} of {} | {}", current_idx + 1, total, ai_type);
                    let requirement = if var.required { "Required" } else { "Optional" };

                    let info_text = format!("{}\n{}\n{}", progress, var.key, var.description);
                    let info = Paragraph::new(info_text)
                        .block(Block::default().borders(Borders::ALL).title(requirement));

                    let inner_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Length(4), Constraint::Min(5)])
                        .split(chunks[1]);

                    frame.render_widget(info, inner_chunks[0]);
                    self.input_widget.render(frame, inner_chunks[1]);
                } else {
                    let info = Paragraph::new(
                        "All environment variables captured. Press Enter to finish.",
                    )
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL));
                    frame.render_widget(info, chunks[1]);
                }

                let help = Paragraph::new("[Enter] Next  [ESC] Cancel")
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL));
                frame.render_widget(help, chunks[2]);
            }
            ProviderMode::DeleteConfirm { dialog, .. } => {
                dialog.render(frame, area);
            }
            ProviderMode::Dialog(dialog) => {
                dialog.render(frame, area);
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        match &mut self.mode {
            ProviderMode::List => {
                if self.list_widget.handle_key(key) {
                    return Ok(ScreenAction::None);
                }

                match key.code {
                    KeyCode::Char('a') | KeyCode::Char('A') => {
                        self.begin_add_flow();
                        Ok(ScreenAction::None)
                    }
                    KeyCode::Char('e') | KeyCode::Char('E') => {
                        if let Some(entry) = self.list_widget.selected() {
                            Ok(ScreenAction::SwitchTo(ScreenType::ProviderEdit(
                                entry.id.clone(),
                            )))
                        } else {
                            Ok(ScreenAction::None)
                        }
                    }
                    KeyCode::Char('d') | KeyCode::Char('D') => {
                        if let Some(entry) = self.list_widget.selected() {
                            let dialog = DialogWidget::confirm(
                                "Confirm Delete".to_string(),
                                format!(
                                    "Are you sure you want to delete provider '{}'?",
                                    entry.label
                                ),
                            );
                            self.mode = ProviderMode::DeleteConfirm {
                                provider_id: entry.id.clone(),
                                dialog,
                            };
                        }
                        Ok(ScreenAction::None)
                    }
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        self.refresh_list()?;
                        Ok(ScreenAction::None)
                    }
                    KeyCode::Enter => {
                        if let Some(entry) = self.list_widget.selected().cloned() {
                            let entry_id = entry.id.clone();
                            let entry_label = entry.label.clone();
                            match self.provider_manager.set_default(&entry_id) {
                                Ok(()) => {
                                    self.refresh_list()?;
                                    self.mode = ProviderMode::Dialog(DialogWidget::info(
                                        "Default Provider".to_string(),
                                        format!("'{}' is now the default provider", entry_label),
                                    ));
                                }
                                Err(err) => {
                                    self.mode = ProviderMode::Dialog(DialogWidget::error(
                                        "Error".to_string(),
                                        format!("Failed to set default provider: {}", err),
                                    ));
                                }
                            }
                        }
                        Ok(ScreenAction::None)
                    }
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                        Ok(ScreenAction::Back)
                    }
                    _ => Ok(ScreenAction::None),
                }
            }
            ProviderMode::AddNameInput => {
                if self.input_widget.handle_key(key) {
                    return Ok(ScreenAction::None);
                }

                match key.code {
                    KeyCode::Enter => {
                        let name = self.input_widget.value().trim().to_string();
                        if name.is_empty() {
                            self.mode = ProviderMode::Dialog(DialogWidget::warning(
                                "Validation".to_string(),
                                "Provider ID cannot be empty".to_string(),
                            ));
                        } else if self.provider_manager.get_provider(&name).is_ok() {
                            self.mode = ProviderMode::Dialog(DialogWidget::warning(
                                "Validation".to_string(),
                                format!("Provider '{}' already exists", name),
                            ));
                        } else {
                            self.input_widget = InputWidget::new("Description".to_string());
                            self.input_widget.set_focused(true);
                            self.mode = ProviderMode::AddDescriptionInput { name };
                        }
                        Ok(ScreenAction::None)
                    }
                    KeyCode::Esc => {
                        self.reset_to_list();
                        Ok(ScreenAction::None)
                    }
                    _ => Ok(ScreenAction::None),
                }
            }
            ProviderMode::AddDescriptionInput { name } => {
                if self.input_widget.handle_key(key) {
                    return Ok(ScreenAction::None);
                }

                match key.code {
                    KeyCode::Enter => {
                        let description = self.input_widget.value().to_string();
                        self.types_selected = vec![false, false, false];
                        self.mode = ProviderMode::AddSelectTypes {
                            name: name.clone(),
                            description,
                        };
                        Ok(ScreenAction::None)
                    }
                    KeyCode::Esc => {
                        self.reset_to_list();
                        Ok(ScreenAction::None)
                    }
                    _ => Ok(ScreenAction::None),
                }
            }
            ProviderMode::AddSelectTypes { name, description } => match key.code {
                KeyCode::Char('1') => {
                    self.types_selected[0] = !self.types_selected[0];
                    Ok(ScreenAction::None)
                }
                KeyCode::Char('2') => {
                    self.types_selected[1] = !self.types_selected[1];
                    Ok(ScreenAction::None)
                }
                KeyCode::Char('3') => {
                    self.types_selected[2] = !self.types_selected[2];
                    Ok(ScreenAction::None)
                }
                KeyCode::Enter => {
                    let selected_types = Self::collect_selected_types(&self.types_selected);
                    if selected_types.is_empty() {
                        self.mode = ProviderMode::Dialog(DialogWidget::warning(
                            "Validation".to_string(),
                            "Please select at least one compatible AI type".to_string(),
                        ));
                    } else {
                        let provider_name = name.clone();
                        let provider_description = description.clone();
                        let env_defs = Self::get_all_env_vars_for_types(&selected_types);
                        let env_vars = HashMap::new();

                        if env_defs.is_empty() {
                            self.finish_provider_creation(
                                provider_name,
                                provider_description,
                                selected_types,
                                env_vars,
                            )?;
                        } else {
                            self.input_widget =
                                Self::build_env_input_widget(&env_defs, 0, &env_vars);
                            self.mode = ProviderMode::AddEnvInput {
                                name: provider_name,
                                description: provider_description,
                                compatible_types: selected_types,
                                env_defs,
                                current_idx: 0,
                                env_vars,
                            };
                        }
                    }
                    Ok(ScreenAction::None)
                }
                KeyCode::Esc => {
                    self.reset_to_list();
                    Ok(ScreenAction::None)
                }
                _ => Ok(ScreenAction::None),
            },
            ProviderMode::AddEnvInput {
                name,
                description,
                compatible_types,
                env_defs,
                env_vars,
                current_idx,
            } => {
                if self.input_widget.handle_key(key) {
                    return Ok(ScreenAction::None);
                }

                match key.code {
                    KeyCode::Enter => {
                        if let Some((_, var)) = env_defs.get(*current_idx) {
                            let value = self.input_widget.value().trim().to_string();
                            if var.required && value.is_empty() {
                                self.mode = ProviderMode::Dialog(DialogWidget::warning(
                                    "Validation".to_string(),
                                    format!("{} is required", var.key),
                                ));
                                return Ok(ScreenAction::None);
                            }

                            if value.is_empty() {
                                env_vars.remove(&var.key);
                            } else {
                                env_vars.insert(var.key.clone(), value);
                            }

                            *current_idx += 1;
                            if *current_idx >= env_defs.len() {
                                let env_snapshot = env_vars.clone();
                                let provider_name = name.clone();
                                let provider_description = description.clone();
                                let compatible_clone = compatible_types.clone();
                                self.finish_provider_creation(
                                    provider_name,
                                    provider_description,
                                    compatible_clone,
                                    env_snapshot,
                                )?;
                            } else {
                                self.input_widget =
                                    Self::build_env_input_widget(env_defs, *current_idx, env_vars);
                            }
                        } else {
                            let env_snapshot = env_vars.clone();
                            let provider_name = name.clone();
                            let provider_description = description.clone();
                            let compatible_clone = compatible_types.clone();
                            self.finish_provider_creation(
                                provider_name,
                                provider_description,
                                compatible_clone,
                                env_snapshot,
                            )?;
                        }
                        Ok(ScreenAction::None)
                    }
                    KeyCode::Esc => {
                        self.reset_to_list();
                        Ok(ScreenAction::None)
                    }
                    _ => Ok(ScreenAction::None),
                }
            }
            ProviderMode::DeleteConfirm {
                provider_id,
                dialog,
            } => {
                let result = dialog.handle_key(key);

                match result {
                    DialogResult::Confirmed => {
                        let target = provider_id.clone();
                        match self.provider_manager.remove_provider(&target) {
                            Ok(()) => {
                                self.refresh_list()?;
                                self.mode = ProviderMode::Dialog(DialogWidget::info(
                                    "Provider Deleted".to_string(),
                                    format!("Provider '{}' has been removed", target),
                                ));
                            }
                            Err(err) => {
                                self.mode = ProviderMode::Dialog(DialogWidget::error(
                                    "Error".to_string(),
                                    format!("Failed to delete provider: {}", err),
                                ));
                            }
                        }
                        Ok(ScreenAction::None)
                    }
                    DialogResult::Cancelled | DialogResult::Closed => {
                        self.reset_to_list();
                        Ok(ScreenAction::None)
                    }
                    DialogResult::None => Ok(ScreenAction::None),
                }
            }
            ProviderMode::Dialog(dialog) => {
                let result = dialog.handle_key(key);

                match result {
                    DialogResult::Closed | DialogResult::Confirmed | DialogResult::Cancelled => {
                        self.reset_to_list();
                        Ok(ScreenAction::None)
                    }
                    DialogResult::None => Ok(ScreenAction::None),
                }
            }
        }
    }

    fn update(&mut self) -> Result<()> {
        Ok(())
    }
}
