//! 🚀 TUI应用主入口
//!
//! 基于成熟TUI库的应用启动和管理

use crate::tui::{App, ExternalScreen};
use tokio::runtime::Handle;

/// TUI应用启动器
pub struct TuiApp;

impl TuiApp {
    /// 在当前 Tokio 运行时中执行异步函数
    /// 使用 block_in_place 避免嵌套运行时问题
    fn run_async<F, T>(future: F) -> Result<T, Box<dyn std::error::Error>>
    where
        F: std::future::Future<Output = Result<T, anyhow::Error>>,
    {
        // 检查是否在 Tokio 运行时中
        match Handle::try_current() {
            Ok(handle) => {
                // 在现有运行时中，使用 block_in_place 安全阻塞
                Ok(tokio::task::block_in_place(|| handle.block_on(future))?)
            }
            Err(_) => {
                // 不在运行时中，创建新的运行时
                Ok(tokio::runtime::Runtime::new()?.block_on(future)?)
            }
        }
    }

    /// 启动TUI应用
    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new();
        loop {
            match app.run()? {
                Some(ExternalScreen::McpBrowse) => {
                    // Launch MCP Browse TUI (async function)
                    Self::run_async(crate::commands::mcp::registry::browse::execute(None))?;
                    // After MCP Browse exits, continue with our TUI
                    app = App::new();
                }
                None => break,
            }
        }
        Ok(())
    }

    /// 启动TUI应用并指定初始屏幕
    pub fn run_with_screen(
        initial_screen: Option<crate::tui::ScreenType>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new();
        if let Some(screen) = initial_screen.clone() {
            app.set_initial_screen(screen);
        }
        loop {
            match app.run()? {
                Some(ExternalScreen::McpBrowse) => {
                    // Launch MCP Browse TUI (async function)
                    Self::run_async(crate::commands::mcp::registry::browse::execute(None))?;
                    // After MCP Browse exits, recreate app with initial screen
                    app = App::new();
                    if let Some(screen) = initial_screen.clone() {
                        app.set_initial_screen(screen);
                    }
                }
                None => break,
            }
        }
        Ok(())
    }

    /// 初始化TUI环境
    pub fn init() -> Result<(), Box<dyn std::error::Error>> {
        // 这里可以添加额外的初始化逻辑
        // 比如日志初始化、配置加载等
        Ok(())
    }

    /// 清理TUI环境
    pub fn cleanup() -> Result<(), Box<dyn std::error::Error>> {
        // 这里可以添加清理逻辑
        Ok(())
    }
}

/// 运行TUI应用的便捷函数
pub fn run_tui_app() -> Result<(), Box<dyn std::error::Error>> {
    run_tui_app_with_screen(None)
}

/// 运行TUI应用并指定初始屏幕
pub fn run_tui_app_with_screen(
    initial_screen: Option<crate::tui::ScreenType>,
) -> Result<(), Box<dyn std::error::Error>> {
    // 初始化
    TuiApp::init()?;

    // 运行应用
    let result = TuiApp::run_with_screen(initial_screen);

    // 清理
    let _ = TuiApp::cleanup();

    result
}
