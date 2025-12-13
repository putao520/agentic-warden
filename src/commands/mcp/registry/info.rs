use super::aggregator::RegistryAggregator;
use crate::commands::mcp::{McpConfigEditor, McpServerConfig};
use anyhow::{anyhow, Result};
use colored::Colorize;

pub async fn execute(name: &str, source: Option<String>) -> Result<()> {
    let editor = McpConfigEditor::new()?;
    let installed_config = editor.get_server(name)?;
    let aggregator = RegistryAggregator::new();

    let detail = match aggregator.get_server_detail(name, source.as_deref()).await {
        Ok(detail) => detail,
        Err(err) => {
            if let Some(cfg) = installed_config {
                print_installed_only(name, &cfg);
                return Ok(());
            }
            return Err(anyhow!("Failed to fetch server info: {}", err));
        }
    };

    println!("MCP Server: {}", detail.info.qualified_name.bold());
    println!("  Source: {}", detail.info.source);
    println!("  Type:   {}", detail.info.install);

    if let Some(desc) = &detail.info.description {
        println!("  Description: {}", desc);
    }
    if let Some(repo) = &detail.repository {
        println!("  Repository: {}", repo);
    }

    if let Some(cfg) = installed_config {
        println!("  Installed: {}", "yes".green());
        println!("  Command:   {} {}", cfg.command, cfg.args.join(" "));
    } else {
        let (cmd, args) = detail.info.install.command_and_args();
        println!("  Installed: {}", "no".yellow());
        println!("  Command:   {} {}", cmd, args.join(" "));
    }

    if !detail.required_env.is_empty() {
        println!("\nRequired environment variables:");
        for env in &detail.required_env {
            let marker = if env.required { "*" } else { "-" };
            let desc = env.description.clone().unwrap_or_else(|| "-".to_string());
            println!("  {} {} - {}", marker, env.name, desc);
        }
    }

    Ok(())
}

fn print_installed_only(name: &str, cfg: &McpServerConfig) {
    println!("MCP Server: {}", name.bold());
    println!("  Source: {}", cfg.source.clone().unwrap_or_else(|| "unknown".to_string()));
    println!(
        "  Command: {} {}",
        cfg.command,
        if cfg.args.is_empty() {
            "".to_string()
        } else {
            cfg.args.join(" ")
        }
    );
    println!("  Installed (offline view)");
}
