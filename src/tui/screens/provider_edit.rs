//! Provider edit screen

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::collections::HashMap;

use super::{Screen, ScreenAction};
use crate::provider::env_mapping::{EnvVarMapping, get_env_vars_for_ai_type};
use crate::provider::{AiType, Provider, ProviderManager};
use crate::tui::widgets::{DialogResult, DialogWidget, InputWidget};

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
    Description,
    AiType(usize),  // Index: 0=Codex, 1=Claude, 2=Gemini
    EnvVar(String), // Key of the environment variable
}

/// Provider edit mode
enum EditMode {
    Browse,
    EditingField(EditField),
    Dialog(DialogWidget),
}

/// Provider edit screen
pub struct ProviderEditScreen {
    provider_name: String,
    provider_manager: ProviderManager,

    // Editable data
    description: String,
    ai_types_selected: Vec<bool>, // [Codex, Claude, Gemini]
    env_vars: HashMap<String, String>,

    // UI state
    mode: EditMode,
    selected_field_idx: usize,
    all_fields: Vec<EditField>,
    input_widget: InputWidget,

    // Original data for comparison
    original_provider: Provider,
}

impl ProviderEditScreen {
    pub fn new(provider_name: String) -> Result<Self> {
        // Load provider data first
        let provider_manager_temp = ProviderManager::new()?;
        let (_, provider) = provider_manager_temp
            .list_providers()
            .into_iter()
            .find(|(name, _)| *name == &provider_name)
            .ok_or_else(|| anyhow::anyhow!("Provider '{}' not found", provider_name))?;

        let description = provider.description.clone();
        let env_vars = provider.env.clone();
        let original_provider = provider.clone();

        // Create a new provider manager instance for the struct
        let provider_manager = ProviderManager::new()?;

        // Initialize AI types selection
        let ai_types_selected = vec![
            provider.compatible_with.contains(&AiType::Codex),
            provider.compatible_with.contains(&AiType::Claude),
            provider.compatible_with.contains(&AiType::Gemini),
        ];

        // Build field list
        let mut all_fields = vec![EditField::Description];
        all_fields.push(EditField::AiType(0)); // Codex
        all_fields.push(EditField::AiType(1)); // Claude
        all_fields.push(EditField::AiType(2)); // Gemini

        // Add env vars for selected AI types
        for (idx, selected) in ai_types_selected.iter().enumerate() {
            if *selected {
                let ai_type = match idx {
                    0 => AiType::Codex,
                    1 => AiType::Claude,
                    2 => AiType::Gemini,
                    _ => continue,
                };
                let vars = get_env_vars_for_ai_type(ai_type);
                for var in vars {
                    all_fields.push(EditField::EnvVar(var.key.to_string()));
                }
            }
        }

        Ok(Self {
            provider_name,
            provider_manager,
            description,
            ai_types_selected,
            env_vars,
            mode: EditMode::Browse,
            selected_field_idx: 0,
            all_fields,
            input_widget: InputWidget::new("".to_string()),
            original_provider,
        })
    }

    fn rebuild_fields(&mut self) {
        self.all_fields.clear();
        self.all_fields.push(EditField::Description);
        self.all_fields.push(EditField::AiType(0));
        self.all_fields.push(EditField::AiType(1));
        self.all_fields.push(EditField::AiType(2));

        for (idx, selected) in self.ai_types_selected.iter().enumerate() {
            if *selected {
                let ai_type = match idx {
                    0 => AiType::Codex,
                    1 => AiType::Claude,
                    2 => AiType::Gemini,
                    _ => continue,
                };
                let vars = get_env_vars_for_ai_type(ai_type);
                for var in vars {
                    self.all_fields.push(EditField::EnvVar(var.key.to_string()));
                }
            }
        }

        // Clamp selected index
        if self.selected_field_idx >= self.all_fields.len() {
            self.selected_field_idx = self.all_fields.len().saturating_sub(1);
        }
    }

    fn save_changes(&mut self) -> Result<()> {
        let compatible_types: Vec<AiType> = self
            .ai_types_selected
            .iter()
            .enumerate()
            .filter_map(|(idx, selected)| {
                if *selected {
                    match idx {
                        0 => Some(AiType::Codex),
                        1 => Some(AiType::Claude),
                        2 => Some(AiType::Gemini),
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .collect();

        let provider = Provider {
            name: self.provider_name.clone(),
            description: self.description.clone(),
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
            env: self.env_vars.clone(),
        };

        self.provider_manager
            .add_provider(self.provider_name.clone(), provider)?;
        Ok(())
    }

    fn get_field_display(&self, field: &EditField) -> String {
        match field {
            EditField::Description => {
                format!("Description: {}", self.description)
            }
            EditField::AiType(idx) => {
                let (name, selected) = match idx {
                    0 => ("Codex", self.ai_types_selected[0]),
                    1 => ("Claude", self.ai_types_selected[1]),
                    2 => ("Gemini", self.ai_types_selected[2]),
                    _ => return String::new(),
                };
                format!("[{}] {}", if selected { "X" } else { " " }, name)
            }
            EditField::EnvVar(key) => {
                let value = self.env_vars.get(key).cloned().unwrap_or_default();
                let display_value = mask_if_sensitive(key, &value);
                format!("  {} = {}", key, display_value)
            }
        }
    }

    fn get_env_var_def(&self, key: &str) -> Option<EnvVarDef> {
        for (idx, selected) in self.ai_types_selected.iter().enumerate() {
            if *selected {
                let ai_type = match idx {
                    0 => AiType::Codex,
                    1 => AiType::Claude,
                    2 => AiType::Gemini,
                    _ => continue,
                };
                let vars = get_env_vars_for_ai_type(ai_type);
                if let Some(var) = vars.iter().find(|v| v.key == key) {
                    return Some(var.clone().into());
                }
            }
        }
        None
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

                // Title
                let title = Paragraph::new(format!("Edit Provider: {}", self.provider_name))
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
                    "[Up/Down] Navigate  [Enter] Edit  [Space] Toggle  [Ctrl+S] Save  [ESC] Back",
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
                    EditField::Description => "Description",
                    EditField::EnvVar(key) => key.as_str(),
                    _ => "",
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
            EditMode::Dialog(dialog) => {
                dialog.render(frame, area);
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        match &mut self.mode {
            EditMode::Browse => {
                match key.code {
                    KeyCode::Up => {
                        if self.selected_field_idx > 0 {
                            self.selected_field_idx -= 1;
                        }
                        Ok(ScreenAction::None)
                    }
                    KeyCode::Down => {
                        if self.selected_field_idx < self.all_fields.len() - 1 {
                            self.selected_field_idx += 1;
                        }
                        Ok(ScreenAction::None)
                    }
                    KeyCode::Enter => {
                        let field = self.all_fields[self.selected_field_idx].clone();

                        match &field {
                            EditField::Description => {
                                self.input_widget = InputWidget::new("Description".to_string())
                                    .with_value(self.description.clone());
                                self.input_widget.set_focused(true);
                                self.mode = EditMode::EditingField(field);
                            }
                            EditField::EnvVar(key) => {
                                if let Some(var_def) = self.get_env_var_def(key) {
                                    let current_value =
                                        self.env_vars.get(key).cloned().unwrap_or_default();
                                    self.input_widget = InputWidget::new(format!(
                                        "{}\n{}",
                                        var_def.key, var_def.description
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
                        // Toggle AI type
                        if let EditField::AiType(idx) = self.all_fields[self.selected_field_idx] {
                            self.ai_types_selected[idx] = !self.ai_types_selected[idx];
                            self.rebuild_fields();
                        }
                        Ok(ScreenAction::None)
                    }
                    KeyCode::Char('s')
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        // Save changes
                        if let Err(e) = self.save_changes() {
                            let dialog = DialogWidget::error(
                                "Error".to_string(),
                                format!("Failed to save provider: {}", e),
                            );
                            self.mode = EditMode::Dialog(dialog);
                        } else {
                            let dialog = DialogWidget::info(
                                "Success".to_string(),
                                format!("Provider '{}' saved successfully", self.provider_name),
                            );
                            self.mode = EditMode::Dialog(dialog);
                        }
                        Ok(ScreenAction::None)
                    }
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                        Ok(ScreenAction::Back)
                    }
                    _ => Ok(ScreenAction::None),
                }
            }
            EditMode::EditingField(field) => {
                if self.input_widget.handle_key(key) {
                    return Ok(ScreenAction::None);
                }

                match key.code {
                    KeyCode::Enter => {
                        let value = self.input_widget.value().to_string();

                        match field {
                            EditField::Description => {
                                self.description = value;
                            }
                            EditField::EnvVar(key) => {
                                self.env_vars.insert(key.clone(), value);
                            }
                            _ => {}
                        }

                        self.mode = EditMode::Browse;
                        Ok(ScreenAction::None)
                    }
                    KeyCode::Esc => {
                        self.mode = EditMode::Browse;
                        Ok(ScreenAction::None)
                    }
                    _ => Ok(ScreenAction::None),
                }
            }
            EditMode::Dialog(dialog) => {
                let result = dialog.handle_key(key);

                match result {
                    DialogResult::Closed | DialogResult::Confirmed | DialogResult::Cancelled => {
                        self.mode = EditMode::Browse;
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
