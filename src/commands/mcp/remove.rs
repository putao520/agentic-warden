//! remove命令实现 - 移除MCP服务器

use super::McpConfigEditor;
use anyhow::Result;
use colored::Colorize;
use dialoguer::Confirm;

pub fn execute(name: &str, yes: bool) -> Result<()> {
    let editor = McpConfigEditor::new()?;

    // 获取服务器配置
    let server = match editor.get_server(name)? {
        Some(s) => s,
        None => {
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
    };

    // 显示要移除的服务器信息
    println!(
        "{} You are about to remove MCP server '{}':",
        "⚠️".yellow(),
        name.cyan()
    );
    println!();
    println!("  Command: {} {}", server.command, server.args.join(" "));
    if let Some(desc) = server.description {
        println!("  Description: {}", desc);
    }
    if let Some(cat) = server.category {
        println!("  Category: {}", cat);
    }
    let status = if server.enabled.unwrap_or(true) {
        "enabled".green()
    } else {
        "disabled".yellow()
    };
    println!("  Status: {}", status);
    println!();

    // 确认提示
    let confirmed = if yes {
        true
    } else {
        Confirm::new()
            .with_prompt("Continue?")
            .default(false)
            .interact()?
    };

    if !confirmed {
        println!("{}", "Cancelled".yellow());
        return Ok(());
    }

    // 移除服务器
    editor.remove_server(name)?;

    println!();
    println!("{} Removed MCP server '{}'", "✅".green(), name.cyan());
    println!();
    println!("Configuration saved to {}", "~/.aiw/mcp.json".cyan());
    println!();

    Ok(())
}
