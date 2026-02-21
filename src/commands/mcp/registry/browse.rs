//! Interactive MCP server browser with TUI
//!
//! Provides a full TUI browsing experience with:
//! - Left panel: Server list with PageUp/PageDown navigation
//! - Right panel: Server details preview
//! - Search/filter functionality
//! - Tab to toggle focus between panels

use super::{aggregator::RegistryAggregator, install, types::EnvVarSpec, McpServerInfo};
use crate::tui::screens::InstalledMcpScreen;
use crate::tui::{Screen, ScreenAction};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use indicatif::{ProgressBar, ProgressStyle};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::collections::HashMap;

// Modern color palette using RGB
mod colors {
    use ratatui::style::Color;

    // Primary colors
    pub const PRIMARY: Color = Color::Rgb(99, 102, 241); // Indigo
    pub const SECONDARY: Color = Color::Rgb(168, 85, 247); // Purple

    // Accent colors
    pub const SUCCESS: Color = Color::Rgb(16, 185, 129); // Emerald
    pub const WARNING: Color = Color::Rgb(245, 158, 11); // Amber
    pub const INFO: Color = Color::Rgb(56, 189, 248); // Sky

    // Neutral colors
    pub const TEXT: Color = Color::Rgb(226, 232, 240); // Slate 200
    pub const TEXT_DIM: Color = Color::Rgb(148, 163, 184); // Slate 400
    pub const SURFACE: Color = Color::Rgb(30, 41, 59); // Slate 800
    pub const BORDER: Color = Color::Rgb(71, 85, 105); // Slate 600

    // Source colors
    pub const REGISTRY: Color = Color::Rgb(52, 211, 153); // Emerald 400
    pub const SMITHERY: Color = Color::Rgb(251, 146, 60); // Orange 400
}
use std::io;

/// Environment variable input state
pub(crate) struct EnvInputState {
    env_specs: Vec<EnvVarSpec>,
    current_index: usize,
    values: HashMap<String, String>,
    input_buffer: String,
    user_edited: bool,
}

impl EnvInputState {
    pub(crate) fn new(specs: Vec<EnvVarSpec>) -> Self {
        Self {
            env_specs: specs,
            current_index: 0,
            values: HashMap::new(),
            input_buffer: String::new(),
            user_edited: false,
        }
    }

    pub(crate) fn current_spec(&self) -> Option<&EnvVarSpec> {
        self.env_specs.get(self.current_index)
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.current_index >= self.env_specs.len()
    }

    pub(crate) fn next(&mut self) {
        if let Some(spec) = self.current_spec().cloned() {
            let trimmed = self.input_buffer.trim();
            // Only store non-empty values or required variables that need to be tracked
            if !trimmed.is_empty() {
                self.values
                    .insert(spec.name.clone(), trimmed.to_string());
            } else if spec.required {
                // For required variables, even if empty, we track them
                // (though this would normally be caught by validation)
                self.values.insert(spec.name.clone(), String::new());
            } else {
                self.values.remove(&spec.name);
            }
            // Optional empty variables are not stored
        }
        self.current_index += 1;
        self.sync_input_buffer();
    }

    pub(crate) fn get_values(&self) -> Vec<(String, String)> {
        self.values.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    pub(crate) fn has_remaining_optional(&self) -> bool {
        self.env_specs[self.current_index..]
            .iter()
            .any(|spec| !spec.required)
    }

    pub(crate) fn skip_all_optional(&mut self) {
        while self.current_index < self.env_specs.len() {
            let spec = self.env_specs[self.current_index].clone();
            if spec.required {
                break;
            }
            self.values.remove(&spec.name);
            self.current_index += 1;
        }
        self.sync_input_buffer();
    }

    pub(crate) fn preload_values(&mut self, values: HashMap<String, String>) {
        self.values = values;
        self.sync_input_buffer();
    }

    pub(crate) fn input_buffer(&self) -> &str {
        &self.input_buffer
    }

    pub(crate) fn user_has_edited(&self) -> bool {
        self.user_edited
    }

    pub(crate) fn push_char(&mut self, c: char) {
        self.input_buffer.push(c);
        self.user_edited = true;
    }

    pub(crate) fn pop_char(&mut self) {
        self.input_buffer.pop();
        self.user_edited = true;
    }

    pub(crate) fn current_index(&self) -> usize {
        self.current_index
    }

    pub(crate) fn total_specs(&self) -> usize {
        self.env_specs.len()
    }

    pub(crate) fn env_specs(&self) -> &[EnvVarSpec] {
        &self.env_specs
    }

    fn sync_input_buffer(&mut self) {
        self.input_buffer = self
            .current_spec()
            .and_then(|spec| self.values.get(&spec.name).cloned())
            .unwrap_or_default();
        self.user_edited = false;
    }
}

/// Browser state
struct BrowserState {
    servers: Vec<McpServerInfo>,
    filtered: Vec<usize>, // indices into servers
    list_state: ListState,
    search_query: String,
    search_mode: bool,
    show_help: bool,
    scroll_offset: u16,
    source_filter: Option<String>,
    env_input: Option<EnvInputState>,
    installed_screen: Option<InstalledMcpScreen>,
}

impl BrowserState {
    fn new(servers: Vec<McpServerInfo>, source_filter: Option<String>) -> Self {
        let filtered: Vec<usize> = (0..servers.len()).collect();
        let mut state = Self {
            servers,
            filtered,
            list_state: ListState::default(),
            search_query: String::new(),
            search_mode: false,
            show_help: false,
            scroll_offset: 0,
            source_filter,
            env_input: None,
            installed_screen: None,
        };
        if !state.filtered.is_empty() {
            state.list_state.select(Some(0));
        }
        state.apply_filter();
        state
    }

    fn apply_filter(&mut self) {
        let query = self.search_query.to_lowercase();
        self.filtered = self
            .servers
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                // Apply source filter if set
                if let Some(ref src) = self.source_filter {
                    if &s.source != src {
                        return false;
                    }
                }
                // Apply search query
                if query.is_empty() {
                    true
                } else {
                    s.qualified_name.to_lowercase().contains(&query)
                        || s.description
                            .as_ref()
                            .map(|d| d.to_lowercase().contains(&query))
                            .unwrap_or(false)
                }
            })
            .map(|(i, _)| i)
            .collect();

        // Reset selection if current selection is out of bounds
        if let Some(selected) = self.list_state.selected() {
            if selected >= self.filtered.len() {
                self.list_state
                    .select(if self.filtered.is_empty() { None } else { Some(0) });
            }
        } else if !self.filtered.is_empty() {
            self.list_state.select(Some(0));
        }
        self.scroll_offset = 0;
    }

    fn selected_server(&self) -> Option<&McpServerInfo> {
        self.list_state
            .selected()
            .and_then(|i| self.filtered.get(i))
            .map(|&idx| &self.servers[idx])
    }

    fn move_up(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if selected > 0 {
                self.list_state.select(Some(selected - 1));
                self.scroll_offset = 0;
            }
        }
    }

    fn move_down(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if selected < self.filtered.len().saturating_sub(1) {
                self.list_state.select(Some(selected + 1));
                self.scroll_offset = 0;
            }
        }
    }

    fn page_up(&mut self, page_size: usize) {
        if let Some(selected) = self.list_state.selected() {
            let new_pos = selected.saturating_sub(page_size);
            self.list_state.select(Some(new_pos));
            self.scroll_offset = 0;
        }
    }

    fn page_down(&mut self, page_size: usize) {
        if let Some(selected) = self.list_state.selected() {
            let new_pos = (selected + page_size).min(self.filtered.len().saturating_sub(1));
            self.list_state.select(Some(new_pos));
            self.scroll_offset = 0;
        }
    }

    fn home(&mut self) {
        if !self.filtered.is_empty() {
            self.list_state.select(Some(0));
            self.scroll_offset = 0;
        }
    }

    fn end(&mut self) {
        if !self.filtered.is_empty() {
            self.list_state.select(Some(self.filtered.len() - 1));
            self.scroll_offset = 0;
        }
    }

    fn scroll_detail_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    fn scroll_detail_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }
}

/// Execute the interactive browse command
pub async fn execute(source: Option<String>) -> Result<()> {
    let aggregator = RegistryAggregator::new();

    // Show loading spinner
    let spinner = ProgressBar::new_spinner()
        .with_style(
            ProgressStyle::default_spinner()
                .template("{spinner} {msg}")
                .unwrap(),
        )
        .with_message("Loading MCP servers from registries...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    // Fetch all servers
    let results = aggregator.search("", source.as_deref(), 500).await?;
    spinner.finish_and_clear();

    if results.is_empty() {
        println!("No MCP servers found.");
        return Ok(());
    }

    // Run TUI
    let selected = run_tui(results, source, &aggregator).await?;

    // Handle selection
    if let Some((server, env_vars)) = selected {
        println!("\nInstalling {}...", server.qualified_name);
        install::install_with_aggregator(
            &aggregator,
            &server.qualified_name,
            Some(server.source.clone()),
            env_vars,
            false,
        )
        .await?;
    }

    Ok(())
}

async fn run_tui(
    servers: Vec<McpServerInfo>,
    source: Option<String>,
    aggregator: &RegistryAggregator,
) -> Result<Option<(McpServerInfo, Vec<(String, String)>)>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = BrowserState::new(servers, source);
    let result = run_event_loop(&mut terminal, &mut state, aggregator).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut BrowserState,
    aggregator: &RegistryAggregator,
) -> Result<Option<(McpServerInfo, Vec<(String, String)>)>> {
    loop {
        terminal.draw(|f| draw_ui(f, state))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            if let Some(installed_screen) = state.installed_screen.as_mut() {
                match installed_screen.handle_key(key)? {
                    ScreenAction::Back => {
                        state.installed_screen = None;
                    }
                    ScreenAction::Quit => {
                        return Ok(None);
                    }
                    _ => {}
                }
                continue;
            }

            // Handle environment variable input mode
            if let Some(ref mut env_input) = state.env_input {
                match key.code {
                    KeyCode::Enter => {
                        env_input.next();
                        if env_input.is_complete() {
                            // Environment variable input complete
                            let env_vars = env_input.get_values();
                            if let Some(server) = state.selected_server() {
                                return Ok(Some((server.clone(), env_vars)));
                            }
                            state.env_input = None;
                        }
                    }
                    KeyCode::Backspace => {
                        env_input.pop_char();
                    }
                    KeyCode::Char(c) if (c == 'a' || c == 'A') && !env_input.user_has_edited() => {
                        let should_skip = env_input
                            .current_spec()
                            .map(|spec| !spec.required)
                            .unwrap_or(false);
                        if should_skip {
                            env_input.skip_all_optional();
                            if env_input.is_complete() {
                                let env_vars = env_input.get_values();
                                if let Some(server) = state.selected_server() {
                                    return Ok(Some((server.clone(), env_vars)));
                                }
                                state.env_input = None;
                            }
                        } else {
                            env_input.push_char(c);
                        }
                    }
                    KeyCode::Char(c) => {
                        env_input.push_char(c);
                    }
                    KeyCode::Esc => {
                        state.env_input = None;
                    }
                    _ => {}
                }
                continue;
            }

            if state.show_help {
                state.show_help = false;
                continue;
            }

            if state.search_mode {
                match key.code {
                    KeyCode::Enter | KeyCode::Esc => {
                        state.search_mode = false;
                    }
                    KeyCode::Backspace => {
                        state.search_query.pop();
                        state.apply_filter();
                    }
                    KeyCode::Char(c) => {
                        state.search_query.push(c);
                        state.apply_filter();
                    }
                    _ => {}
                }
                continue;
            }

            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    return Ok(None);
                }
                KeyCode::Char('?') => {
                    state.show_help = true;
                }
                KeyCode::Char('/') => {
                    state.search_mode = true;
                }
                KeyCode::Char('c') => {
                    state.search_query.clear();
                    state.apply_filter();
                }
                KeyCode::Char('i') | KeyCode::Char('I') => {
                    state.installed_screen = Some(InstalledMcpScreen::new()?);
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    state.move_up();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    state.move_down();
                }
                KeyCode::PageUp => {
                    state.page_up(10);
                }
                KeyCode::PageDown => {
                    state.page_down(10);
                }
                KeyCode::Home => {
                    state.home();
                }
                KeyCode::End => {
                    state.end();
                }
                KeyCode::Tab => {
                    state.scroll_detail_down();
                }
                KeyCode::BackTab => {
                    state.scroll_detail_up();
                }
                KeyCode::Enter => {
                    if let Some(server) = state.selected_server() {
                        // Fetch server details to get environment variable requirements
                        let detail = aggregator
                            .get_server_detail(&server.qualified_name, Some(server.source.as_str()))
                            .await;

                        match detail {
                            Ok(detail) if !detail.required_env.is_empty() => {
                                // Enter environment variable input mode
                                state.env_input = Some(EnvInputState::new(detail.required_env));
                            }
                            _ => {
                                // No environment variables required, proceed with installation
                                return Ok(Some((server.clone(), Vec::new())));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn draw_ui(f: &mut Frame, state: &mut BrowserState) {
    let size = f.size();

    if let Some(installed_screen) = state.installed_screen.as_mut() {
        installed_screen.render(f, size);
        return;
    }

    // Create layout: main area + status bar
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(1)])
        .split(size);

    // Split main area into left (list) and right (details) panels
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(main_chunks[0]);

    draw_list_panel(f, state, content_chunks[0]);
    draw_detail_panel(f, state, content_chunks[1]);
    draw_status_bar(f, state, main_chunks[1]);

    // Draw search bar at bottom if in search mode
    if state.search_mode {
        draw_search_bar(f, state, size);
    }

    // Draw help popup if needed
    if state.show_help {
        draw_help_popup(f, size);
    }

    // Draw environment variable input dialog if in env input mode
    if state.env_input.is_some() {
        draw_env_input_dialog(f, state, size);
    }
}

fn format_downloads(downloads: Option<u64>) -> String {
    match downloads {
        Some(n) if n >= 1_000_000 => format!("{:.1}M", n as f64 / 1_000_000.0),
        Some(n) if n >= 1_000 => format!("{:.1}k", n as f64 / 1_000.0),
        Some(n) => format!("{}", n),
        None => "—".to_string(),
    }
}

fn draw_list_panel(f: &mut Frame, state: &mut BrowserState, area: Rect) {
    let items: Vec<ListItem> = state
        .filtered
        .iter()
        .map(|&idx| {
            let server = &state.servers[idx];

            // Type icon
            let type_icon = match server.install.label() {
                "npm" => "📦",
                "uvx" => "🐍",
                "docker" => "🐳",
                _ => "📋",
            };

            // Source indicator
            let (src_icon, src_color) = match server.source.as_str() {
                "registry" => ("●", colors::REGISTRY),
                "smithery" => ("◆", colors::SMITHERY),
                _ => ("○", colors::TEXT_DIM),
            };

            // Downloads text
            let dl_text = format_downloads(server.downloads);

            // Single line: icon + name + downloads
            let line = Line::from(vec![
                Span::styled(format!("{}", type_icon), Style::default()),
                Span::styled(src_icon, Style::default().fg(src_color)),
                Span::raw(" "),
                Span::styled(
                    truncate_str(&server.qualified_name, 28),
                    Style::default().fg(colors::TEXT),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("⬇{}", dl_text),
                    Style::default().fg(colors::TEXT_DIM),
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    let title_spans = vec![
        Span::styled(" MCP Servers ", Style::default().fg(colors::TEXT).add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("({}/{})", state.filtered.len(), state.servers.len()),
            Style::default().fg(colors::TEXT_DIM),
        ),
        if !state.search_query.is_empty() {
            Span::styled(
                format!(" 🔍 {}", state.search_query),
                Style::default().fg(colors::WARNING),
            )
        } else {
            Span::raw("")
        },
        Span::raw(" "),
    ];

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_set(border::ROUNDED)
                .title(Line::from(title_spans))
                .border_style(Style::default().fg(colors::PRIMARY)),
        )
        .highlight_style(
            Style::default()
                .bg(colors::SURFACE)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");

    f.render_stateful_widget(list, area, &mut state.list_state);
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}

fn draw_detail_panel(f: &mut Frame, state: &BrowserState, area: Rect) {
    let content = if let Some(server) = state.selected_server() {
        let (src_color, src_label) = match server.source.as_str() {
            "registry" => (colors::REGISTRY, "Official Registry"),
            "smithery" => (colors::SMITHERY, "Smithery"),
            _ => (colors::TEXT_DIM, "Unknown"),
        };

        let (type_color, type_icon) = match server.install.label() {
            "npm" => (colors::INFO, "📦"),
            "uvx" => (colors::WARNING, "🐍"),
            "docker" => (colors::SECONDARY, "🐳"),
            _ => (colors::TEXT_DIM, "📋"),
        };

        let mut lines = vec![
            // Header section with name
            Line::from(vec![
                Span::styled(
                    &server.qualified_name,
                    Style::default()
                        .fg(colors::TEXT)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            // Metadata badges
            Line::from(vec![
                Span::styled(format!("{} ", type_icon), Style::default()),
                Span::styled(
                    format!("{:<6}", server.install.label()),
                    Style::default().fg(type_color),
                ),
                Span::styled("  ●  ", Style::default().fg(colors::BORDER)),
                Span::styled(src_label, Style::default().fg(src_color)),
                if let Some(author) = &server.author {
                    Span::styled(
                        format!("  ●  by {}", author),
                        Style::default().fg(colors::TEXT_DIM),
                    )
                } else {
                    Span::raw("")
                },
            ]),
            // Downloads
            Line::from(vec![
                Span::styled("⬇ ", Style::default().fg(colors::SUCCESS)),
                Span::styled(
                    format_downloads(server.downloads),
                    Style::default().fg(colors::TEXT_DIM),
                ),
                Span::styled(" downloads", Style::default().fg(colors::TEXT_DIM)),
            ]),
            Line::from(""),
            // Description header
            Line::from(Span::styled(
                "━━━ Description ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━",
                Style::default().fg(colors::BORDER),
            )),
            Line::from(""),
        ];

        // Add description
        let desc = server
            .description
            .as_deref()
            .unwrap_or("No description available.");
        for line in desc.lines() {
            lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(colors::TEXT),
            )));
        }

        // Install command section
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "━━━ Quick Install ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━",
            Style::default().fg(colors::BORDER),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  $ ", Style::default().fg(colors::TEXT_DIM)),
            Span::styled(
                format!("aiw mcp install {}", server.qualified_name),
                Style::default().fg(colors::SUCCESS),
            ),
        ]));
        lines.push(Line::from(""));

        // Keyboard hints
        lines.push(Line::from(Span::styled(
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━",
            Style::default().fg(colors::BORDER),
        )));
        lines.push(Line::from(vec![
            Span::styled("  Enter ", Style::default().fg(colors::SUCCESS).add_modifier(Modifier::BOLD)),
            Span::styled("Install", Style::default().fg(colors::TEXT_DIM)),
            Span::styled("    Tab ", Style::default().fg(colors::INFO).add_modifier(Modifier::BOLD)),
            Span::styled("Scroll ↓", Style::default().fg(colors::TEXT_DIM)),
            Span::styled("    S-Tab ", Style::default().fg(colors::INFO).add_modifier(Modifier::BOLD)),
            Span::styled("Scroll ↑", Style::default().fg(colors::TEXT_DIM)),
        ]));

        lines
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No server selected",
                Style::default().fg(colors::TEXT_DIM),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Use ↑/↓ to navigate the list",
                Style::default().fg(colors::TEXT_DIM),
            )),
        ]
    };

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_set(border::ROUNDED)
                .title(Line::from(vec![
                    Span::styled(" Details ", Style::default().fg(colors::TEXT).add_modifier(Modifier::BOLD)),
                ]))
                .border_style(Style::default().fg(colors::SECONDARY)),
        )
        .wrap(Wrap { trim: false })
        .scroll((state.scroll_offset, 0));

    f.render_widget(paragraph, area);
}

fn draw_status_bar(f: &mut Frame, state: &BrowserState, area: Rect) {
    let registry_count = state.servers.iter().filter(|s| s.source == "registry").count();
    let smithery_count = state.servers.iter().filter(|s| s.source == "smithery").count();

    let status_line = Line::from(vec![
        Span::styled(" MCP Browser ", Style::default().fg(colors::TEXT).add_modifier(Modifier::BOLD)),
        Span::styled("│", Style::default().fg(colors::BORDER)),
        Span::styled(
            format!(" {} servers ", state.servers.len()),
            Style::default().fg(colors::TEXT_DIM),
        ),
        Span::styled("│", Style::default().fg(colors::BORDER)),
        Span::styled(" ●", Style::default().fg(colors::REGISTRY)),
        Span::styled(format!("{} ", registry_count), Style::default().fg(colors::TEXT_DIM)),
        Span::styled("◆", Style::default().fg(colors::SMITHERY)),
        Span::styled(format!("{} ", smithery_count), Style::default().fg(colors::TEXT_DIM)),
        Span::styled("│", Style::default().fg(colors::BORDER)),
        Span::styled(" ↑↓", Style::default().fg(colors::INFO)),
        Span::styled(" Nav ", Style::default().fg(colors::TEXT_DIM)),
        Span::styled("/", Style::default().fg(colors::WARNING)),
        Span::styled(" Search ", Style::default().fg(colors::TEXT_DIM)),
        Span::styled("i", Style::default().fg(colors::WARNING)),
        Span::styled(" Installed ", Style::default().fg(colors::TEXT_DIM)),
        Span::styled("Enter", Style::default().fg(colors::SUCCESS)),
        Span::styled(" Install ", Style::default().fg(colors::TEXT_DIM)),
        Span::styled("?", Style::default().fg(colors::SECONDARY)),
        Span::styled(" Help ", Style::default().fg(colors::TEXT_DIM)),
    ]);

    let paragraph = Paragraph::new(status_line).style(Style::default().bg(colors::SURFACE));
    f.render_widget(paragraph, area);
}

fn draw_search_bar(f: &mut Frame, state: &BrowserState, size: Rect) {
    let area = Rect {
        x: 0,
        y: size.height - 2,
        width: size.width,
        height: 1,
    };

    let search_line = Line::from(vec![
        Span::styled(" 🔍 Search: ", Style::default().fg(colors::WARNING)),
        Span::styled(&state.search_query, Style::default().fg(colors::TEXT)),
        Span::styled("_", Style::default().fg(colors::TEXT).add_modifier(Modifier::SLOW_BLINK)),
        Span::styled(
            "  (Enter to confirm, Esc to cancel)",
            Style::default().fg(colors::TEXT_DIM),
        ),
    ]);

    let paragraph = Paragraph::new(search_line).style(Style::default().bg(colors::SURFACE));
    f.render_widget(paragraph, area);
}

fn draw_help_popup(f: &mut Frame, size: Rect) {
    let popup_width = 55;
    let popup_height = 19;
    let area = Rect {
        x: (size.width.saturating_sub(popup_width)) / 2,
        y: (size.height.saturating_sub(popup_height)) / 2,
        width: popup_width.min(size.width),
        height: popup_height.min(size.height),
    };

    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  ⌨️  Keyboard Shortcuts",
            Style::default()
                .fg(colors::TEXT)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("    ↑/k  ↓/j      ", Style::default().fg(colors::INFO)),
            Span::styled("Move up/down", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    PgUp  PgDn    ", Style::default().fg(colors::INFO)),
            Span::styled("Page up/down (10 items)", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    Home  End     ", Style::default().fg(colors::INFO)),
            Span::styled("Jump to start/end", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    Tab  S-Tab    ", Style::default().fg(colors::INFO)),
            Span::styled("Scroll details panel", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("    /             ", Style::default().fg(colors::WARNING)),
            Span::styled("Search/filter servers", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    c             ", Style::default().fg(colors::WARNING)),
            Span::styled("Clear current filter", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    i             ", Style::default().fg(colors::WARNING)),
            Span::styled("View installed MCPs", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("    Enter         ", Style::default().fg(colors::SUCCESS)),
            Span::styled("Install selected server", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("    q  Esc        ", Style::default().fg(colors::SECONDARY)),
            Span::styled("Quit browser", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "        Press any key to close",
            Style::default().fg(colors::TEXT_DIM),
        )),
    ];

    let paragraph = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .title(Line::from(vec![
                Span::styled(" Help ", Style::default().fg(colors::TEXT).add_modifier(Modifier::BOLD)),
            ]))
            .border_style(Style::default().fg(colors::SUCCESS))
            .style(Style::default().bg(Color::Rgb(15, 23, 42))), // Slate 900
    );

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

fn draw_env_input_dialog(f: &mut Frame, state: &mut BrowserState, area: Rect) {
    if let Some(ref env_input) = state.env_input {
        // Create dialog area (centered, 70% width, variable height)
        let dialog_width = (area.width as f32 * 0.7) as u16;
        let extra_hint = if env_input.has_remaining_optional() { 1 } else { 0 };
        let dialog_height = (env_input.total_specs() as u16 + 6 + extra_hint)
            .min(area.height.saturating_sub(2));
        let dialog_x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
        let dialog_y = area.y + (area.height.saturating_sub(dialog_height)) / 2;

        let dialog_area = Rect {
            x: dialog_x,
            y: dialog_y,
            width: dialog_width,
            height: dialog_height,
        };

        // Draw background
        f.render_widget(Clear, dialog_area);

        // Build dialog content
        let mut lines = vec![];

        if let Some(spec) = env_input.current_spec() {
            // Variable name
            let name_line = Line::from(vec![
                Span::styled(
                    format!("{}", spec.name),
                    Style::default()
                        .fg(colors::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(if spec.required {
                    " (required)"
                } else {
                    " (optional)"
                }),
            ]);
            lines.push(name_line);
            lines.push(Line::from(""));

            // Description
            if let Some(desc) = &spec.description {
                lines.push(Line::from(Span::styled(
                    desc.clone(),
                    Style::default().fg(colors::TEXT_DIM),
                )));
            }

            // Default value
            if let Some(default) = &spec.default {
                lines.push(Line::from(Span::styled(
                    format!("Default: {}", default),
                    Style::default().fg(colors::TEXT_DIM),
                )));
            }

            lines.push(Line::from(""));

            // Input field
            let input_line = Line::from(vec![
                Span::raw("Value: "),
                Span::styled(
                    format!("{}_", env_input.input_buffer()),
                    Style::default()
                        .fg(colors::SUCCESS)
                        .add_modifier(Modifier::BOLD),
                ),
            ]);
            lines.push(input_line);

            lines.push(Line::from(""));

            // Progress indicator
            let progress = format!(
                "{} / {}",
                env_input.current_index() + 1,
                env_input.total_specs()
            );
            lines.push(Line::from(Span::styled(
                progress,
                Style::default().fg(colors::INFO),
            )));

            lines.push(Line::from(""));

            // Instructions
            lines.push(Line::from(Span::styled(
                "Press Enter to continue, ESC to cancel",
                Style::default().fg(colors::TEXT_DIM).add_modifier(Modifier::DIM),
            )));
            if env_input.has_remaining_optional() {
                lines.push(Line::from(Span::styled(
                    "Press 'a' to skip optional variables",
                    Style::default().fg(colors::TEXT_DIM).add_modifier(Modifier::DIM),
                )));
            }
        }

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_set(border::ROUNDED)
                    .title(Line::from(vec![Span::styled(
                        " Environment Variables ",
                        Style::default()
                            .fg(colors::TEXT)
                            .add_modifier(Modifier::BOLD),
                    )]))
                    .border_style(Style::default().fg(colors::PRIMARY))
                    .style(Style::default().bg(Color::Rgb(15, 23, 42))), // Slate 900
            )
            .style(Style::default().fg(colors::TEXT));

        f.render_widget(paragraph, dialog_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::mcp::registry::types::ServerInstallType;

    fn create_test_servers(count: usize) -> Vec<McpServerInfo> {
        (0..count)
            .map(|i| McpServerInfo {
                qualified_name: format!("test-server-{}", i),
                display_name: format!("Test Server {}", i),
                description: Some(format!("Description for server {}", i)),
                source: if i % 2 == 0 {
                    "registry".to_string()
                } else {
                    "smithery".to_string()
                },
                install: ServerInstallType::Npm {
                    package: format!("@test/server-{}", i),
                },
                author: Some(format!("author-{}", i)),
                downloads: Some(i as u64 * 100),
            })
            .collect()
    }

    #[test]
    fn test_browser_state_new() {
        let servers = create_test_servers(5);
        let state = BrowserState::new(servers.clone(), None);

        assert_eq!(state.servers.len(), 5);
        assert_eq!(state.filtered.len(), 5);
        assert_eq!(state.list_state.selected(), Some(0));
        assert!(state.search_query.is_empty());
        assert!(!state.search_mode);
        assert!(!state.show_help);
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn test_browser_state_new_empty() {
        let servers: Vec<McpServerInfo> = vec![];
        let state = BrowserState::new(servers, None);

        assert_eq!(state.servers.len(), 0);
        assert_eq!(state.filtered.len(), 0);
        assert_eq!(state.list_state.selected(), None);
    }

    #[test]
    fn test_browser_state_source_filter() {
        let servers = create_test_servers(10);
        let state = BrowserState::new(servers, Some("registry".to_string()));

        // Only registry servers should be in filtered (indices 0, 2, 4, 6, 8)
        assert_eq!(state.filtered.len(), 5);
        for &idx in &state.filtered {
            assert_eq!(state.servers[idx].source, "registry");
        }
    }

    #[test]
    fn test_apply_filter_search() {
        let servers = create_test_servers(10);
        let mut state = BrowserState::new(servers, None);

        // Search for "server-5"
        state.search_query = "server-5".to_string();
        state.apply_filter();

        assert_eq!(state.filtered.len(), 1);
        assert_eq!(state.servers[state.filtered[0]].qualified_name, "test-server-5");
    }

    #[test]
    fn test_apply_filter_description() {
        let servers = create_test_servers(10);
        let mut state = BrowserState::new(servers, None);

        // Search in description
        state.search_query = "Description for server 3".to_string();
        state.apply_filter();

        assert_eq!(state.filtered.len(), 1);
        assert_eq!(state.servers[state.filtered[0]].qualified_name, "test-server-3");
    }

    #[test]
    fn test_apply_filter_case_insensitive() {
        let servers = create_test_servers(10);
        let mut state = BrowserState::new(servers, None);

        // Search case insensitive
        state.search_query = "TEST-SERVER-1".to_string();
        state.apply_filter();

        assert_eq!(state.filtered.len(), 1);
    }

    #[test]
    fn test_move_up() {
        let servers = create_test_servers(5);
        let mut state = BrowserState::new(servers, None);

        // Start at 0, move up should stay at 0
        state.move_up();
        assert_eq!(state.list_state.selected(), Some(0));

        // Move to position 2, then move up
        state.list_state.select(Some(2));
        state.move_up();
        assert_eq!(state.list_state.selected(), Some(1));
    }

    #[test]
    fn test_move_down() {
        let servers = create_test_servers(5);
        let mut state = BrowserState::new(servers, None);

        // Start at 0, move down
        state.move_down();
        assert_eq!(state.list_state.selected(), Some(1));

        // Move to last position, move down should stay
        state.list_state.select(Some(4));
        state.move_down();
        assert_eq!(state.list_state.selected(), Some(4));
    }

    #[test]
    fn test_page_up() {
        let servers = create_test_servers(30);
        let mut state = BrowserState::new(servers, None);

        // Start at position 15
        state.list_state.select(Some(15));
        state.page_up(10);
        assert_eq!(state.list_state.selected(), Some(5));

        // Page up from position 5 should go to 0
        state.page_up(10);
        assert_eq!(state.list_state.selected(), Some(0));
    }

    #[test]
    fn test_page_down() {
        let servers = create_test_servers(30);
        let mut state = BrowserState::new(servers, None);

        // Start at 0, page down
        state.page_down(10);
        assert_eq!(state.list_state.selected(), Some(10));

        // Page down from 25 should go to 29 (last item)
        state.list_state.select(Some(25));
        state.page_down(10);
        assert_eq!(state.list_state.selected(), Some(29));
    }

    #[test]
    fn test_home() {
        let servers = create_test_servers(10);
        let mut state = BrowserState::new(servers, None);

        state.list_state.select(Some(5));
        state.home();
        assert_eq!(state.list_state.selected(), Some(0));
    }

    #[test]
    fn test_end() {
        let servers = create_test_servers(10);
        let mut state = BrowserState::new(servers, None);

        state.end();
        assert_eq!(state.list_state.selected(), Some(9));
    }

    #[test]
    fn test_scroll_detail() {
        let servers = create_test_servers(5);
        let mut state = BrowserState::new(servers, None);

        assert_eq!(state.scroll_offset, 0);

        state.scroll_detail_down();
        assert_eq!(state.scroll_offset, 1);

        state.scroll_detail_down();
        assert_eq!(state.scroll_offset, 2);

        state.scroll_detail_up();
        assert_eq!(state.scroll_offset, 1);

        state.scroll_detail_up();
        state.scroll_detail_up(); // Should not go below 0
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn test_selected_server() {
        let servers = create_test_servers(5);
        let state = BrowserState::new(servers, None);

        let selected = state.selected_server();
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().qualified_name, "test-server-0");
    }

    #[test]
    fn test_selected_server_with_filter() {
        let servers = create_test_servers(10);
        let mut state = BrowserState::new(servers, None);

        // Filter to only show server-5
        state.search_query = "server-5".to_string();
        state.apply_filter();

        let selected = state.selected_server();
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().qualified_name, "test-server-5");
    }

    #[test]
    fn test_filter_resets_selection() {
        let servers = create_test_servers(10);
        let mut state = BrowserState::new(servers, None);

        // Select item 5
        state.list_state.select(Some(5));

        // Apply filter that only includes item 3
        state.search_query = "server-3".to_string();
        state.apply_filter();

        // Selection should reset to 0 (first filtered item)
        assert_eq!(state.list_state.selected(), Some(0));
        assert_eq!(state.filtered.len(), 1);
    }

    #[test]
    fn test_navigation_resets_scroll() {
        let servers = create_test_servers(10);
        let mut state = BrowserState::new(servers, None);

        state.scroll_offset = 5;
        state.move_down();
        assert_eq!(state.scroll_offset, 0);

        state.scroll_offset = 5;
        state.move_up();
        assert_eq!(state.scroll_offset, 0);

        state.scroll_offset = 5;
        state.page_down(10);
        assert_eq!(state.scroll_offset, 0);

        state.scroll_offset = 5;
        state.page_up(10);
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn test_env_input_state_new() {
        let specs = vec![
            EnvVarSpec {
                name: "API_KEY".to_string(),
                description: Some("Your API key".to_string()),
                required: true,
                default: None,
            },
            EnvVarSpec {
                name: "API_URL".to_string(),
                description: Some("API endpoint URL".to_string()),
                required: false,
                default: Some("https://api.example.com".to_string()),
            },
        ];

        let env_input = EnvInputState::new(specs);
        assert_eq!(env_input.env_specs.len(), 2);
        assert_eq!(env_input.current_index, 0);
        assert!(env_input.values.is_empty());
        assert!(env_input.input_buffer.is_empty());
    }

    #[test]
    fn test_env_input_state_current_spec() {
        let specs = vec![
            EnvVarSpec {
                name: "VAR1".to_string(),
                description: None,
                required: true,
                default: None,
            },
            EnvVarSpec {
                name: "VAR2".to_string(),
                description: None,
                required: false,
                default: None,
            },
        ];

        let mut env_input = EnvInputState::new(specs);
        assert_eq!(env_input.current_spec().unwrap().name, "VAR1");

        env_input.next();
        assert_eq!(env_input.current_spec().unwrap().name, "VAR2");

        env_input.next();
        assert!(env_input.current_spec().is_none());
        assert!(env_input.is_complete());
    }

    #[test]
    fn test_env_input_state_next_with_value() {
        let specs = vec![
            EnvVarSpec {
                name: "API_KEY".to_string(),
                description: None,
                required: true,
                default: None,
            },
            EnvVarSpec {
                name: "OPTIONAL_VAR".to_string(),
                description: None,
                required: false,
                default: None,
            },
        ];

        let mut env_input = EnvInputState::new(specs);
        env_input.input_buffer = "secret-key-123".to_string();
        env_input.next();

        assert_eq!(env_input.values.get("API_KEY"), Some(&"secret-key-123".to_string()));
        assert_eq!(env_input.current_index, 1);
        assert!(env_input.input_buffer.is_empty());
    }

    #[test]
    fn test_env_input_state_next_empty_optional() {
        let specs = vec![
            EnvVarSpec {
                name: "OPTIONAL_VAR".to_string(),
                description: None,
                required: false,
                default: None,
            },
        ];

        let mut env_input = EnvInputState::new(specs);
        env_input.input_buffer = String::new(); // Empty input for optional
        env_input.next();

        // Empty optional variable should not be stored
        assert!(!env_input.values.contains_key("OPTIONAL_VAR"));
    }

    #[test]
    fn test_env_input_skip_all_optional() {
        let specs = vec![
            EnvVarSpec {
                name: "REQ1".to_string(),
                description: None,
                required: true,
                default: None,
            },
            EnvVarSpec {
                name: "OPT1".to_string(),
                description: None,
                required: false,
                default: None,
            },
            EnvVarSpec {
                name: "OPT2".to_string(),
                description: None,
                required: false,
                default: None,
            },
            EnvVarSpec {
                name: "REQ2".to_string(),
                description: None,
                required: true,
                default: None,
            },
        ];

        let mut env_input = EnvInputState::new(specs);
        env_input.next(); // Move to OPT1
        env_input.skip_all_optional();

        assert_eq!(env_input.current_spec().unwrap().name, "REQ2");
    }

    #[test]
    fn test_env_input_state_get_values() {
        let specs = vec![
            EnvVarSpec {
                name: "VAR1".to_string(),
                description: None,
                required: true,
                default: None,
            },
            EnvVarSpec {
                name: "VAR2".to_string(),
                description: None,
                required: true,
                default: None,
            },
        ];

        let mut env_input = EnvInputState::new(specs);

        // Input VAR1
        env_input.input_buffer = "value1".to_string();
        env_input.next();

        // Input VAR2
        env_input.input_buffer = "value2".to_string();
        env_input.next();

        let values = env_input.get_values();
        assert_eq!(values.len(), 2);
        assert!(values.iter().any(|(k, v)| k == "VAR1" && v == "value1"));
        assert!(values.iter().any(|(k, v)| k == "VAR2" && v == "value2"));
    }

    #[test]
    fn test_env_input_state_whitespace_trimming() {
        let specs = vec![EnvVarSpec {
            name: "VAR".to_string(),
            description: None,
            required: true,
            default: None,
        }];

        let mut env_input = EnvInputState::new(specs);
        env_input.input_buffer = "  spaced value  ".to_string();
        env_input.next();

        assert_eq!(
            env_input.values.get("VAR"),
            Some(&"spaced value".to_string())
        );
    }

    #[test]
    fn test_browser_state_with_env_input() {
        let servers = create_test_servers(5);
        let state = BrowserState::new(servers, None);

        assert!(state.env_input.is_none());

        // Can create and manage env input
        let specs = vec![EnvVarSpec {
            name: "TEST_VAR".to_string(),
            description: None,
            required: true,
            default: None,
        }];

        let mut env_input = EnvInputState::new(specs);
        env_input.input_buffer = "test_value".to_string();

        assert_eq!(env_input.current_spec().unwrap().name, "TEST_VAR");
    }
}
