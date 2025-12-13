use super::types::{EnvVarSpec, McpServerInfo};
use anyhow::{anyhow, Result};
use colored::Colorize;
use dialoguer::{Confirm, Input};
use prettytable::{format, Cell, Row, Table};
use std::collections::HashMap;

pub fn render_results(results: &[McpServerInfo]) {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.add_row(Row::new(vec![
        Cell::new("#").style_spec("b"),
        Cell::new("Name").style_spec("b"),
        Cell::new("Source").style_spec("b"),
        Cell::new("Type").style_spec("b"),
        Cell::new("Description").style_spec("b"),
    ]));

    for (idx, result) in results.iter().enumerate() {
        table.add_row(Row::new(vec![
            Cell::new(&format!("{}", idx + 1)),
            Cell::new(&result.qualified_name),
            Cell::new(&result.source),
            Cell::new(result.install.label()),
            Cell::new(&result.short_description()),
        ]));
    }

    table.printstd();
}

pub fn prompt_selection(total: usize) -> Result<Option<usize>> {
    if total == 0 {
        return Ok(None);
    }

    loop {
        let input: String = Input::new()
            .with_prompt("Select to install (number or 'q' to quit)")
            .interact_text()?;

        let trimmed = input.trim().to_lowercase();
        if trimmed == "q" {
            return Ok(None);
        }

        if let Ok(num) = trimmed.parse::<usize>() {
            if num >= 1 && num <= total {
                return Ok(Some(num - 1));
            }
        }

        println!("{}", "Invalid selection, please try again.".yellow());
    }
}

pub fn collect_env_vars(
    specs: &[EnvVarSpec],
    provided: &HashMap<String, String>,
    skip_env: bool,
) -> Result<HashMap<String, String>> {
    let mut env = HashMap::new();

    for spec in specs {
        if let Some(value) = provided.get(&spec.name) {
            env.insert(spec.name.clone(), normalize_env_value(&spec.name, value));
            continue;
        }

        if let Ok(existing) = std::env::var(&spec.name) {
            let use_existing = Confirm::new()
                .with_prompt(format!(
                    "Found existing {}. Use current environment value?",
                    spec.name
                ))
                .default(true)
                .interact()?;

            if use_existing {
                env.insert(spec.name.clone(), format!("${{{}}}", spec.name));
                continue;
            }

            if !skip_env {
                let value = prompt_env_value(spec, Some(existing))?;
                env.insert(spec.name.clone(), normalize_env_value(&spec.name, &value));
                continue;
            }
        }

        if skip_env {
            if spec.required {
                return Err(anyhow!(
                    "Missing required environment variable {} (use --env or remove --skip-env)",
                    spec.name
                ));
            }
            continue;
        }

        let value = prompt_env_value(spec, None)?;
        env.insert(spec.name.clone(), normalize_env_value(&spec.name, &value));
    }

    Ok(env)
}

fn prompt_env_value(spec: &EnvVarSpec, existing: Option<String>) -> Result<String> {
    println!();
    println!("{} (required: {})", spec.name.bold(), spec.required);
    if let Some(desc) = &spec.description {
        println!("  {}", desc);
    }
    if let Some(default) = &spec.default {
        println!("  Default: {}", default);
    }
    if let Some(current) = existing {
        println!("  Current value detected, leave empty to keep it.");
        let input: String = Input::new()
            .with_prompt(format!("Enter {}", spec.name))
            .allow_empty(true)
            .interact_text()?;
        if input.is_empty() {
            return Ok(current);
        }
        return Ok(input);
    }

    if spec.required {
        let input: String = Input::new()
            .with_prompt(format!("Enter {}", spec.name))
            .validate_with(|val: &String| {
                if val.trim().is_empty() {
                    Err("Value cannot be empty")
                } else {
                    Ok(())
                }
            })
            .interact_text()?;
        Ok(input)
    } else {
        let input: String = Input::new()
            .with_prompt(format!("Enter {} (optional)", spec.name))
            .allow_empty(true)
            .interact_text()?;
        Ok(input)
    }
}

fn normalize_env_value(name: &str, raw: &str) -> String {
    if raw.starts_with("${") && raw.ends_with('}') {
        raw.to_string()
    } else if raw.starts_with('$') {
        let cleaned = raw.trim_start_matches('$').trim_matches('{').trim_matches('}');
        format!("${{{}}}", cleaned)
    } else if raw.is_empty() {
        format!("${{{}}}", name)
    } else {
        // Store as reference and make best effort to set current process env
        std::env::set_var(name, raw);
        format!("${{{}}}", name)
    }
}
