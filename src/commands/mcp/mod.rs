//! MCP管理CLI命令
//!
//! 提供对 ~/.aiw/mcp.json 的管理命令

mod add;
pub mod config_editor;
mod edit;
mod enable_disable;
mod get;
mod list;
mod remove;
pub mod registry;

pub use config_editor::{McpConfigEditor, McpServerConfig};

use anyhow::Result;

/// MCP命令枚举
#[derive(Debug, Clone)]
pub enum McpCommand {
    /// 列出所有MCP服务器
    List,
    /// 添加MCP服务器
    Add {
        name: String,
        command: String,
        args: Vec<String>,
        description: Option<String>,
        category: Option<String>,
        env: Vec<(String, String)>,
        disabled: bool,
    },
    /// 移除MCP服务器
    Remove { name: String, yes: bool },
    /// 获取服务器配置
    Get { name: String },
    /// 启用服务器
    Enable { name: String },
    /// 禁用服务器
    Disable { name: String },
    /// 编辑配置文件
    Edit,

    /// 搜索MCP服务器
    Search {
        query: String,
        source: Option<String>,
        limit: Option<usize>,
    },

    /// 安装MCP服务器
    Install {
        name: String,
        source: Option<String>,
        env: Vec<(String, String)>,
        skip_env: bool,
    },

    /// 查看服务器信息
    Info { name: String, source: Option<String> },

    /// 更新仓库缓存
    Update,

    /// 交互式浏览所有服务器
    Browse { source: Option<String> },
}

/// 执行MCP命令
pub async fn handle_mcp_command(cmd: McpCommand) -> Result<()> {
    match cmd {
        McpCommand::List => list::execute(),
        McpCommand::Add {
            name,
            command,
            args,
            description,
            category,
            env,
            disabled,
        } => add::execute(&name, &command, args, description, category, env, disabled),
        McpCommand::Remove { name, yes } => remove::execute(&name, yes),
        McpCommand::Get { name } => get::execute(&name),
        McpCommand::Enable { name } => enable_disable::execute_enable(&name),
        McpCommand::Disable { name } => enable_disable::execute_disable(&name),
        McpCommand::Edit => edit::execute(),
        McpCommand::Search {
            query,
            source,
            limit,
        } => registry::search::execute(&query, source, limit).await,
        McpCommand::Install {
            name,
            source,
            env,
            skip_env,
        } => registry::install::execute(&name, source, env, skip_env).await,
        McpCommand::Info { name, source } => registry::info::execute(&name, source).await,
        McpCommand::Update => registry::update::execute().await,
        McpCommand::Browse { source } => registry::browse::execute(source).await,
    }
}
