//! get命令实现 - 获取服务器详细配置

use super::McpConfigEditor;
use anyhow::Result;
use colored::Colorize;

pub fn execute(name: &str) -> Result<()> {
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

    // 输出YAML格式的配置
    println!("{}: {}", "name".bold(), name);
    println!("{}: {}", "command".bold(), server.command);
    if let Some(source) = server.source.as_ref() {
        println!("{}: {}", "source".bold(), source);
    }

    if !server.args.is_empty() {
        println!("{}:", "args".bold());
        for arg in &server.args {
            println!("  - \"{}\"", arg);
        }
    }

    if let Some(desc) = server.description {
        println!("{}: {}", "description".bold(), desc);
    }

    if let Some(cat) = server.category {
        println!("{}: {}", "category".bold(), cat);
    }

    println!(
        "{}: {}",
        "enabled".bold(),
        if server.enabled.unwrap_or(true) {
            "true".green().to_string()
        } else {
            "false".yellow().to_string()
        }
    );

    if !server.env.is_empty() {
        println!("{}:", "env".bold());
        for (key, value) in &server.env {
            // 脱敏显示可能的敏感信息
            let display_value = if key.to_uppercase().contains("KEY")
                || key.to_uppercase().contains("TOKEN")
                || key.to_uppercase().contains("SECRET")
            {
                format!("{}...", &value.chars().take(8).collect::<String>())
            } else {
                value.clone()
            };
            println!("  {}: {}", key, display_value);
        }
    }

    println!();

    Ok(())
}
