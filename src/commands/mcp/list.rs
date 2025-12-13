//! list命令实现 - 列出所有MCP服务器

use super::McpConfigEditor;
use anyhow::Result;
use colored::Colorize;
use prettytable::{format, Cell, Row, Table};

pub fn execute() -> Result<()> {
    let editor = McpConfigEditor::new()?;
    let servers = editor.list_servers()?;
    let (total, enabled, disabled) = editor.server_stats()?;

    if servers.is_empty() {
        println!("{}", "No MCP servers configured".yellow());
        println!();
        println!("To add your first MCP server:");
        println!("  {} mcp add <name> <command> [args...]", "aiw".cyan());
        println!();
        println!("Example:");
        println!(
            "  {} mcp add filesystem npx -y @modelcontextprotocol/server-filesystem /home/user",
            "aiw".cyan()
        );
        return Ok(());
    }

    println!("{}", format!("MCP Servers ({})", "~/.aiw/mcp.json".cyan()));
    println!();

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);

    // 表头
    table.add_row(Row::new(vec![
        Cell::new("NAME").style_spec("b"),
        Cell::new("SOURCE").style_spec("b"),
        Cell::new("COMMAND").style_spec("b"),
        Cell::new("STATUS").style_spec("b"),
        Cell::new("DESCRIPTION").style_spec("b"),
    ]));

    // 添加每个服务器
    for (name, config) in servers {
        let status = if config.enabled.unwrap_or(true) {
            "enabled".green().to_string()
        } else {
            "disabled".yellow().to_string()
        };

        let description = config.description.unwrap_or_else(|| "-".to_string());
        let source = config.source.unwrap_or_else(|| "-".to_string());

        table.add_row(Row::new(vec![
            Cell::new(&name),
            Cell::new(&source),
            Cell::new(&config.command),
            Cell::new(&status),
            Cell::new(&description),
        ]));
    }

    table.printstd();
    println!();
    println!(
        "Total: {} servers ({} enabled, {} disabled)",
        total.to_string().cyan(),
        enabled.to_string().green(),
        disabled.to_string().yellow()
    );
    println!();

    Ok(())
}
