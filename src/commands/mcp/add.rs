//! add命令实现 - 添加MCP服务器

use super::{McpConfigEditor, McpServerConfig};
use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;

pub fn execute(
    name: &str,
    command: &str,
    args: Vec<String>,
    description: Option<String>,
    category: Option<String>,
    env: Vec<(String, String)>,
    disabled: bool,
) -> Result<()> {
    let editor = McpConfigEditor::new()?;

    // 检查服务器是否已存在
    if editor.server_exists(name)? {
        eprintln!(
            "{} MCP server '{}' already exists",
            "❌".red(),
            name.yellow()
        );
        println!();
        println!("To update: {} mcp edit", "aiw".cyan());
        println!("To remove: {} mcp remove {}", "aiw".cyan(), name);
        return Ok(());
    }

    // 构建环境变量HashMap
    let env_map: HashMap<String, String> = env.into_iter().collect();

    // 创建服务器配置
    let server_config = McpServerConfig {
        command: command.to_string(),
        args: args.clone(),
        env: env_map,
        description: description.clone(),
        category: category.clone(),
        enabled: if disabled { Some(false) } else { Some(true) },
        source: None,
    };

    // 添加服务器
    editor.add_server(name, server_config)?;

    // 输出成功消息
    println!("{} Added MCP server '{}'", "✅".green(), name.cyan());
    println!();
    println!("Configuration:");
    println!("  Command: {} {}", command, args.join(" "));
    if let Some(desc) = description {
        println!("  Description: {}", desc);
    }
    if let Some(cat) = category {
        println!("  Category: {}", cat);
    }
    println!(
        "  Status: {}",
        if disabled {
            "disabled".yellow()
        } else {
            "enabled".green()
        }
    );
    println!();
    println!("Configuration saved to {}", "~/.aiw/mcp.json".cyan());
    println!("Restart your AI CLI to apply changes.");
    println!();

    Ok(())
}
