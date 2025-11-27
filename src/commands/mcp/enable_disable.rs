//! enable/disable命令实现 - 启用/禁用MCP服务器

use super::McpConfigEditor;
use anyhow::Result;
use colored::Colorize;

pub fn execute_enable(name: &str) -> Result<()> {
    let editor = McpConfigEditor::new()?;

    // 检查服务器是否存在
    if !editor.server_exists(name)? {
        eprintln!("{} MCP server '{}' not found", "❌".red(), name.yellow());
        println!();
        println!("Available servers:");
        let servers = editor.list_servers()?;
        for (server_name, _) in servers {
            println!("  • {}", server_name);
        }
        println!();
        println!(
            "Use '{}' to see all servers",
            format!("{} mcp list", "aiw").cyan()
        );
        return Ok(());
    }

    // 启用服务器
    editor.set_server_enabled(name, true)?;

    println!("{} Enabled MCP server '{}'", "✅".green(), name.cyan());
    println!();
    println!("Restart your AI CLI to apply changes.");
    println!();

    Ok(())
}

pub fn execute_disable(name: &str) -> Result<()> {
    let editor = McpConfigEditor::new()?;

    // 检查服务器是否存在
    if !editor.server_exists(name)? {
        eprintln!("{} MCP server '{}' not found", "❌".red(), name.yellow());
        println!();
        println!("Available servers:");
        let servers = editor.list_servers()?;
        for (server_name, _) in servers {
            println!("  • {}", server_name);
        }
        println!();
        println!(
            "Use '{}' to see all servers",
            format!("{} mcp list", "aiw").cyan()
        );
        return Ok(());
    }

    // 禁用服务器
    editor.set_server_enabled(name, false)?;

    println!("{} Disabled MCP server '{}'", "✅".green(), name.cyan());
    println!();
    println!("The server configuration is preserved but will not be loaded.");
    println!("To re-enable: {} mcp enable {}", "aiw".cyan(), name);
    println!();

    Ok(())
}
