//! Provider management screen

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
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
        let sensitive = mapping.key.contains("KEY") || mapping.key.contains("SECRET");
        EnvVarDef {
            key: mapping.key.to_string(),
            description: mapping.description.to_string(),
            required: mapping.required,
            sensitive,
        }
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
        env_vars: HashMap<String, String>,
        current_ai_type_idx: usize,
        current_env_idx: usize,
    },
    DeleteConfirm(String),
    Dialog(DialogWidget),
}

/// Provider list screen
pub struct ProviderScreen {
    provider_manager: ProviderManager,
    list_widget: ListWidget<String>,
    mode: ProviderMode,
    input_widget: InputWidget,
    types_selected: Vec<bool>, // For multi-select AI types
}

impl ProviderScreen {
    pub fn new() -> Result<Self> {
        let provider_manager = ProviderManager::new()?;
        let provider_names: Vec<String> = provider_manager
            .list_providers()
            .into_iter()
            .map(|(name, _)| name.clone())
            .collect();

        let list_widget = ListWidget::new("Providers".to_string(), provider_names);

        Ok(Self {
            provider_manager,
            list_widget,
            mode: ProviderMode::List,
            input_widget: InputWidget::new("Input".to_string()),
            types_selected: vec![false, false, false], // codex, claude, gemini
        })
    }

    fn refresh_list(&mut self) -> Result<()> {
        let provider_names: Vec<String> = self
            .provider_manager
            .list_providers()
            .into_iter()
            .map(|(name, _)| name.clone())
            .collect();
        self.list_widget = ListWidget::new("Providers".to_string(), provider_names);
        Ok(())
    }

    fn get_selected_ai_types(&self) -> Vec<AiType> {
        let mut types = Vec::new();
        if self.types_selected[0] {
            types.push(AiType::Codex);
        }
        if self.types_selected[1] {
            types.push(AiType::Claude);
        }
        if self.types_selected[2] {
            types.push(AiType::Gemini);
        }
        types
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
}

impl Screen for ProviderScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        match &self.mode {
            ProviderMode::List => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(10),
                        Constraint::Length(3),
                    ])
                    .split(area);

                // Title
                let title = Paragraph::new("Provider Management")
                    .style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL));
                frame.render_widget(title, chunks[0]);

                // Provider list
                let default_provider = self
                    .provider_manager
                    .get_default_provider()
                    .map(|(name, _)| name.clone())
                    .unwrap_or_else(|_| "official".to_string());

                self.list_widget
                    .render(frame, chunks[1], |name, is_selected| {
                        let marker = if name == &default_provider {
                            " (default)"
                        } else {
                            ""
                        };
                        let prefix = if is_selected { "> " } else { "  " };
                        format!("{}{}{}", prefix, name, marker)
                    });

                // Help
                let help = Paragraph::new(
                    "[A] Add  [E] Edit  [D] Delete  [Enter] Set Default  [ESC] Back",
                )
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
                frame.render_widget(help, chunks[2]);
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
                compatible_types,
                env_vars,
                current_ai_type_idx,
                current_env_idx,
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

                // Show current variable being input
                let all_vars = Self::get_all_env_vars_for_types(compatible_types);
                let total_vars = all_vars.len();
                let current_idx = current_ai_type_idx * 3 + current_env_idx;

                if current_idx < total_vars {
                    let (ai_type, var) = &all_vars[current_idx];
                    let progress = format!(
                        "Variable {} of {} for {}",
                        current_idx + 1,
                        total_vars,
                        ai_type
                    );
                    let req_marker = if var.required {
                        " (REQUIRED)"
                    } else {
                        " (optional)"
                    };

                    let mut input =
                        InputWidget::new(format!("{}{}\n{}", var.key, req_marker, var.description))
                            .masked(var.sensitive);

                    if let Some(existing) = env_vars.get(&var.key) {
                        input = input.with_value(existing.clone());
                    }
                    input.set_focused(true);

                    let info_text = format!("{}\n\n", progress);
                    let info = Paragraph::new(info_text);

                    let inner_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Length(3), Constraint::Min(5)])
                        .split(chunks[1]);

                    frame.render_widget(info, inner_chunks[0]);
                    input.render(frame, inner_chunks[1]);
                }

                let help = Paragraph::new("[Enter] Next  [ESC] Cancel")
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL));
                frame.render_widget(help, chunks[2]);
            }
            ProviderMode::DeleteConfirm(name) => {
                let dialog = DialogWidget::confirm(
                    "Confirm Delete".to_string(),
                    format!("Are you sure you want to delete provider '{}'?", name),
                );
                dialog.render(frame, area);
            }
            ProviderMode::Dialog(dialog) => {
                dialog.render(frame, area);
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        // Extract selected_types before borrowing self.mode mutably
        let selected_types = if matches!(self.mode, ProviderMode::AddSelectTypes { .. })
            && matches!(key.code, KeyCode::Enter)
        {
            Some(self.get_selected_ai_types())
        } else {
            None
        };

        match &mut self.mode {
            ProviderMode::List => {
                // Let list widget handle navigation
                if self.list_widget.handle_key(key) {
                    return Ok(ScreenAction::None);
                }

                match key.code {
                    KeyCode::Char('a') | KeyCode::Char('A') => {
                        // Launch new provider add wizard (v2.0)
                        Ok(ScreenAction::SwitchTo(ScreenType::ProviderAddWizard))
                    }
                    KeyCode::Char('e') | KeyCode::Char('E') => {
                        if let Some(provider_name) = self.list_widget.selected() {
                            Ok(ScreenAction::SwitchTo(ScreenType::ProviderEdit(
                                provider_name.clone(),
                            )))
                        } else {
                            Ok(ScreenAction::None)
                        }
                    }
                    KeyCode::Char('d') | KeyCode::Char('D') => {
                        if let Some(provider_name) = self.list_widget.selected() {
                            self.mode = ProviderMode::DeleteConfirm(provider_name.clone());
                        }
                        Ok(ScreenAction::None)
                    }
                    KeyCode::Enter => {
                        if let Some(provider_name) = self.list_widget.selected() {
                            if let Err(e) = self.provider_manager.set_default(&provider_name) {
                                let dialog = DialogWidget::error(
                                    "Error".to_string(),
                                    format!("Failed to set default provider: {}", e),
                                );
                                self.mode = ProviderMode::Dialog(dialog);
                            } else {
                                self.refresh_list()?;
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
                        let name = self.input_widget.value().to_string();
                        if name.is_empty() {
                            let dialog = DialogWidget::warning(
                                "Warning".to_string(),
                                "Provider name cannot be empty".to_string(),
                            );
                            self.mode = ProviderMode::Dialog(dialog);
                        } else {
                            // Move to description input
                            self.input_widget = InputWidget::new("Description".to_string());
                            self.input_widget.set_focused(true);
                            self.mode = ProviderMode::AddDescriptionInput { name };
                        }
                        Ok(ScreenAction::None)
                    }
                    KeyCode::Esc => {
                        self.mode = ProviderMode::List;
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
                        // Move to type selection
                        self.types_selected = vec![false, false, false];
                        self.mode = ProviderMode::AddSelectTypes {
                            name: name.clone(),
                            description,
                        };
                        Ok(ScreenAction::None)
                    }
                    KeyCode::Esc => {
                        self.mode = ProviderMode::List;
                        Ok(ScreenAction::None)
                    }
                    _ => Ok(ScreenAction::None),
                }
            }
            ProviderMode::AddSelectTypes { name, description } => {
                match key.code {
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
                        if let Some(selected_types) = selected_types {
                            if selected_types.is_empty() {
                                let dialog = DialogWidget::warning(
                                    "Warning".to_string(),
                                    "Please select at least one AI type".to_string(),
                                );
                                self.mode = ProviderMode::Dialog(dialog);
                            } else {
                                // Start env input flow with collected data
                                self.input_widget = InputWidget::new("".to_string());
                                self.input_widget.set_focused(true);
                                self.mode = ProviderMode::AddEnvInput {
                                    name: name.clone(),
                                    description: description.clone(),
                                    compatible_types: selected_types,
                                    env_vars: HashMap::new(),
                                    current_ai_type_idx: 0,
                                    current_env_idx: 0,
                                };
                            }
                        }
                        Ok(ScreenAction::None)
                    }
                    KeyCode::Esc => {
                        self.mode = ProviderMode::List;
                        Ok(ScreenAction::None)
                    }
                    _ => Ok(ScreenAction::None),
                }
            }
            ProviderMode::AddEnvInput {
                name,
                description,
                compatible_types,
                env_vars,
                current_ai_type_idx,
                current_env_idx,
            } => {
                if self.input_widget.handle_key(key) {
                    return Ok(ScreenAction::None);
                }

                match key.code {
                    KeyCode::Enter => {
                        let all_vars = Self::get_all_env_vars_for_types(compatible_types);
                        let total_vars = all_vars.len();
                        let current_idx = *current_ai_type_idx * 3 + *current_env_idx;

                        if current_idx < total_vars {
                            let (_, var) = &all_vars[current_idx];
                            let value = self.input_widget.value().to_string();

                            if !value.is_empty() || !var.required {
                                env_vars.insert(var.key.clone(), value);

                                // Move to next variable
                                if current_idx + 1 < total_vars {
                                    let next_ai_type_idx = (current_idx + 1) / 3;
                                    let next_env_idx = (current_idx + 1) % 3;
                                    *current_ai_type_idx = next_ai_type_idx;
                                    *current_env_idx = next_env_idx;
                                    self.input_widget = InputWidget::new("".to_string());
                                    self.input_widget.set_focused(true);
                                } else {
                                    // All done, save provider
                                    let provider = Provider {
                                        name: name.clone(),
                                        description: description.clone(),
                                        icon: None,
                                        official: false,
                                        protected: false,
                                        custom: true,
                                        support_modes: vec![],
                                        compatible_with: compatible_types.clone(),
                                        validation_endpoint: None,
                                        category: None,
                                        website: None,
                                        regions: vec![],
                                        env: env_vars.clone(),
                                    };

                                    let provider_name = name.clone();
                                    let add_result = self
                                        .provider_manager
                                        .add_provider(provider_name.clone(), provider);

                                    if let Err(e) = add_result {
                                        let dialog = DialogWidget::error(
                                            "Error".to_string(),
                                            format!("Failed to add provider: {}", e),
                                        );
                                        self.mode = ProviderMode::Dialog(dialog);
                                    } else {
                                        self.refresh_list()?;
                                        let dialog = DialogWidget::info(
                                            "Success".to_string(),
                                            format!(
                                                "Provider '{}' added successfully",
                                                provider_name
                                            ),
                                        );
                                        self.mode = ProviderMode::Dialog(dialog);
                                    }
                                }
                            }
                        }
                        Ok(ScreenAction::None)
                    }
                    KeyCode::Esc => {
                        self.mode = ProviderMode::List;
                        Ok(ScreenAction::None)
                    }
                    _ => Ok(ScreenAction::None),
                }
            }
            ProviderMode::DeleteConfirm(name) => {
                let mut dialog = DialogWidget::confirm("".to_string(), "".to_string());
                let result = dialog.handle_key(key);

                match result {
                    DialogResult::Confirmed => {
                        let name_to_delete = name.clone();
                        if let Err(e) = self.provider_manager.remove_provider(&name_to_delete) {
                            let dialog = DialogWidget::error(
                                "Error".to_string(),
                                format!("Failed to delete provider: {}", e),
                            );
                            self.mode = ProviderMode::Dialog(dialog);
                        } else {
                            self.refresh_list()?;
                            self.mode = ProviderMode::List;
                        }
                        Ok(ScreenAction::None)
                    }
                    DialogResult::Cancelled | DialogResult::Closed => {
                        self.mode = ProviderMode::List;
                        Ok(ScreenAction::None)
                    }
                    DialogResult::None => Ok(ScreenAction::None),
                }
            }
            ProviderMode::Dialog(dialog) => {
                let result = dialog.handle_key(key);

                match result {
                    DialogResult::Closed | DialogResult::Confirmed | DialogResult::Cancelled => {
                        self.mode = ProviderMode::List;
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
