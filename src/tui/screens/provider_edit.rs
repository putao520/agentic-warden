//! Provider edit screen

use anyhow::{Result, bail};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::collections::{HashMap, HashSet};

use super::{Screen, ScreenAction, ScreenType};
use crate::provider::env_mapping::{EnvVarMapping, get_env_vars_for_ai_type};
use crate::provider::{AiType, Provider, ProviderManager};
use super::render_helpers::{DialogResult, DialogState, InputState};

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
        let sensitive = mapping.key.contains("KEY")
            || mapping.key.contains("SECRET")
            || mapping.key.contains("TOKEN");
        EnvVarDef {
            key: mapping.key.to_string(),
            description: mapping.description.to_string(),
            required: mapping.required,
            sensitive,
        }
    }
}

/// Mask sensitive value for display
fn mask_if_sensitive(key: &str, value: &str) -> String {
    let sensitive_keywords = ["KEY", "SECRET", "TOKEN", "PASSWORD"];

    if sensitive_keywords
        .iter()
        .any(|k| key.to_uppercase().contains(k))
    {
        if value.is_empty() {
            "(not set)".to_string()
        } else {
            format!("{}***", &value.chars().take(4).collect::<String>())
        }
    } else {
        value.to_string()
    }
}

/// Edit field type
#[derive(Debug, Clone, PartialEq)]
enum EditField {
    SectionTitle(String),
    DisplayName,
    Description,
    AiType(AiType),
    EnvGroup(AiType),
    EnvVar { ai_type: AiType, key: String },
}

/// Dialog handling behavior
enum DialogOutcome {
    Info { exit_on_close: bool },
    ConfirmDiscard,
}

/// Provider edit mode
enum EditMode {
    Browse,
    EditingField(EditField),
    Dialog {
        widget: DialogState,
        outcome: DialogOutcome,
    },
}

/// Provider edit screen
pub struct ProviderEditScreen {
    provider_id: String,
    provider_manager: ProviderManager,

    // Editable data
    display_name: String,
    description: String,
    ai_types_selected: Vec<bool>, // [Codex, Claude, Gemini]
    env_vars: HashMap<String, String>,

    // UI state
    mode: EditMode,
    selected_field_idx: usize,
    all_fields: Vec<EditField>,
    input_widget: InputState,

    // Original data for comparison
    original_provider: Provider,
}

impl ProviderEditScreen {
    pub fn new(provider_id: String) -> Result<Self> {
        let provider_manager = ProviderManager::new()?;
        let provider = provider_manager.get_provider(&provider_id)?.clone();

        let display_name = provider.name.clone();
        let description = provider.description.clone();
        let env_vars = provider.env.clone();
        let ai_types_selected = vec![
            provider.compatible_with.contains(&AiType::Codex),
            provider.compatible_with.contains(&AiType::Claude),
            provider.compatible_with.contains(&AiType::Gemini),
        ];

        let mut screen = Self {
            provider_id,
            provider_manager,
            display_name,
            description,
            ai_types_selected,
            env_vars,
            mode: EditMode::Browse,
            selected_field_idx: 0,
            all_fields: Vec::new(),
            input_widget: InputState::new("".to_string()),
            original_provider: provider,
        };
        screen.rebuild_fields();
        Ok(screen)
    }

    fn ai_types() -> [AiType; 3] {
        [AiType::Codex, AiType::Claude, AiType::Gemini]
    }

    fn ai_type_label(ai_type: &AiType) -> &'static str {
        match ai_type {
            AiType::Codex => "Codex",
            AiType::Claude => "Claude",
            AiType::Gemini => "Gemini",
        }
    }

    fn ai_type_index(ai_type: &AiType) -> usize {
        match ai_type {
            AiType::Codex => 0,
            AiType::Claude => 1,
            AiType::Gemini => 2,
        }
    }

    fn is_ai_type_selected(&self, ai_type: &AiType) -> bool {
        let idx = Self::ai_type_index(ai_type);
        self.ai_types_selected.get(idx).copied().unwrap_or(false)
    }

    fn remove_env_vars_for(&mut self, ai_type: &AiType) {
        let keys: Vec<String> = get_env_vars_for_ai_type(ai_type.clone())
            .into_iter()
            .map(|mapping| mapping.key.to_string())
            .collect();

        for key in keys {
            self.env_vars.remove(&key);
        }
    }

    fn prune_env_for_unselected(&mut self) {
        let mut allowed = HashSet::new();
        for ai_type in Self::ai_types() {
            if self.is_ai_type_selected(&ai_type) {
                for mapping in get_env_vars_for_ai_type(ai_type.clone()) {
                    allowed.insert(mapping.key.to_string());
                }
            }
        }
        self.env_vars.retain(|key, _| allowed.contains(key));
    }

    fn get_selected_ai_types(&self) -> Vec<AiType> {
        Self::ai_types()
            .iter()
            .enumerate()
            .filter_map(|(idx, ai_type)| {
                if self.ai_types_selected.get(idx).copied().unwrap_or(false) {
                    Some(ai_type.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    fn has_changes(&self) -> bool {
        if self.display_name != self.original_provider.name {
            return true;
        }

        if self.description != self.original_provider.description {
            return true;
        }

        let selected_types = self.get_selected_ai_types();
        if selected_types != self.original_provider.compatible_with {
            return true;
        }

        // Compare environment variables (after pruning unselected types)
        let mut current_env = self.env_vars.clone();
        let mut original_env = self.original_provider.env.clone();

        let mut allowed = HashSet::new();
        for ai_type in selected_types {
            for mapping in get_env_vars_for_ai_type(ai_type.clone()) {
                allowed.insert(mapping.key.to_string());
            }
        }

        current_env.retain(|key, _| allowed.contains(key));
        original_env.retain(|key, _| allowed.contains(key));

        if current_env != original_env {
            return true;
        }

        false
    }

    fn validate_required_env(&self, selected_types: &[AiType]) -> Result<()> {
        for ai_type in selected_types {
            for mapping in get_env_vars_for_ai_type(ai_type.clone())
                .into_iter()
                .filter(|m| m.required)
            {
                let value = self.env_vars.get(mapping.key).map(|s| s.trim());
                if value.is_none() || value == Some("") {
                    bail!(
                        "Missing required environment variable '{}' for {}",
                        mapping.key,
                        Self::ai_type_label(ai_type)
                    );
                }
            }
        }
        Ok(())
    }

    fn rebuild_fields(&mut self) {
        self.all_fields.clear();
        self.all_fields
            .push(EditField::SectionTitle("General".to_string()));
        self.all_fields.push(EditField::DisplayName);
        self.all_fields.push(EditField::Description);
        self.all_fields
            .push(EditField::SectionTitle("Compatibility".to_string()));

        for ai_type in Self::ai_types() {
            self.all_fields.push(EditField::AiType(ai_type.clone()));
        }

        for ai_type in Self::ai_types() {
            if self.is_ai_type_selected(&ai_type) {
                self.all_fields.push(EditField::EnvGroup(ai_type.clone()));
                let vars = get_env_vars_for_ai_type(ai_type.clone());
                for var in vars {
                    self.all_fields.push(EditField::EnvVar {
                        ai_type: ai_type.clone(),
                        key: var.key.to_string(),
                    });
                }
            }
        }

        // Clamp selected index
        if self.selected_field_idx >= self.all_fields.len() {
            self.selected_field_idx = self.all_fields.len().saturating_sub(1);
        }
    }

    fn save_changes(&mut self) -> Result<()> {
        let trimmed_name = self.display_name.trim();
        if trimmed_name.is_empty() {
            bail!("Display name cannot be empty");
        }
        self.display_name = trimmed_name.to_string();

        self.prune_env_for_unselected();

        let compatible_types = self.get_selected_ai_types();
        if compatible_types.is_empty() {
            bail!("Select at least one compatible AI type");
        }

        self.validate_required_env(&compatible_types)?;

        let mut provider = self.original_provider.clone();
        provider.name = self.display_name.clone();
        provider.description = self.description.clone();
        provider.compatible_with = compatible_types;
        provider.env = self.env_vars.clone();

        self.provider_manager
            .update_provider(&self.provider_id, provider.clone())?;
        self.original_provider = provider;
        Ok(())
    }

    fn get_field_display(&self, field: &EditField) -> String {
        match field {
            EditField::SectionTitle(title) => format!("-- {} --", title),
            EditField::DisplayName => format!("Display Name: {}", self.display_name),
            EditField::Description => format!("Description: {}", self.description),
            EditField::AiType(ai_type) => {
                let selected = self.is_ai_type_selected(ai_type);
                format!(
                    "[{}] {}",
                    if selected { "X" } else { " " },
                    Self::ai_type_label(ai_type)
                )
            }
            EditField::EnvGroup(ai_type) => {
                format!("{} Environment Variables", Self::ai_type_label(ai_type))
            }
            EditField::EnvVar { ai_type, key } => {
                let raw_value = self.env_vars.get(key).cloned().unwrap_or_default();
                let trimmed_empty = raw_value.trim().is_empty();
                let display_value = if trimmed_empty {
                    "(not set)".to_string()
                } else {
                    mask_if_sensitive(key, &raw_value)
                };

                let status = if let Some(def) = self.get_env_var_def(ai_type, key) {
                    if def.required && trimmed_empty {
                        " (required - missing)"
                    } else if def.required {
                        " (required)"
                    } else if trimmed_empty {
                        " (optional)"
                    } else {
                        " (optional)"
                    }
                } else {
                    ""
                };

                format!("  {} = {}{}", key, display_value, status)
            }
        }
    }

    fn get_env_var_def(&self, ai_type: &AiType, key: &str) -> Option<EnvVarDef> {
        if !self.is_ai_type_selected(ai_type) {
            return None;
        }

        let vars = get_env_vars_for_ai_type(ai_type.clone());
        vars.into_iter()
            .find(|var| var.key == key)
            .map(EnvVarDef::from)
    }
}

impl Screen for ProviderEditScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        match &self.mode {
            EditMode::Browse => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(10),
                        Constraint::Length(3),
                    ])
                    .split(area);

                let mut title_text = if self.display_name == self.provider_id {
                    format!("Edit Provider: {}", self.provider_id)
                } else {
                    format!(
                        "Edit Provider: {} ({})",
                        self.display_name, self.provider_id
                    )
                };
                if self.has_changes() {
                    title_text.push_str(" *modified");
                }

                let title = Paragraph::new(title_text)
                    .style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL));
                frame.render_widget(title, chunks[0]);

                // Field list
                let items: Vec<ListItem> = self
                    .all_fields
                    .iter()
                    .enumerate()
                    .map(|(idx, field)| {
                        let display = self.get_field_display(field);
                        let prefix = if idx == self.selected_field_idx {
                            "> "
                        } else {
                            "  "
                        };
                        let content = format!("{}{}", prefix, display);

                        let style = if idx == self.selected_field_idx {
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        };

                        ListItem::new(content).style(style)
                    })
                    .collect();

                let list = List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Configuration"),
                );
                frame.render_widget(list, chunks[1]);

                // Help
                let help = Paragraph::new(
                    "[Up/Down] Navigate  [Enter] Edit  [Space] Toggle Compatibility  [Ctrl+S] Save  [Esc] Cancel",
                )
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
                frame.render_widget(help, chunks[2]);
            }
            EditMode::EditingField(field) => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(10),
                        Constraint::Length(3),
                    ])
                    .split(area);

                let field_name = match field {
                    EditField::DisplayName => "Display Name",
                    EditField::Description => "Description",
                    EditField::EnvVar { key, .. } => key.as_str(),
                    _ => "Field",
                };

                let title = Paragraph::new(format!("Edit Field: {}", field_name))
                    .style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL));
                frame.render_widget(title, chunks[0]);

                self.input_widget.render(frame, chunks[1]);

                let help = Paragraph::new("[Enter] Save  [ESC] Cancel")
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL));
                frame.render_widget(help, chunks[2]);
            }
            EditMode::Dialog { widget, .. } => {
                widget.render(frame, area);
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        match &mut self.mode {
            EditMode::Browse => match key.code {
                KeyCode::Up => {
                    if self.selected_field_idx > 0 {
                        self.selected_field_idx -= 1;
                    }
                    Ok(ScreenAction::None)
                }
                KeyCode::Down => {
                    if self.selected_field_idx + 1 < self.all_fields.len() {
                        self.selected_field_idx += 1;
                    }
                    Ok(ScreenAction::None)
                }
                KeyCode::Enter => {
                    let field = self.all_fields[self.selected_field_idx].clone();
                    match &field {
                        EditField::DisplayName => {
                            self.input_widget = InputState::new("Display Name".to_string())
                                .with_value(self.display_name.clone());
                            self.input_widget.set_focused(true);
                            self.mode = EditMode::EditingField(field);
                        }
                        EditField::Description => {
                            self.input_widget = InputState::new("Description".to_string())
                                .with_value(self.description.clone());
                            self.input_widget.set_focused(true);
                            self.mode = EditMode::EditingField(field);
                        }
                        EditField::EnvVar { ai_type, key } => {
                            if let Some(var_def) = self.get_env_var_def(ai_type, key) {
                                let current_value =
                                    self.env_vars.get(key).cloned().unwrap_or_default();
                                self.input_widget = InputState::new(format!(
                                    "{} [{}]\n{}",
                                    var_def.key,
                                    Self::ai_type_label(ai_type),
                                    var_def.description
                                ))
                                .with_value(current_value)
                                .masked(var_def.sensitive);
                                self.input_widget.set_focused(true);
                                self.mode = EditMode::EditingField(field);
                            }
                        }
                        _ => {}
                    }
                    Ok(ScreenAction::None)
                }
                KeyCode::Char(' ') => {
                    if let EditField::AiType(ai_type) =
                        self.all_fields[self.selected_field_idx].clone()
                    {
                        let idx = Self::ai_type_index(&ai_type);
                        if let Some(selected) = self.ai_types_selected.get_mut(idx) {
                            *selected = !*selected;
                            if !*selected {
                                self.remove_env_vars_for(&ai_type);
                            }
                            self.rebuild_fields();
                        }
                    }
                    Ok(ScreenAction::None)
                }
                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    match self.save_changes() {
                        Ok(()) => {
                            self.rebuild_fields();
                            let dialog = DialogState::info(
                                "Success".to_string(),
                                format!("Provider '{}' saved successfully", self.display_name),
                            );
                            self.mode = EditMode::Dialog {
                                widget: dialog,
                                outcome: DialogOutcome::Info {
                                    exit_on_close: true,
                                },
                            };
                        }
                        Err(err) => {
                            let dialog = DialogState::error(
                                "Error".to_string(),
                                format!("Failed to save provider: {}", err),
                            );
                            self.mode = EditMode::Dialog {
                                widget: dialog,
                                outcome: DialogOutcome::Info {
                                    exit_on_close: false,
                                },
                            };
                        }
                    }
                    Ok(ScreenAction::None)
                }
                KeyCode::Esc => {
                    if self.has_changes() {
                        let dialog = DialogState::confirm(
                            "Discard Changes".to_string(),
                            "Discard all unsaved changes?".to_string(),
                        );
                        self.mode = EditMode::Dialog {
                            widget: dialog,
                            outcome: DialogOutcome::ConfirmDiscard,
                        };
                        Ok(ScreenAction::None)
                    } else {
                        Ok(ScreenAction::SwitchTo(ScreenType::Provider))
                    }
                }
                _ => Ok(ScreenAction::None),
            },
            EditMode::EditingField(field) => {
                if self.input_widget.handle_key(key) {
                    return Ok(ScreenAction::None);
                }

                match key.code {
                    KeyCode::Enter => {
                        let value = self.input_widget.value().to_string();
                        match field {
                            EditField::DisplayName => {
                                self.display_name = value;
                            }
                            EditField::Description => {
                                self.description = value;
                            }
                            EditField::EnvVar { key, .. } => {
                                let trimmed = value.trim();
                                if trimmed.is_empty() {
                                    self.env_vars.remove(key);
                                } else {
                                    self.env_vars.insert(key.clone(), value);
                                }
                            }
                            _ => {}
                        }
                        self.mode = EditMode::Browse;
                        self.rebuild_fields();
                        Ok(ScreenAction::None)
                    }
                    KeyCode::Esc => {
                        self.mode = EditMode::Browse;
                        Ok(ScreenAction::None)
                    }
                    _ => Ok(ScreenAction::None),
                }
            }
            EditMode::Dialog { widget, outcome } => {
                let result = widget.handle_key(key);
                let next_action = match outcome {
                    DialogOutcome::Info { exit_on_close } => match result {
                        DialogResult::Closed
                        | DialogResult::Confirmed
                        | DialogResult::Cancelled => {
                            if *exit_on_close {
                                Some(ScreenAction::SwitchTo(ScreenType::Provider))
                            } else {
                                None
                            }
                        }
                        DialogResult::None => return Ok(ScreenAction::None),
                    },
                    DialogOutcome::ConfirmDiscard => match result {
                        DialogResult::Confirmed => {
                            Some(ScreenAction::SwitchTo(ScreenType::Provider))
                        }
                        DialogResult::Cancelled | DialogResult::Closed => None,
                        DialogResult::None => return Ok(ScreenAction::None),
                    },
                };

                if next_action.is_none() {
                    self.mode = EditMode::Browse;
                }

                Ok(next_action.unwrap_or(ScreenAction::None))
            }
        }
    }

    fn update(&mut self) -> Result<()> {
        Ok(())
    }
}
