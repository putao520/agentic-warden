use aiw::commands::market::filter::McpFilter;
use aiw::commands::market::github_source::GithubSource;
use aiw::commands::market::config::ConfigStore;
use aiw::commands::market::local_source::LocalSource;
use aiw::commands::market::plugin::{
    MarketplaceConfig, MarketplaceMetadata, MarketplaceOwner, MarketplacePluginEntry, PluginAuthor,
    PluginManifest, PluginMetadata, PluginSource,
};
use aiw::commands::market::plugin_io::extract_mcp_config;
use aiw::commands::market::remote_source::RemoteSource;
use aiw::commands::market::source::MarketSource;
use aiw::commands::market::cache::MarketCacheManager;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn write_marketplace(root: &Path) -> MarketplaceConfig {
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
    marketplace
}

fn write_plugin(root: &Path) {
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

#[test]
fn filters_mcp_plugins() {
    let metadata = PluginMetadata {
        name: "demo".to_string(),
        version: "0.1.0".to_string(),
        description: "demo".to_string(),
        author: PluginAuthor {
            name: "Tester".to_string(),
            email: None,
        },
        marketplace: "local".to_string(),
        source: PluginSource::Path("./demo".to_string()),
        has_mcp_servers: true,
        mcp_servers: Vec::new(),
        category: None,
        tags: vec![],
    };
    assert!(PluginMetadata::is_mcp_plugin(&metadata));
}

#[test]
fn extracts_inline_mcp_config() {
    let manifest = PluginManifest {
        name: "demo".to_string(),
        version: "0.1.0".to_string(),
        description: "demo".to_string(),
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
                    "env": {}
                }
            }
        })),
        commands: None,
        agents: None,
        hooks: None,
    };
    let extracted = extract_mcp_config(&manifest, Path::new(".")).unwrap();
    let config = extracted.unwrap();
    assert!(config.mcp_servers.contains_key("demo"));
}

#[tokio::test]
async fn local_source_reads_marketplace() {
    let temp = TempDir::new().unwrap();
    write_marketplace(temp.path());
    write_plugin(temp.path());
    let cache = MarketCacheManager::new().unwrap();
    let source = LocalSource::new("local".to_string(), temp.path().to_path_buf(), cache).unwrap();
    let marketplace = source.fetch_marketplace().await.unwrap();
    assert_eq!(marketplace.name, "local-market");
    let entry = marketplace.plugins.first().unwrap().clone();
    let manifest = source.fetch_plugin(&entry).await.unwrap();
    assert_eq!(manifest.name, "demo-plugin");
}

#[tokio::test]
async fn github_source_clones_repository() {
    let temp = TempDir::new().unwrap();
    let repo_dir = temp.path().join("repo");
    fs::create_dir_all(&repo_dir).unwrap();
    let repo = git2::Repository::init(&repo_dir).unwrap();
    write_marketplace(&repo_dir);
    write_plugin(&repo_dir);

    let mut index = repo.index().unwrap();
    index.add_all(["."].iter(), git2::IndexAddOption::DEFAULT, None).unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = git2::Signature::now("test", "test@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[]).unwrap();

    let cache = MarketCacheManager::new().unwrap();
    let source = GithubSource::new(
        "file-market".to_string(),
        format!("file://{}", repo_dir.to_string_lossy()),
        cache,
    );
    let marketplace = source.fetch_marketplace().await.unwrap();
    assert_eq!(marketplace.plugins.len(), 1);
}

#[tokio::test]
async fn remote_source_fetches_manifest() {
    let temp = TempDir::new().unwrap();
    let _marketplace = write_marketplace(temp.path());
    write_plugin(temp.path());

    let marketplace_body = fs::read_to_string(temp.path().join(".claude-plugin").join("marketplace.json")).unwrap();
    let plugin_body = fs::read_to_string(
        temp
            .path()
            .join("plugins")
            .join("demo-plugin")
            .join(".claude-plugin")
            .join("plugin.json"),
    )
    .unwrap();

    let mut server = mockito::Server::new_async().await;
    let base = server.url();
    let _marketplace_mock = server
        .mock("GET", "/marketplace.json")
        .with_body(marketplace_body)
        .create_async()
        .await;
    let _plugin_mock = server
        .mock("GET", "/plugins/demo-plugin/.claude-plugin/plugin.json")
        .with_body(plugin_body)
        .create_async()
        .await;

    let cache = MarketCacheManager::new().unwrap();
    let source = RemoteSource::new(
        "remote".to_string(),
        format!("{}/marketplace.json", base),
        cache,
    )
    .unwrap();
    let marketplace = source.fetch_marketplace().await.unwrap();
    let entry = marketplace.plugins.first().unwrap().clone();
    let manifest = source.fetch_plugin(&entry).await.unwrap();
    assert_eq!(manifest.name, "demo-plugin");
}

#[test]
fn migrates_legacy_yaml() {
    let temp = TempDir::new().unwrap();
    std::env::set_var("HOME", temp.path());
    let config_dir = temp.path().join(".aiw");
    fs::create_dir_all(&config_dir).unwrap();
    let legacy = r#"
    mcpServers:
      demo:
        source: registry
        command: npx
        args: ["-y", "@demo/server"]
        env:
          TOKEN: "${TOKEN}"
    "#;
    fs::write(config_dir.join("mcp_servers.yaml"), legacy).unwrap();

    let store = ConfigStore::new().unwrap();
    store.migrate_legacy_configs().unwrap();

    let mcp_path = config_dir.join("mcp.json");
    assert!(mcp_path.exists());
    let mcp_contents = fs::read_to_string(mcp_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&mcp_contents).unwrap();
    assert!(parsed["mcpServers"]["demo"].is_object());

    let backups: Vec<_> = fs::read_dir(&config_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("mcp_servers.yaml.bak"))
        .collect();
    assert!(!backups.is_empty());
}
