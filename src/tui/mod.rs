//! 🎨 Modern TUI System - 直接使用成熟TUI组件
//!
//! 基于 ratatui 的统一 TUI 架构，所有屏幕通过共享的应用状态协同工作。

use std::time::Instant;
use crossterm::{
    event::{self, Event, KeyEvent, KeyCode},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, ClearType, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use std::{collections::HashMap, io::stdout, time::Duration};

mod data_binding;

pub mod app;
pub mod app_state;
pub mod components;
pub mod screens;

use self::data_binding::DataBindingController;

// 重新导出常用类型
pub use screens::{ExternalScreen, Screen, ScreenAction, ScreenType};

/// 全局 TUI 应用容器
pub struct App {
    should_quit: bool,
    current_screen: ScreenType,
    screens: HashMap<ScreenType, Box<dyn screens::Screen>>,
    history: Vec<ScreenType>,
    #[allow(dead_code)]
    data_binding: DataBindingController,
    last_update: Instant,
    /// External screen to launch after TUI exits
    launch_external: Option<ExternalScreen>,
}

impl App {
    pub fn new() -> Self {
        Self::with_initial_screen(ScreenType::Dashboard)
    }

    pub fn with_initial_screen(initial_screen: ScreenType) -> Self {
        let history = vec![initial_screen.clone()];
        Self {
            should_quit: false,
            current_screen: initial_screen,
            screens: HashMap::new(),
            history,
            data_binding: DataBindingController::start(),
            last_update: Instant::now(),
            launch_external: None,
        }
    }

    /// 设置初始屏幕（仅在启动时调用）
    pub fn set_initial_screen(&mut self, screen: ScreenType) {
        self.current_screen = screen.clone();
        self.history.clear();
        self.history.push(screen);
    }

    /// Run the TUI application
    /// Returns Ok(Some(external)) if an external screen should be launched after exit
    pub fn run(&mut self) -> Result<Option<ExternalScreen>, Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            crossterm::terminal::Clear(ClearType::All)
        )?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        let tick_rate = Duration::from_millis(16); // ~60 FPS for better responsiveness

        const UPDATE_INTERVAL: Duration = Duration::from_millis(100);

        loop {
            self.ensure_screen_ready()?;

            // 节流更新，避免过于频繁的 screen.update() 调用
            let now = Instant::now();
            if now.duration_since(self.last_update) >= UPDATE_INTERVAL {
                if let Some(screen) = self.screens.get_mut(&self.current_screen) {
                    screen.update()?;
                }
                self.last_update = now;
            }

            terminal.draw(|frame| {
                self.render(frame);
            })?;

            if event::poll(tick_rate)? {
                match event::read()? {
                    Event::Key(key) => self.handle_input(key)?,
                    Event::Resize(_, _) => {
                        // 下一次循环会重新渲染
                    }
                    _ => {}
                }
            }

            if self.should_quit || self.launch_external.is_some() {
                break;
            }
        }

        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            crossterm::terminal::Clear(ClearType::All)
        )?;
        disable_raw_mode()?;
        Ok(self.launch_external.take())
    }

    fn ensure_screen_ready(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.screens.contains_key(&self.current_screen) {
            let screen = match self.current_screen.create() {
                Ok(screen) => screen,
                Err(err) => {
                    let io_err = std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("failed to create screen {}: {}", self.current_screen, err),
                    );
                    return Err(Box::new(io_err));
                }
            };
            self.screens.insert(self.current_screen.clone(), screen);
        }
        Ok(())
    }

    fn handle_input(&mut self, key: KeyEvent) -> Result<(), Box<dyn std::error::Error>> {
        self.ensure_screen_ready()?;

        // Handle global key bindings first
        match key.code {
            KeyCode::Char('1') => {
                self.current_screen = crate::tui::ScreenType::Dashboard;
                self.history.push(crate::tui::ScreenType::Dashboard);
                return Ok(());
            }
            KeyCode::Char('2') => {
                self.current_screen = crate::tui::ScreenType::Status;
                self.history.push(crate::tui::ScreenType::Status);
                return Ok(());
            }
            KeyCode::Char('3') => {
                self.current_screen = crate::tui::ScreenType::Provider;
                self.history.push(crate::tui::ScreenType::Provider);
                return Ok(());
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                // Launch MCP Browse external TUI
                self.launch_external = Some(ExternalScreen::McpBrowse);
                return Ok(());
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.should_quit = true;
                return Ok(());
            }
            KeyCode::Esc => {
                if self.history.len() > 1 {
                    self.history.pop();
                    // Safe: We just checked that len > 1, so after pop there's at least 1 element
                    self.current_screen = self
                        .history
                        .last()
                        .expect("history should have at least one screen after pop")
                        .clone();
                }
                return Ok(());
            }
            _ => {}
        }

        // If no global binding was matched, pass to the current screen
        let screen = self
            .screens
            .get_mut(&self.current_screen)
            .expect("current screen must be initialised");

        match screen.handle_key(key)? {
            ScreenAction::None => {}
            ScreenAction::SwitchTo(new_screen) => {
                self.current_screen = new_screen.clone();
                self.history.push(new_screen);
            }
            ScreenAction::Back => {
                if self.history.len() > 1 {
                    self.history.pop();
                    // Safe: We just checked that len > 1, so after pop there's at least 1 element
                    self.current_screen = self
                        .history
                        .last()
                        .expect("history should have at least one screen after pop")
                        .clone();
                }
            }
            ScreenAction::Quit => {
                self.should_quit = true;
            }
            ScreenAction::LaunchExternal(external) => {
                self.launch_external = Some(external);
            }
        }

        Ok(())
    }

    fn render(&mut self, frame: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3), // 标题栏
                    Constraint::Min(0),    // 主内容区
                    Constraint::Length(3), // 快捷键提示
                ]
                .as_ref(),
            )
            .split(frame.size());

        self.render_title_bar(frame, chunks[0]);

        if let Some(screen) = self.screens.get_mut(&self.current_screen) {
            screen.render(frame, chunks[1]);
        }

        self.render_key_hints(frame, chunks[2]);
    }

    fn render_title_bar(&self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        let title = format!("🚀 Agentic Warden - {}", self.current_screen.to_string());
        let paragraph = Paragraph::new(title)
            .style(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan))
            .block(ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::ALL));
        frame.render_widget(paragraph, area);
    }

    fn render_key_hints(&self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        let hints = vec!["1:Dashboard 2:Status 3:Provider  m:MCP Browse  q:Quit  ESC:Back"];

        let text: Vec<ratatui::text::Line> = hints
            .into_iter()
            .map(|hint| {
                ratatui::text::Line::from(vec![ratatui::text::Span::styled(
                    hint,
                    ratatui::style::Style::default().fg(ratatui::style::Color::White),
                )])
            })
            .collect();

        let paragraph = Paragraph::new(text)
            .style(
                ratatui::style::Style::default()
                    .fg(ratatui::style::Color::White)
                    .bg(ratatui::style::Color::DarkGray),
            )
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
