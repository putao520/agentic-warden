//! Provider Add Wizard - Step-by-step provider configuration (v2.0)
//!
//! This module provides a comprehensive 5-step wizard for adding AI providers
//! with intelligent recommendations and seamless token validation.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::provider::{
    config::{AiType, ProvidersConfig, Region, SupportMode},
    manager::ProviderManager,
    recommendation_engine::{RecommendationEngine, RecommendationPreferences},
    token_validator::TokenValidator,
};
use crate::tui::screens::{Screen, ScreenAction, ScreenType};

/// Wizard step enumeration (Enhanced v2.0)
#[derive(Debug, Clone, PartialEq)]
pub enum WizardStep {
    /// Step 1: Select CLI type
    CliTypeSelection,
    /// Step 2: Select provider (with intelligent recommendations)
    ProviderSelection,
    /// Step 3: Configure provider (auto-filled based on network and region)
    ProviderConfiguration,
    /// Step 4: Token input and validation (with regional support)
    TokenValidation,
    /// Step 5: Summary and confirmation
    Summary,
}

impl WizardStep {
    /// Get step title
    pub fn title(&self) -> &'static str {
        match self {
            Self::CliTypeSelection => "Select CLI Type",
            Self::ProviderSelection => "Select Provider",
            Self::ProviderConfiguration => "Configure Provider",
            Self::TokenValidation => "Verify API Token",
            Self::Summary => "Confirm Configuration",
        }
    }

    /// Get step description (Enhanced v2.0)
    pub fn description(&self) -> &'static str {
        match self {
            Self::CliTypeSelection => "Select the AI CLI tool type you want to use",
            Self::ProviderSelection => {
                "Intelligent recommendations based on network environment and CLI type"
            }
            Self::ProviderConfiguration => {
                "System has automatically configured optimal parameters based on network environment"
            }
            Self::TokenValidation => {
                "Enter API Token and verify connection, supports regional configuration"
            }
            Self::Summary => "Confirm configuration details and save to provider database",
        }
    }

    /// Get step number (1-based)
    pub fn step_number(&self) -> usize {
        match self {
            Self::CliTypeSelection => 1,
            Self::ProviderSelection => 2,
            Self::ProviderConfiguration => 3,
            Self::TokenValidation => 4,
            Self::Summary => 5,
        }
    }
}

/// Provider Add Wizard Screen (Enhanced v2.0)
pub struct ProviderAddWizard {
    /// Current wizard step
    current_step: WizardStep,
    /// List state for selections
    list_state: ListState,
    /// Selected AI type
    selected_ai_type: Option<AiType>,
    /// Selected provider ID
    selected_provider_id: Option<String>,
    /// Selected support mode
    selected_support_mode: Option<SupportMode>,
    /// Selected region (for v2.0)
    selected_region: Option<Region>,
    /// Input text buffer
    input_buffer: String,
    /// Token validation status
    token_valid: Option<bool>,
    /// Token validation message
    token_validation_message: String,
    /// Is testing connection
    is_testing_connection: bool,
    /// Recommendation engine
    recommendation_engine: RecommendationEngine,
    /// Available providers list
    available_providers: Vec<String>,
    /// Current recommendations (v2.0 format)
    current_recommendations: Vec<crate::provider::recommendation_engine::Recommendation>,
    /// Configured provider name
    provider_name: String,
    /// Provider icon
    provider_icon: Option<String>,
    /// Show confirmation dialog
    show_confirmation: bool,
    /// Token validator
    token_validator: TokenValidator,
    /// Providers config (v2.0)
    providers_config: ProvidersConfig,
}

impl ProviderAddWizard {
    /// Create new provider add wizard (v2.0)
    pub fn new() -> Result<Self> {
        let mut wizard = Self {
            current_step: WizardStep::CliTypeSelection,
            list_state: ListState::default(),
            selected_ai_type: None,
            selected_provider_id: None,
            selected_support_mode: None,
            selected_region: None,
            input_buffer: String::new(),
            token_valid: None,
            token_validation_message: String::new(),
            is_testing_connection: false,
            recommendation_engine: RecommendationEngine::new(),
            available_providers: Vec::new(),
            current_recommendations: Vec::new(),
            provider_name: String::new(),
            provider_icon: None,
            show_confirmation: false,
            token_validator: TokenValidator::new(),
            providers_config: ProvidersConfig::default(),
        };

        // Initialize list state
        wizard.list_state.select(Some(0));

        Ok(wizard)
    }

    /// Create wizard with existing providers config
    pub fn with_config(providers_config: ProvidersConfig) -> Result<Self> {
        let mut wizard = Self::new()?;
        wizard.providers_config = providers_config;
        Ok(wizard)
    }

    /// Get CLI type options
    fn get_cli_type_options(&self) -> Vec<(&'static str, AiType)> {
        vec![
            ("Claude Code", AiType::Claude),
            ("Codex CLI", AiType::Codex),
            ("Gemini CLI", AiType::Gemini),
        ]
    }

    /// Handle CLI type selection (v2.0)
    fn handle_cli_type_selection(&mut self) -> Result<()> {
        if let Some(selected) = self.list_state.selected() {
            let options = self.get_cli_type_options();
            if let Some((_, ai_type)) = options.get(selected) {
                self.selected_ai_type = Some(ai_type.clone());
                self.load_provider_recommendations_v2()?;
                self.current_step = WizardStep::ProviderSelection;
                self.list_state.select(Some(0));
            }
        }
        Ok(())
    }

    /// Load provider recommendations based on selected AI type (v2.0)
    fn load_provider_recommendations_v2(&mut self) -> Result<()> {
        if let Some(ai_type) = &self.selected_ai_type {
            let preferences = RecommendationPreferences::default();

            // Use v2.0 recommendation engine
            let recommendations =
                futures::executor::block_on(self.recommendation_engine.get_recommendations_v2(
                    &self.providers_config,
                    ai_type,
                    &preferences,
                ))?;

            self.current_recommendations = recommendations;
            self.available_providers = self
                .current_recommendations
                .iter()
                .map(|r| r.provider_id.clone())
                .collect();
        }
        Ok(())
    }

    /// Handle provider selection (v2.0)
    fn handle_provider_selection(&mut self) -> Result<()> {
        if let Some(selected) = self.list_state.selected() {
            if let Some(provider_id) = self.available_providers.get(selected) {
                self.selected_provider_id = Some(provider_id.clone());

                // Auto-select best mode and region for this provider
                if let Some(recommendation) = self.current_recommendations.get(selected) {
                    self.selected_support_mode = Some(recommendation.support_mode.clone());
                    self.selected_region = Some(recommendation.recommended_region.clone());
                    self.provider_name = recommendation.provider.name.clone();
                    self.provider_icon = recommendation.provider.icon.clone();
                }

                self.current_step = WizardStep::ProviderConfiguration;
            }
        }
        Ok(())
    }

    /// Move to next step (v2.0)
    fn next_step(&mut self) -> Result<()> {
        match &self.current_step {
            WizardStep::CliTypeSelection => {
                self.handle_cli_type_selection()?;
            }
            WizardStep::ProviderSelection => {
                self.handle_provider_selection()?;
            }
            WizardStep::ProviderConfiguration => {
                self.current_step = WizardStep::TokenValidation;
                self.input_buffer.clear();
                self.token_valid = None;
                self.token_validation_message.clear();
            }
            WizardStep::TokenValidation => {
                if self.token_valid.unwrap_or(false) {
                    self.current_step = WizardStep::Summary;
                }
            }
            WizardStep::Summary => {
                self.show_confirmation = true;
            }
        }
        Ok(())
    }

    /// Move to previous step
    fn prev_step(&mut self) -> Result<()> {
        if self.show_confirmation {
            self.show_confirmation = false;
            return Ok(());
        }

        match &self.current_step {
            WizardStep::CliTypeSelection => {
                // Cannot go back from first step
            }
            WizardStep::ProviderSelection => {
                self.current_step = WizardStep::CliTypeSelection;
                self.selected_ai_type = None;
            }
            WizardStep::ProviderConfiguration => {
                self.current_step = WizardStep::ProviderSelection;
                self.selected_provider_id = None;
                self.selected_support_mode = None;
            }
            WizardStep::TokenValidation => {
                self.current_step = WizardStep::ProviderConfiguration;
                self.token_valid = None;
                self.token_validation_message.clear();
            }
            WizardStep::Summary => {
                self.current_step = WizardStep::TokenValidation;
            }
        }
        self.list_state.select(Some(0));
        Ok(())
    }

    /// Test token validation (v2.0 with async support)
    fn test_token(&mut self) -> Result<()> {
        if self.input_buffer.trim().is_empty() {
            self.token_validation_message = "Please enter API Token".to_string();
            self.token_valid = Some(false);
            return Ok(());
        }

        self.is_testing_connection = true;

        // Validate token using v2.0 token validator
        if let (Some(provider_id), Some(_region), Some(support_mode)) = (
            &self.selected_provider_id,
            &self.selected_region,
            &self.selected_support_mode,
        ) {
            let _provider = self.providers_config.get_provider(provider_id).unwrap();
            let validation_result =
                futures::executor::block_on(self.token_validator.validate_with_network(
                    &self.providers_config,
                    &self.selected_provider_id.as_ref().unwrap(),
                    &support_mode.mode_type,
                    &self.input_buffer,
                ));

            match validation_result {
                Ok(result) if result.is_valid => {
                    self.token_validation_message =
                        "Token validation successful ✓ Connection normal".to_string();
                    self.token_valid = Some(true);
                }
                Ok(result) => {
                    self.token_validation_message =
                        format!("Token validation failed: {}", result.message);
                    self.token_valid = Some(false);
                }
                Err(e) => {
                    self.token_validation_message = format!("Error during validation: {}", e);
                    self.token_valid = Some(false);
                }
            }
        } else {
            self.token_validation_message =
                "Configuration incomplete, please start over".to_string();
            self.token_valid = Some(false);
        }

        self.is_testing_connection = false;
        Ok(())
    }

    /// Confirm and save provider configuration (v2.0)
    fn confirm_and_save(&mut self) -> Result<ScreenAction> {
        if let (Some(provider_id), Some(region), token) = (
            self.selected_provider_id.clone(),
            self.selected_region.clone(),
            &self.input_buffer,
        ) {
            // Save token to configuration
            let mut manager = ProviderManager::new()?;

            if let Err(e) = manager.set_token(&provider_id, region.clone(), token.clone()) {
                return Ok(ScreenAction::SwitchTo(ScreenType::ProviderEdit(format!(
                    "保存失败: {}",
                    e
                ))));
            }

            // Set as default provider if it's the first one
            if manager.list_providers_v2().len() == 1 {
                let _ = manager.set_default_provider(&provider_id);
            }

            Ok(ScreenAction::SwitchTo(ScreenType::Provider))
        } else {
            Ok(ScreenAction::SwitchTo(ScreenType::ProviderEdit(
                "Configuration incomplete".to_string(),
            )))
        }
    }

    /// Render wizard header with progress
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let header_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(3)])
            .split(area);

        // Title
        let title = Paragraph::new("Add AI Provider Wizard")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, header_area[0]);

        // Progress indicator
        let _progress = (self.current_step.step_number() as f32) / 5.0;
        let progress_gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Blue))
            .label(format!("Step {} / 5", self.current_step.step_number()));
        frame.render_widget(progress_gauge, header_area[1]);
    }

    /// Render step content
    fn render_step_content(&mut self, frame: &mut Frame, area: Rect) {
        let content_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Step title and description
                Constraint::Min(0),    // Main content
            ])
            .split(area);

        // Step info
        let step_info = Paragraph::new(vec![
            Line::from(Span::styled(
                format!(
                    "Step {}: {}",
                    self.current_step.step_number(),
                    self.current_step.title()
                ),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                self.current_step.description(),
                Style::default().fg(Color::Gray),
            )),
        ])
        .block(Block::default().borders(Borders::ALL))
        .wrap(Wrap { trim: true });
        frame.render_widget(step_info, content_area[0]);

        // Step-specific content
        match &self.current_step {
            WizardStep::CliTypeSelection => {
                self.render_cli_type_selection(frame, content_area[1]);
            }
            WizardStep::ProviderSelection => {
                self.render_provider_selection(frame, content_area[1]);
            }
            WizardStep::ProviderConfiguration => {
                self.render_provider_configuration(frame, content_area[1]);
            }
            WizardStep::TokenValidation => {
                self.render_token_validation(frame, content_area[1]);
            }
            WizardStep::Summary => {
                self.render_summary(frame, content_area[1]);
            }
        }
    }

    /// Render CLI type selection
    fn render_cli_type_selection(&mut self, frame: &mut Frame, area: Rect) {
        let options = self.get_cli_type_options();
        let items: Vec<ListItem> = options
            .iter()
            .enumerate()
            .map(|(i, (name, _))| {
                let style = if Some(i) == self.list_state.selected() {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(Span::styled(*name, style)))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Select CLI Type")
                    .borders(Borders::ALL),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        frame.render_stateful_widget(list, area, &mut self.list_state);
    }

    /// Render provider selection with recommendations (v2.0)
    fn render_provider_selection(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .current_recommendations
            .iter()
            .enumerate()
            .map(|(i, rec)| {
                let style = if Some(i) == self.list_state.selected() {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let icon = rec.provider.icon.as_deref().unwrap_or("🔧");

                let content = vec![
                    Line::from(vec![
                        Span::styled(icon, Style::default().fg(Color::Magenta)),
                        Span::raw(" "),
                        Span::styled(
                            format!("{} - {}", rec.provider.name, rec.support_mode.name),
                            style,
                        ),
                    ]),
                    Line::from(Span::styled(
                        format!("Recommendation reason: {}", rec.reason),
                        Style::default().fg(Color::Green),
                    )),
                    Line::from(Span::styled(
                        format!("Recommended region: {}", rec.recommended_region),
                        Style::default().fg(Color::Cyan),
                    )),
                    Line::from(Span::styled(
                        format!(
                            "Rating: {} | Token Status: {}",
                            rec.score,
                            if rec.has_token {
                                "Configured"
                            } else {
                                "Not configured"
                            }
                        ),
                        Style::default().fg(Color::Blue),
                    )),
                    // Show warnings if any
                    if !rec.warnings.is_empty() {
                        Line::from(Span::styled(
                            format!("⚠ {}", rec.warnings.join("; ")),
                            Style::default().fg(Color::Yellow),
                        ))
                    } else {
                        Line::from("")
                    },
                ];

                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Smart Recommended Providers (v2.0)")
                    .borders(Borders::ALL),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        frame.render_stateful_widget(list, area, &mut self.list_state);
    }

    /// Render provider configuration (v2.0)
    fn render_provider_configuration(&mut self, frame: &mut Frame, area: Rect) {
        let icon = self.provider_icon.as_deref().unwrap_or("🔧");

        let content = vec![
            Line::from("✨ Configuration information auto-filled:"),
            Line::from(""),
            Line::from(vec![
                Span::styled(icon, Style::default().fg(Color::Magenta)),
                Span::raw(" "),
                Span::styled(
                    format!("Provider: {}", self.provider_name),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(
                format!(
                    "Mode: {}",
                    self.selected_support_mode
                        .as_ref()
                        .map(|m| &m.name)
                        .unwrap_or(&"Not selected".to_string())
                ),
                Style::default().fg(Color::Green),
            )),
            Line::from(Span::styled(
                format!(
                    "Region: {}",
                    self.selected_region
                        .as_ref()
                        .map(|r| r.to_string())
                        .unwrap_or_else(|| "Not detected".to_string())
                ),
                Style::default().fg(Color::Yellow),
            )),
            Line::from(Span::styled(
                "✓ Intelligently optimized based on network environment",
                Style::default().fg(Color::Green),
            )),
            Line::from(""),
            Line::from("Press Enter to continue token configuration, Esc to return"),
        ];

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .title("Smart Configuration (v2.0)")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    /// Render token validation
    fn render_token_validation(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6),
                Constraint::Length(4),
                Constraint::Min(0),
            ])
            .split(area);

        // Input area
        let input_text = if self.input_buffer.is_empty() {
            "Please enter API Token..."
        } else {
            &self.input_buffer
        };

        let input_paragraph = Paragraph::new(input_text)
            .block(Block::default().title("API Token").borders(Borders::ALL))
            .style(Style::default().fg(Color::Yellow));

        frame.render_widget(input_paragraph, chunks[0]);

        // Validation status
        if !self.token_validation_message.is_empty() {
            let color = if self.token_valid.unwrap_or(false) {
                Color::Green
            } else {
                Color::Red
            };

            let status_paragraph = Paragraph::new(self.token_validation_message.as_str())
                .style(Style::default().fg(color))
                .block(
                    Block::default()
                        .title("Validation Status")
                        .borders(Borders::ALL),
                );
            frame.render_widget(status_paragraph, chunks[1]);
        }

        // Instructions
        let instructions = vec![
            Line::from("Instructions:"),
            Line::from("• Enter API Token"),
            Line::from("• Tab - Test connection"),
            Line::from("• Enter - Continue"),
            Line::from("• Esc - Return"),
        ];

        let instructions_paragraph = Paragraph::new(instructions)
            .block(Block::default().title("Instructions").borders(Borders::ALL))
            .wrap(Wrap { trim: true });

        frame.render_widget(instructions_paragraph, chunks[2]);
    }

    /// Render summary (v2.0)
    fn render_summary(&mut self, frame: &mut Frame, area: Rect) {
        let icon = self.provider_icon.as_deref().unwrap_or("🔧");

        let content = vec![
            Line::from("📋 Configuration Summary Confirmation:"),
            Line::from(""),
            Line::from(vec![
                Span::styled("CLI Type: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!(
                        "{:?}",
                        self.selected_ai_type.as_ref().unwrap_or(&AiType::Claude)
                    ),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled(icon, Style::default().fg(Color::Magenta)),
                Span::raw(" "),
                Span::styled("Provider: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    &self.provider_name,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Mode: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    self.selected_support_mode
                        .as_ref()
                        .map(|m| m.name.as_str())
                        .unwrap_or("Not selected"),
                    Style::default().fg(Color::Green),
                ),
            ]),
            Line::from(vec![
                Span::styled("Region: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    self.selected_region
                        .as_ref()
                        .map(|r| r.to_string())
                        .unwrap_or_else(|| "Not detected".to_string()),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
            Line::from(vec![
                Span::styled("Token: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    if self.token_valid.unwrap_or(false) {
                        "✓ Validation passed"
                    } else {
                        "✗ Not validated or validation failed"
                    },
                    if self.token_valid.unwrap_or(false) {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::Red)
                    },
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Configuration will be saved to ~/.agentic-warden/providers.json",
                Style::default().fg(Color::Gray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Enter - Confirm Save  Esc - Return to Edit",
                Style::default().fg(Color::Yellow),
            )),
        ];

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .title("Final Confirmation (v2.0)")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    /// Render confirmation dialog
    fn render_confirmation_dialog(&self, frame: &mut Frame) {
        let size = frame.size();
        let dialog_area = Rect {
            x: size.width / 4,
            y: size.height / 4,
            width: size.width / 2,
            height: 10,
        };

        let dialog = Paragraph::new(vec![
            Line::from("Confirm save configuration?"),
            Line::from(""),
            Line::from("Provider configuration is ready"),
            Line::from(""),
            Line::from("Enter - Confirm  Esc - Cancel"),
        ])
        .block(Block::default().title("Confirm").borders(Borders::ALL))
        .style(Style::default().bg(Color::DarkGray))
        .alignment(Alignment::Center);

        frame.render_widget(Clear, dialog_area);
        frame.render_widget(dialog, dialog_area);
    }
}

impl Screen for ProviderAddWizard {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5), // Header
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Instructions
            ])
            .split(area);

        self.render_header(frame, chunks[0]);
        self.render_step_content(frame, chunks[1]);

        // Instructions
        let instructions = match &self.current_step {
            WizardStep::CliTypeSelection | WizardStep::ProviderSelection => {
                "↑↓ Select  Enter Confirm  Esc Exit"
            }
            WizardStep::ProviderConfiguration => "Enter Continue  Esc Return",
            WizardStep::TokenValidation => "Enter Token  Tab Test  Enter Continue  Esc Return",
            WizardStep::Summary => "Enter Save  Esc Return",
        };

        let instructions_paragraph = Paragraph::new(instructions)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        frame.render_widget(instructions_paragraph, chunks[2]);

        // Render confirmation dialog if needed
        if self.show_confirmation {
            self.render_confirmation_dialog(frame);
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        // Handle confirmation dialog first
        if self.show_confirmation {
            match key.code {
                KeyCode::Enter => return self.confirm_and_save(),
                KeyCode::Esc => {
                    self.show_confirmation = false;
                    return Ok(ScreenAction::None);
                }
                _ => return Ok(ScreenAction::None),
            }
        }

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
                    let max_items = match &self.current_step {
                        WizardStep::CliTypeSelection => 3,
                        WizardStep::ProviderSelection => self.available_providers.len(),
                        _ => 0,
                    };
                    if selected < max_items.saturating_sub(1) {
                        self.list_state.select(Some(selected + 1));
                    }
                }
            }
            KeyCode::Enter => {
                self.next_step()?;
            }
            KeyCode::Esc => {
                self.prev_step()?;
            }
            KeyCode::Tab => {
                if matches!(self.current_step, WizardStep::TokenValidation) {
                    self.test_token()?;
                }
            }
            KeyCode::Char(c) => {
                if matches!(self.current_step, WizardStep::TokenValidation) {
                    self.input_buffer.push(c);
                }
            }
            KeyCode::Backspace => {
                if matches!(self.current_step, WizardStep::TokenValidation) {
                    self.input_buffer.pop();
                }
            }
            _ => {}
        }

        Ok(ScreenAction::None)
    }

    fn update(&mut self) -> Result<()> {
        // Update async operations here if needed
        Ok(())
    }
}
