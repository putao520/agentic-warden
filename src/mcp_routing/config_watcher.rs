//! MCP配置文件监听和热重载
//!
//! 监听 ~/.aiw/mcp.json 文件变化并自动重载配置

use crate::mcp_routing::{config::McpConfigManager, McpConnectionPool};
use anyhow::{Context, Result};
use notify::{
    event::{AccessKind, AccessMode, ModifyKind},
    Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::mpsc;

/// Start watching MCP configuration file for changes
pub async fn start_config_watcher(
    connection_pool: Arc<McpConnectionPool>,
    config_path: PathBuf,
) -> Result<()> {
    let (tx, mut rx) = mpsc::channel(100);

    // Spawn blocking file watcher in separate thread
    let watcher_path = config_path.clone();
    std::thread::spawn(move || {
        if let Err(e) = run_file_watcher(watcher_path, tx) {
            eprintln!("⚠️  Config file watcher stopped: {}", e);
        }
    });

    // Handle file change events
    tokio::spawn(async move {
        eprintln!("👀 Watching MCP config file: {}", config_path.display());

        while let Some(event) = rx.recv().await {
            if should_reload(&event) {
                match reload_config(&connection_pool).await {
                    Ok(()) => {
                        // Success message is printed in update_config
                    }
                    Err(e) => {
                        eprintln!("⚠️  Failed to reload MCP config: {}", e);
                    }
                }
            }
        }
    });

    Ok(())
}

fn run_file_watcher(config_path: PathBuf, tx: mpsc::Sender<Event>) -> Result<()> {
    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                let _ = tx.blocking_send(event);
            }
        },
        Config::default().with_poll_interval(Duration::from_secs(1)),
    )?;

    // Watch the directory containing the config file
    // (watching the file directly doesn't work well with editors that use atomic writes)
    let watch_dir = config_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Config file has no parent directory"))?;

    watcher
        .watch(watch_dir, RecursiveMode::NonRecursive)
        .with_context(|| format!("Failed to watch directory: {}", watch_dir.display()))?;

    // Keep watcher alive
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}

fn should_reload(event: &Event) -> bool {
    match &event.kind {
        // File was modified (write, close)
        EventKind::Modify(ModifyKind::Data(_)) => true,
        EventKind::Modify(ModifyKind::Any) => true,
        // File was closed after writing (some editors)
        EventKind::Access(AccessKind::Close(AccessMode::Write)) => true,
        // Atomic write completed (vim, etc.)
        EventKind::Create(_) => event.paths.iter().any(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n == "mcp.json")
                .unwrap_or(false)
        }),
        _ => false,
    }
}

async fn reload_config(connection_pool: &McpConnectionPool) -> Result<()> {
    // Small delay to ensure file write is complete
    tokio::time::sleep(Duration::from_millis(100)).await;

    let config_manager = McpConfigManager::load().context("Failed to load MCP configuration")?;

    let new_config = Arc::new(config_manager.config().clone());

    connection_pool.update_config(new_config).await;

    Ok(())
}
