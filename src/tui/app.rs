//! 🚀 TUI应用主入口
//!
//! 基于成熟TUI库的应用启动和管理

use crate::tui::App;

/// TUI应用启动器
pub struct TuiApp;

impl TuiApp {
    /// 启动TUI应用
    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new();
        app.run()
    }

    /// 启动TUI应用并指定初始屏幕
    pub fn run_with_screen(initial_screen: Option<crate::tui::ScreenType>) -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new();
        if let Some(screen) = initial_screen {
            app.set_initial_screen(screen);
        }
        app.run()
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
pub fn run_tui_app_with_screen(initial_screen: Option<crate::tui::ScreenType>) -> Result<(), Box<dyn std::error::Error>> {
    // 初始化
    TuiApp::init()?;

    // 运行应用
    let result = TuiApp::run_with_screen(initial_screen);

    // 清理
    let _ = TuiApp::cleanup();

    result
}