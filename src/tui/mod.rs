//! 🎨 Modern TUI System - 直接使用成熟TUI组件库
//!
//! 基于ratatui + tui-widgets + tui-textarea等成熟组件库的一步到位TUI实现

use ratatui::{
    backend::CrosstermBackend,
    Terminal,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use crossterm::{
    event::{self, Event, KeyEvent},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
        ClearType,
    },
};
use std::io::{stdout};
use std::collections::HashMap;

pub mod app;
pub mod screens;
pub mod app_state;

// 重新导出常用类型
pub use screens::{Screen, ScreenAction, ScreenType};

/// 全局TUI应用状态 - 直接使用成熟组件库
pub struct App {
    should_quit: bool,
    current_screen: ScreenType,
    screens: HashMap<ScreenType, Box<dyn screens::Screen>>,
    history: Vec<ScreenType>,
}

impl App {
    pub fn new() -> Self {
        Self::with_initial_screen(ScreenType::Dashboard)
    }

    pub fn with_initial_screen(initial_screen: ScreenType) -> Self {
        let mut screens: HashMap<ScreenType, Box<dyn screens::Screen>> = HashMap::new();
        let history = vec![];

        Self {
            should_quit: false,
            current_screen: initial_screen,
            screens,
            history,
        }
    }

    /// 设置初始屏幕（仅在启动时调用）
    pub fn set_initial_screen(&mut self, screen: ScreenType) {
        self.current_screen = screen.clone();
        self.history.push(screen);
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // 初始化terminal
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen, crossterm::terminal::Clear(ClearType::All))?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        loop {
            terminal.draw(|frame| {
                self.render(frame);
            })?;

            // 处理输入事件
            if let Event::Key(key) = event::read()? {
                self.handle_input(key)?;
            }

            if self.should_quit {
                break;
            }
        }

        // 恢复终端
        execute!(terminal.backend_mut(), LeaveAlternateScreen, crossterm::terminal::Clear(ClearType::All))?;
        disable_raw_mode()?;
        Ok(())
    }

    fn handle_input(&mut self, key: KeyEvent) -> Result<(), Box<dyn std::error::Error>> {
        // 获取或创建当前屏幕
        let screen = self.screens.entry(self.current_screen.clone())
            .or_insert_with(|| match self.current_screen.create() {
                Ok(screen) => screen,
                Err(e) => panic!("Failed to create screen {}: {}", self.current_screen, e),
            });

        // 处理输入
        match screen.handle_key(key)? {
            screens::ScreenAction::None => {}
            screens::ScreenAction::SwitchTo(new_screen) => {
                // 切换到新屏幕
                self.current_screen = new_screen.clone();
                self.history.push(new_screen);
            }
            screens::ScreenAction::Back => {
                // 返回上一个屏幕
                if self.history.len() > 1 {
                    self.history.pop();
                    self.current_screen = self.history.last().unwrap().clone();
                }
            }
            screens::ScreenAction::Quit => {
                self.should_quit = true;
            }
        }

        Ok(())
    }

    fn render(&mut self, frame: &mut ratatui::Frame) {
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(3),  // 标题栏
                ratatui::layout::Constraint::Min(0),     // 主内容区
                ratatui::layout::Constraint::Length(3),  // 快捷键提示
            ])
            .split(frame.size());

        // 渲染标题栏
        self.render_title_bar(frame, chunks[0]);

        // 渲染主内容区域
        if let Some(screen) = self.screens.get_mut(&self.current_screen) {
            screen.render(frame, chunks[1]);
        }

        // 渲染快捷键提示
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
        let hints = vec![
            "1:Dashboard 2:Status 3:Provider 4:Push/Pull  q:Quit  ESC:Back",
        ];

        let text: Vec<ratatui::text::Line> = hints
            .into_iter()
            .map(|hint| {
                ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled(hint, ratatui::style::Style::default().fg(ratatui::style::Color::White)),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(text)
            .style(ratatui::style::Style::default().fg(ratatui::style::Color::White).bg(ratatui::style::Color::DarkGray))
            .block(ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::ALL))
            .wrap(ratatui::widgets::Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }
}