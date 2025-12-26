mod support {
    include!("../mcp_browse_test_support.rs");
}

use std::collections::HashMap;

use aiw::mcp_routing::config::McpConfigManager;
use aiw::tui::screens::InstalledMcpScreen;
use aiw::tui::Screen;
use anyhow::Result;
use crossterm::event::KeyCode;

use support::*;

// TEST-INT-MCP-EDIT-001
// Covers REQ-020
// Category: Happy path
#[test]
fn edit_env_preloads_modifies_and_saves() -> Result<()> {
    let home = TempHome::new();
    let server_name = fake_server_name();
    let key_a = format!("A_{}", fake_env_key());
    let key_b = format!("Z_{}", fake_env_key());
    let value_a = fake_env_value();
    let value_b = fake_env_value();
    let new_value_a = format!("new_{}", fake_env_value());

    let mut env = HashMap::new();
    env.insert(key_a.clone(), value_a.clone());
    env.insert(key_b.clone(), value_b.clone());

    write_mcp_config(
        &home,
        vec![ServerFixture {
            name: server_name.clone(),
            description: Some(fake_description()),
            enabled: Some(true),
            source: Some("manual".to_string()),
            command: fake_command(),
            args: Vec::new(),
            env,
        }],
    )?;

    let mut screen = InstalledMcpScreen::new()?;
    screen.handle_key(key(KeyCode::Char('e')))?;

    let edit_render = render_installed_screen(&mut screen);
    assert!(edit_render.contains("Edit Env"));
    assert!(edit_render.contains(&value_a));
    assert!(edit_render.contains(&server_name));

    send_backspaces(&mut screen, value_a.chars().count())?;
    send_text(&mut screen, &new_value_a)?;
    screen.handle_key(key(KeyCode::Enter))?;
    screen.handle_key(key(KeyCode::Enter))?;

    let completed_render = render_installed_screen(&mut screen);
    assert!(completed_render.contains("Editing complete"));
    assert!(completed_render.contains("s: save"));
    assert!(!completed_render.contains("Press 'a' to skip optional variables"));

    screen.handle_key(key(KeyCode::Char('s')))?;
    screen.handle_key(key(KeyCode::Char('y')))?;

    let list_render = render_installed_screen(&mut screen);
    assert!(list_render.contains("Saved changes to"));
    assert!(list_render.contains(&server_name));
    assert!(list_render.contains("Installed MCPs"));

    let manager = McpConfigManager::load()?;
    let server = manager
        .config()
        .mcp_servers
        .get(&server_name)
        .expect("server exists");
    assert_eq!(server.env.get(&key_a), Some(&new_value_a));
    assert_eq!(server.env.get(&key_b), Some(&value_b));
    assert_eq!(server.env.len(), 2);

    Ok(())
}

// TEST-INT-MCP-EDIT-002
// Covers REQ-020
// Category: Negative
#[test]
fn edit_env_save_failure_surfaces_error() -> Result<()> {
    let home = TempHome::new();
    let server_name = fake_server_name();
    let env_key = fake_env_key();
    let env_value = fake_env_value();
    let new_value = format!("new_{}", fake_env_value());

    let mut env = HashMap::new();
    env.insert(env_key.clone(), env_value.clone());
    let config_path = write_mcp_config(
        &home,
        vec![ServerFixture {
            name: server_name.clone(),
            description: Some(fake_description()),
            enabled: Some(true),
            source: Some("manual".to_string()),
            command: fake_command(),
            args: Vec::new(),
            env,
        }],
    )?;

    set_read_only(&config_path, true)?;

    let mut screen = InstalledMcpScreen::new()?;
    screen.handle_key(key(KeyCode::Char('e')))?;
    send_backspaces(&mut screen, env_value.chars().count())?;
    send_text(&mut screen, &new_value)?;
    screen.handle_key(key(KeyCode::Enter))?;
    screen.handle_key(key(KeyCode::Char('s')))?;
    screen.handle_key(key(KeyCode::Char('y')))?;

    let error_render = render_installed_screen(&mut screen);
    assert!(error_render.contains("Failed to save"));
    assert!(error_render.contains("Edit Env"));
    assert!(!error_render.contains("Saved changes to"));

    let manager = McpConfigManager::load()?;
    let server = manager
        .config()
        .mcp_servers
        .get(&server_name)
        .expect("server exists");
    assert_eq!(server.env.get(&env_key), Some(&env_value));
    assert_ne!(server.env.get(&env_key), Some(&new_value));
    assert_eq!(server.env.len(), 1);

    set_read_only(&config_path, false)?;

    Ok(())
}

// TEST-INT-MCP-EDIT-003
// Covers REQ-020
// Category: Boundary
#[test]
fn edit_env_with_no_variables_shows_empty_state() -> Result<()> {
    let home = TempHome::new();
    let server_name = fake_server_name();

    write_mcp_config(
        &home,
        vec![ServerFixture {
            name: server_name.clone(),
            description: Some(fake_description()),
            enabled: Some(true),
            source: Some("manual".to_string()),
            command: fake_command(),
            args: Vec::new(),
            env: HashMap::new(),
        }],
    )?;

    let mut screen = InstalledMcpScreen::new()?;
    screen.handle_key(key(KeyCode::Char('e')))?;

    let render = render_installed_screen(&mut screen);
    assert!(render.contains("No environment variables configured."));
    assert!(render.contains("Press Esc to return."));
    assert!(render.contains(&server_name));

    Ok(())
}

// TEST-INT-MCP-EDIT-004
// Covers REQ-020
// Category: Security
#[test]
fn edit_env_persists_literal_security_payload() -> Result<()> {
    let home = TempHome::new();
    let server_name = fake_server_name();
    let env_key = fake_env_key();
    let env_value = fake_env_value();
    let payload = format!("<script>{}</script>;DROP_TABLE", fake_word());

    let mut env = HashMap::new();
    env.insert(env_key.clone(), env_value.clone());

    write_mcp_config(
        &home,
        vec![ServerFixture {
            name: server_name.clone(),
            description: Some(fake_description()),
            enabled: Some(true),
            source: Some("manual".to_string()),
            command: fake_command(),
            args: Vec::new(),
            env,
        }],
    )?;

    let mut screen = InstalledMcpScreen::new()?;
    screen.handle_key(key(KeyCode::Char('e')))?;
    send_backspaces(&mut screen, env_value.chars().count())?;
    send_text(&mut screen, &payload)?;
    screen.handle_key(key(KeyCode::Enter))?;
    screen.handle_key(key(KeyCode::Char('s')))?;
    screen.handle_key(key(KeyCode::Char('y')))?;

    let list_render = render_installed_screen(&mut screen);
    assert!(list_render.contains("Saved changes to"));
    assert!(list_render.contains(&server_name));
    assert!(list_render.contains("Installed MCPs"));

    let manager = McpConfigManager::load()?;
    let server = manager
        .config()
        .mcp_servers
        .get(&server_name)
        .expect("server exists");
    assert_eq!(server.env.get(&env_key), Some(&payload));
    assert_ne!(server.env.get(&env_key), Some(&env_value));
    assert_eq!(server.env.len(), 1);

    Ok(())
}
