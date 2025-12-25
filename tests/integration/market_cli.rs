use aiw::commands::market::config::ConfigStore;
use aiw::commands::market::plugin::{
    MarketplaceConfig, MarketplaceMetadata, MarketplaceOwner, MarketplacePluginEntry, PluginAuthor,
    PluginManifest, PluginSource,
};
use aiw::commands::market::handle_plugin_action;
use aiw::commands::parser::{MarketplaceAction, PluginAction};
use std::fs;
use tempfile::TempDir;

fn write_marketplace(root: &std::path::Path) {
    let marketplace = MarketplaceConfig {
        name: "local-market".to_string(),
        owner: MarketplaceOwner {
            name: "Maintainer".to_string(),
            email: None,
        },
        metadata: Some(MarketplaceMetadata {
            description: Some("Test market".to_string()),
            version: Some("1.0.0".to_string()),
            plugin_root: Some("./plugins".to_string()),
        }),
        plugins: vec![MarketplacePluginEntry {
            name: "demo-plugin".to_string(),
            source: PluginSource::Path("./plugins/demo-plugin".to_string()),
            description: Some("Demo".to_string()),
            version: Some("0.1.0".to_string()),
            author: Some(PluginAuthor {
                name: "Tester".to_string(),
                email: None,
            }),
            category: Some("development".to_string()),
            tags: Some(vec!["mcp".to_string()]),
            strict: Some(false),
        }],
    };
    let marketplace_path = root.join(".claude-plugin");
    fs::create_dir_all(&marketplace_path).unwrap();
    fs::write(
        marketplace_path.join("marketplace.json"),
        serde_json::to_string_pretty(&marketplace).unwrap(),
    )
    .unwrap();

    let plugin_root = root.join("plugins").join("demo-plugin");
    fs::create_dir_all(plugin_root.join(".claude-plugin")).unwrap();
    let manifest = PluginManifest {
        name: "demo-plugin".to_string(),
        version: "0.1.0".to_string(),
        description: "Demo plugin".to_string(),
        author: PluginAuthor {
            name: "Tester".to_string(),
            email: None,
        },
        homepage: None,
        repository: None,
        license: None,
        keywords: None,
        mcp_servers: Some(serde_json::json!({
            "mcpServers": {
                "demo": {
                    "command": "npx",
                    "args": ["-y", "@demo/server"],
                    "env": {"TOKEN": "${TOKEN}"}
                }
            }
        })),
        commands: None,
        agents: None,
        hooks: None,
    };
    fs::write(
        plugin_root.join(".claude-plugin").join("plugin.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
}

#[tokio::test]
async fn marketplace_cli_flow() {
    let temp = TempDir::new().unwrap();
    std::env::set_var("HOME", temp.path());
    std::env::set_var("USERPROFILE", temp.path());

    let marketplace_dir = temp.path().join("marketplace");
    fs::create_dir_all(&marketplace_dir).unwrap();
    write_marketplace(&marketplace_dir);

    handle_plugin_action(PluginAction::Marketplace(MarketplaceAction::Add {
        repo_url: marketplace_dir.to_string_lossy().to_string(),
        name: Some("local".to_string()),
    }))
    .await
    .unwrap();

    handle_plugin_action(PluginAction::Search {
        query: "demo".to_string(),
        market: Some("local".to_string()),
    })
    .await
    .unwrap();

    handle_plugin_action(PluginAction::Install {
        plugin: "demo-plugin@local".to_string(),
        env_vars: Vec::new(),
        skip_env: true,
    })
    .await
    .unwrap();

    let store = ConfigStore::new().unwrap();
    let plugins = store.load_plugins().unwrap();
    assert!(plugins.plugins.contains_key("demo-plugin@local"));

    let mcp = store.load_mcp().unwrap();
    assert!(mcp.mcp_servers.contains_key("demo"));

    handle_plugin_action(PluginAction::Remove {
        plugin: "demo-plugin".to_string(),
    })
    .await
    .unwrap();

    let plugins = store.load_plugins().unwrap();
    assert!(!plugins.plugins.contains_key("demo-plugin@local"));
}
