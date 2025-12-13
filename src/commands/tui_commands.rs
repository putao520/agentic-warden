//! TUI 管理命令处理逻辑
//!
//! 处理 Dashboard、Status、Provider 管理等 TUI 相关命令

use anyhow::Result;
use std::process::ExitCode;

/// TUI 命令类型
#[derive(Debug, Clone)]
pub enum TuiCommand {
    Dashboard,
    Status,
    ProviderManagement,
}

impl TuiCommand {
    /// 执行 TUI 命令
    ///
    /// 启动相应的 TUI 界面。实际的 TUI 实现在 crate::tui::app 模块中。
    pub async fn execute(&self) -> Result<ExitCode> {
        // 初始化 color-eyre 以获得更好的错误处理
        color_eyre::install()
            .map_err(|e| anyhow::anyhow!("Failed to install color_eyre: {}", e))?;

        match self {
            TuiCommand::Dashboard => {
                // 启动 Dashboard TUI (默认主界面)
                crate::tui::app::run_tui_app()
                    .map_err(|e| anyhow::anyhow!("Dashboard TUI error: {}", e))?;
                Ok(ExitCode::from(0))
            }
            TuiCommand::Status => {
                // 启动 Status TUI
                crate::tui::app::run_tui_app_with_screen(Some(crate::tui::ScreenType::Status))
                    .map_err(|e| anyhow::anyhow!("Status TUI error: {}", e))?;
                Ok(ExitCode::from(0))
            }
            TuiCommand::ProviderManagement => {
                // 启动 Provider 管理 TUI
                crate::tui::app::run_tui_app_with_screen(Some(crate::tui::ScreenType::Provider))
                    .map_err(|e| anyhow::anyhow!("Provider management TUI error: {}", e))?;
                Ok(ExitCode::from(0))
            }
        }
    }
}
