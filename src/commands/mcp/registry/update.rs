use super::aggregator::RegistryAggregator;
use anyhow::{anyhow, Result};
use colored::Colorize;

pub async fn execute() -> Result<()> {
    let aggregator = RegistryAggregator::new();
    aggregator.clear_cache().await;

    println!("🔄 Updating registry cache...");
    let sources = ["registry", "smithery"];
    let mut success = false;

    for source in sources {
        match aggregator.search("mcp", Some(source), 3).await {
            Ok(results) => {
                success = true;
                println!(
                    "  {} {}: {} result(s) fetched",
                    "✓".green(),
                    source,
                    results.len()
                );
            }
            Err(err) => {
                println!("  {} {} update failed: {}", "⚠️".yellow(), source, err);
            }
        }
    }

    if success {
        println!("{}", "Cache refreshed (in-memory)".green());
        Ok(())
    } else {
        Err(anyhow!(
            "Unable to refresh registry cache; all sources failed."
        ))
    }
}
