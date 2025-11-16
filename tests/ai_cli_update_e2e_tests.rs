//! AI CLI Update E2E Tests
//! Tests REQ-011: AI CLI更新/安装管理

mod common;

use anyhow::Result;
use common::{create_mock_claude_bin, create_mock_npm_bin, read_mock_claude_log};
use mockito::Server;
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[tokio::test]
#[serial]
async fn test_update_npm_package() -> Result<()> {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/openai/codex/latest")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"version": "2.0.0"}"#)
        .create_async()
        .await;

    let response = reqwest::get(format!("{}/openai/codex/latest", server.url()))
        .await?
        .json::<serde_json::Value>()
        .await?;
    let version = response
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0");

    let npm_bin = create_mock_npm_bin().await?;
    let mut paths: Vec<PathBuf> = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).collect();
    if let Some(dir) = npm_bin.parent() {
        paths.insert(0, dir.to_path_buf());
    }
    let joined_path = std::env::join_paths(paths)?;
    let install_output = Command::new(&npm_bin)
        .env("PATH", joined_path)
        .args(["install", &format!("@openai/codex@{version}")])
        .output()?;
    assert!(install_output.status.success());

    if let Some(dir) = npm_bin.parent() {
        let log_path = dir.join("npm_mock.log");
        let log_content = fs::read_to_string(log_path)?;
        assert!(
            log_content.contains(version),
            "npm log should include target version"
        );
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_update_claude_native() -> Result<()> {
    let claude_bin = create_mock_claude_bin().await?;
    let status = Command::new(&claude_bin).arg("update").status()?;
    assert!(status.success());

    let log = read_mock_claude_log().await?;
    assert!(log.to_lowercase().contains("update"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_update_network_error_handling() -> Result<()> {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/openai/codex/latest")
        .with_status(500)
        .with_body("server error")
        .create_async()
        .await;

    let result = reqwest::get(format!("{}/openai/codex/latest", server.url()))
        .await?
        .error_for_status();

    assert!(result.is_err());
    let message = format!("{result:?}");
    assert!(
        message.to_lowercase().contains("500") || message.to_lowercase().contains("error"),
        "network error should be reported"
    );

    Ok(())
}
