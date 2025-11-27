//! edit命令实现 - 在编辑器中编辑配置文件

use super::McpConfigEditor;
use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use std::env;
use std::fs;
use std::process::Command;

pub fn execute() -> Result<()> {
    let editor = McpConfigEditor::new()?;
    let config_path = editor.config_path().clone();

    // 确保配置文件存在
    if !config_path.exists() {
        // 创建空配置
        let empty_config = r#"{
  "mcpServers": {}
}
"#;
        fs::write(&config_path, empty_config).with_context(|| {
            format!("Failed to create config file at {}", config_path.display())
        })?;
    }

    // 获取编辑器
    let editor_cmd = env::var("EDITOR")
        .or_else(|_| env::var("VISUAL"))
        .unwrap_or_else(|_| {
            // 尝试常见编辑器
            if which::which("vim").is_ok() {
                "vim".to_string()
            } else if which::which("nano").is_ok() {
                "nano".to_string()
            } else if which::which("vi").is_ok() {
                "vi".to_string()
            } else {
                "".to_string()
            }
        });

    if editor_cmd.is_empty() {
        eprintln!("{} No editor found", "❌".red());
        println!();
        println!("Please set the $EDITOR environment variable:");
        println!("  export EDITOR=vim");
        println!();
        println!("Or install a text editor:");
        println!("  vim, nano, or vi");
        return Ok(());
    }

    println!(
        "Opening {} in {}...",
        config_path.display().to_string().cyan(),
        editor_cmd.green()
    );

    // 读取编辑前的内容用于验证
    let original_content = fs::read_to_string(&config_path)?;

    // 打开编辑器
    let status = Command::new(&editor_cmd)
        .arg(&config_path)
        .status()
        .with_context(|| format!("Failed to launch editor: {}", editor_cmd))?;

    if !status.success() {
        return Err(anyhow!("Editor exited with non-zero status"));
    }

    // 验证编辑后的JSON
    let new_content = fs::read_to_string(&config_path)?;

    match serde_json::from_str::<serde_json::Value>(&new_content) {
        Ok(_) => {
            // 尝试加载完整配置以验证结构
            match editor.read() {
                Ok(config) => {
                    println!();
                    println!("{} Configuration saved", "✅".green());
                    println!("   {} servers configured", config.mcp_servers.len());
                    println!();
                }
                Err(e) => {
                    // JSON有效但结构不正确，恢复原始内容
                    fs::write(&config_path, original_content)?;
                    eprintln!("{} Invalid MCP configuration structure: {}", "❌".red(), e);
                    println!();
                    println!("Changes have been reverted.");
                    println!();
                }
            }
        }
        Err(e) => {
            // JSON语法错误，恢复原始内容
            fs::write(&config_path, original_content)?;
            eprintln!("{} Invalid JSON syntax", "❌".red());
            println!();
            println!("Error: {}", e);
            println!();
            println!("Changes have been reverted.");
            println!();
        }
    }

    Ok(())
}
