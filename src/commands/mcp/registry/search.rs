use super::{aggregator::RegistryAggregator, install, interactive};
use anyhow::{anyhow, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

pub async fn execute(query: &str, source: Option<String>, limit: Option<usize>) -> Result<()> {
    let aggregator = RegistryAggregator::new();
    let spinner = ProgressBar::new_spinner()
        .with_style(
            ProgressStyle::default_spinner()
                .template("{spinner} {msg}")
                .unwrap(),
        )
        .with_message("Searching MCP servers...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let results = aggregator
        .search(query, source.as_deref(), limit.unwrap_or(20))
        .await?;
    spinner.finish_and_clear();

    if results.is_empty() {
        return Err(anyhow!("No MCP servers found for '{}'", query));
    }

    println!("🔍 Results for '{}':", query);
    interactive::render_results(&results);

    if let Some(index) = interactive::prompt_selection(results.len())? {
        let selected = &results[index];
        println!(
            "Installing {} from {}",
            selected.qualified_name.cyan(),
            selected.source
        );
        install::install_with_aggregator(
            &aggregator,
            &selected.qualified_name,
            Some(selected.source.clone()),
            Vec::new(),
            false,
        )
        .await?;
    }

    Ok(())
}
