use super::{aggregator::RegistryAggregator, interactive, types::McpServerDetail};
use crate::commands::mcp::{McpConfigEditor, McpServerConfig};
use anyhow::{anyhow, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;

pub async fn execute(
    name: &str,
    source: Option<String>,
    env_vars: Vec<(String, String)>,
    skip_env: bool,
) -> Result<()> {
    let aggregator = RegistryAggregator::new();
    install_with_aggregator(&aggregator, name, source, env_vars, skip_env).await
}

pub async fn install_with_aggregator(
    aggregator: &RegistryAggregator,
    name: &str,
    source: Option<String>,
    env_vars: Vec<(String, String)>,
    skip_env: bool,
) -> Result<()> {
    let spinner = ProgressBar::new_spinner()
        .with_style(
            ProgressStyle::default_spinner()
                .template("{spinner} {msg}")
                .unwrap(),
        )
        .with_message("Resolving server details...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let detail = aggregator
        .get_server_detail(name, source.as_deref())
        .await?;

    let mut config = aggregator
        .get_install_config(name, source.as_deref())
        .await?;
    apply_detail_metadata(&detail, &mut config);

    let provided_env = parse_env_pairs(env_vars);
    let resolved_env =
        interactive::collect_env_vars(&detail.required_env, &provided_env, skip_env)?;

    // Merge resolved env vars, prioritising user-provided or interactive values
    for (key, value) in resolved_env {
        config.env.insert(key, value);
    }

    for (key, value) in provided_env {
        config
            .env
            .entry(key.clone())
            .or_insert_with(|| normalize_env_reference(&key, &value));
    }

    spinner.finish_and_clear();
    write_config(&detail, config)
}

fn apply_detail_metadata(detail: &McpServerDetail, config: &mut McpServerConfig) {
    if config.description.is_none() {
        config.description = detail.info.description.clone();
    }
    if config.source.is_none() {
        config.source = Some(detail.info.source.clone());
    }
}

fn parse_env_pairs(pairs: Vec<(String, String)>) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (key, value) in pairs {
        if !key.trim().is_empty() {
            map.insert(key, value);
        }
    }
    map
}

fn write_config(detail: &McpServerDetail, config: McpServerConfig) -> Result<()> {
    let editor = McpConfigEditor::new()?;
    let name = detail.info.qualified_name.clone();

    if editor.server_exists(&name)? {
        return Err(anyhow!("MCP server '{}' already exists in config", name));
    }

    editor.add_server(&name, config)?;

    println!(
        "{} Installed {} from {}",
        "✅".green(),
        name.cyan(),
        detail.info.source
    );
    println!(
        "Configuration saved to {}",
        editor.config_path().display()
    );
    Ok(())
}

fn normalize_env_reference(name: &str, raw: &str) -> String {
    if raw.starts_with("${") && raw.ends_with('}') {
        raw.to_string()
    } else if raw.starts_with('$') {
        let cleaned = raw.trim_start_matches('$').trim_matches('{').trim_matches('}');
        format!("${{{}}}", cleaned)
    } else if raw.is_empty() {
        format!("${{{}}}", name)
    } else {
        std::env::set_var(name, raw);
        format!("${{{}}}", name)
    }
}
