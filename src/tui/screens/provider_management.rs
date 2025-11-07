    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

/// Provider显示数据结构
#[derive(Debug, Clone)]
pub struct ProviderDisplayItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub icon: String,
    pub ai_types: Vec<String>,
    pub region: Option<String>,
    pub is_default: bool,
    pub is_official: bool,
    pub has_token: bool,
}

/// Provider管理模式
#[derive(Debug, Clone, PartialEq)]
pub enum ProviderMode {
    View,
    Edit,
    Add,
}

/// Provider管理屏幕
pub struct ProviderManagementScreen {
    pub providers: Vec<ProviderDisplayItem>,
    pub list_state: ListState,
    pub mode: ProviderMode,
    pub selected_provider: usize,
    pub message: Option<String>,
    pub message_type: Option<String>,
}

impl ProviderManagementScreen {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            providers: vec![],
            list_state,
            mode: ProviderMode::View,
            selected_provider: 0,
            message: None,
            message_type: None,
        }
    }

    /// 刷新Provider列表
    ///
    /// 从 ProviderManager 加载最新的 provider 配置
    pub fn refresh_providers(&mut self) {
        use crate::provider::ProviderManager;

        match ProviderManager::new() {
            Ok(manager) => {
                let default_provider = manager.default_provider_name().to_string();

                // 转换 Provider 数据到显示格式
                self.providers = manager
                    .list_providers()
                    .into_iter()
                    .filter_map(|(id, provider)| {
                        // 获取第一个 region，如果没有则跳过该 provider
                        let first_region = provider.regions.keys().next()?;

                        Some(ProviderDisplayItem {
                            id: id.clone(),
                            name: provider.name.clone(),
                            description: provider.description.clone().unwrap_or_default(),
                            status: if manager.get_token(id, first_region).is_some() {
                                "Active".to_string()
                            } else {
                                "Inactive".to_string()
                            },
                            icon: provider.icon.clone().unwrap_or_else(|| "🔧".to_string()),
                            ai_types: provider.modes.keys().cloned().collect(),
                            region: Some(first_region.to_string()),
                            is_default: id == &default_provider,
                            is_official: provider.official.unwrap_or(false),
                            has_token: manager.has_token(id, first_region),
                        })
                    })
                    .collect();

                self.set_message(
                    format!("Loaded {} providers", self.providers.len()),
                    "success".to_string()
                );
            }
            Err(e) => {
                self.set_message(
                    format!("Failed to load providers: {}", e),
                    "error".to_string()
                );
            }
        }
    }

    /// 设置临时消息
    pub fn set_message(&mut self, message: String, msg_type: String) {
        self.message = Some(message);
        self.message_type = Some(msg_type);
    }

    /// 清除消息
    pub fn clear_message(&mut self) {
        self.message = None;
        self.message_type = None;
    }
}

impl Screen for ProviderManagementScreen {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // 标题
                Constraint::Min(0),     // 主内容区
                Constraint::Length(3),  // 帮助文本
            ])
            .split(area);

        // 标题
        let title = Paragraph::new("🔧 Provider Management")
            .style(Style::default().fg(Color::Cyan))
            .block(Block::default().borders(Borders::ALL).title("Agentic Warden"));
        frame.render_widget(title, chunks[0]);

        // 主内容区
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Provider列表
                Constraint::Percentage(50), // Provider详情
            ])
            .split(chunks[1]);

        // Provider列表
        let provider_items: Vec<ListItem> = self.providers.iter().map(|provider| {
            let default_indicator = if provider.is_default { " [DEFAULT]" } else { "" };
            let status_color = if provider.has_token { Color::Green } else { Color::Red };
            let status_text = if provider.has_token { "✅" } else { "❌" };

            let content = vec![
                Line::from(vec![
                    Span::styled(&provider.icon, Style::default()),
                    Span::raw(" "),
                    Span::styled(&provider.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Span::styled(default_indicator, Style::default().fg(Color::Yellow)),
                ]),
                Line::from(vec![
                    Span::styled(&provider.description, Style::default().fg(Color::Gray)),
                ]),
                Line::from(vec![
                    Span::styled("Status: ", Style::default().fg(Color::Gray)),
                    Span::styled(status_text, Style::default().fg(status_color)),
                    Span::styled(&provider.status, Style::default().fg(status_color)),
                ]),
            ];

            ListItem::new(content)
        }).collect();

        let provider_list = List::new(provider_items)
            .block(Block::default().borders(Borders::ALL).title("Providers"))
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(provider_list, main_chunks[0], &mut self.list_state);

        // 帮助文本
        let help_text = match self.mode {
            ProviderMode::View => "↑↓-Navigate | E-Edit | A-Add | Del-Delete | Enter-Set Default | Q-Back",
            ProviderMode::Edit => "↑↓-Navigate | Enter-Save | Esc-Cancel | Q-Back",
            ProviderMode::Add => "Enter-Save | Esc-Cancel | Q-Back",
        };

        let help_paragraph = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL).title("Help"));
        frame.render_widget(help_paragraph, chunks[2]);
    }

    fn handle_input(&mut self, key: crossterm::event::KeyEvent) -> ScreenAction {
        use crossterm::event::KeyCode;

        self.clear_message();

        match key.code {
            // 导航
            KeyCode::Up => {
                if let Some(selected) = self.list_state.selected() {
                    if selected > 0 {
                        self.list_state.select(Some(selected - 1));
                    }
                }
                ScreenAction::None
            }
            KeyCode::Down => {
                if let Some(selected) = self.list_state.selected() {
                    if selected < self.providers.len() - 1 {
                        self.list_state.select(Some(selected + 1));
                    }
                }
                ScreenAction::None
            }

            // 返回Dashboard
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                ScreenAction::NavigateTo(ScreenType::Dashboard)
            }

            // Provider操作
            KeyCode::Char('e') | KeyCode::Char('E') => {
                self.mode = ProviderMode::Edit;
                self.set_message("Edit provider feature coming soon".to_string(), "info".to_string());
                ScreenAction::None
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.set_message("Delete provider feature coming soon".to_string(), "info".to_string());
                ScreenAction::None
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.mode = ProviderMode::Add;
                self.set_message("Add provider feature coming soon".to_string(), "info".to_string());
                ScreenAction::None
            }

            _ => ScreenAction::None,
        }
    }

    fn get_title(&self) -> &str {
        "Provider Management"
    }
}