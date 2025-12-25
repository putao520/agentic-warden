//! CLI entry for plugin marketplace commands.

use crate::commands::market::cli_marketplace::handle_marketplace_action;
use crate::commands::market::cli_plugins::{
    browse_plugins, list_installed, plugin_info, plugin_install, remove_plugin, search_plugins,
    set_plugin_enabled,
};
use crate::commands::market::source::MarketError;
use crate::commands::parser::PluginAction;
use std::process::ExitCode;

pub async fn handle_plugin_action(action: PluginAction) -> Result<ExitCode, String> {
    let result = match action {
        PluginAction::Marketplace(action) => handle_marketplace_action(action).await,
        PluginAction::Browse { market, category, tags } => {
            browse_plugins(market, category, tags).await
        }
        PluginAction::Search { query, market } => search_plugins(query, market).await,
        PluginAction::Info { plugin } => plugin_info(plugin).await,
        PluginAction::Install {
            plugin,
            env_vars,
            skip_env,
        } => plugin_install(plugin, env_vars, skip_env).await,
        PluginAction::List { show_disabled } => list_installed(show_disabled).await,
        PluginAction::Remove { plugin } => remove_plugin(plugin).await,
        PluginAction::Enable { plugin } => set_plugin_enabled(plugin, true).await,
        PluginAction::Disable { plugin } => set_plugin_enabled(plugin, false).await,
    };

    result.map(|_| ExitCode::from(0)).map_err(format_error)
}

fn format_error(err: MarketError) -> String {
    if let Some(ref source) = err.source {
        format!("{} ({})", err, source)
    } else {
        err.to_string()
    }
}
