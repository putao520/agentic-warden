mod support {
    include!("../mcp_browse_test_support.rs");
}

use std::collections::HashMap;

use aiw::mcp_routing::config::McpConfigManager;
use aiw::tui::{Screen, ScreenAction};
use anyhow::Result;
use crossterm::event::KeyCode;

use support::*;

// TEST-INT-MCP-BROWSE-001
// Covers REQ-018
// Category: Happy path
#[test]
fn skip_optional_shortcut_clears_values_and_prompts_save() -> Result<()> {
    let home = TempHome::new();
    let server_name = fake_server_name();
    let env_key = fake_env_key();
    let env_value = fake_env_value();
    let command = fake_command();
    let mut env = HashMap::new();
    env.insert(env_key.clone(), env_value.clone());

    write_mcp_config(
        &home,
        vec![ServerFixture {
            name: server_name.clone(),
            description: Some(fake_description()),
            enabled: Some(true),
            source: Some("manual".to_string()),
            command: command.clone(),
            args: Vec::new(),
            env,
        }],
    )?;

    let mut screen = aiw::tui::screens::InstalledMcpScreen::new()?;
    let list_render = render_installed_screen(&mut screen);
    assert!(list_render.contains("Installed MCPs"));
    assert!(list_render.contains(&server_name));
    assert!(list_render.contains("env: 1"));

    screen.handle_key(key(KeyCode::Char('e')))?;
    let edit_render = render_installed_screen(&mut screen);
    assert!(edit_render.contains("Edit Env"));
    assert!(edit_render.contains(&env_value));
    assert!(edit_render.contains("Press 'a' to skip optional variables"));

    screen.handle_key(key(KeyCode::Char('a')))?;
    let skipped_render = render_installed_screen(&mut screen);
    assert!(skipped_render.contains("Editing complete"));
    assert!(skipped_render.contains("s: save"));
    assert!(!skipped_render.contains("Press 'a' to skip optional variables"));

    screen.handle_key(key(KeyCode::Char('s')))?;
    let confirm_render = render_installed_screen(&mut screen);
    assert!(confirm_render.contains("Confirm Save"));
    assert!(confirm_render.contains(&server_name));
    assert!(confirm_render.contains("Save changes"));

    screen.handle_key(key(KeyCode::Char('y')))?;
    let final_render = render_installed_screen(&mut screen);
    assert!(final_render.contains("Saved changes to"));
    assert!(final_render.contains(&server_name));
    assert!(final_render.contains("Installed MCPs"));

    let manager = McpConfigManager::load()?;
    let server = manager
        .config()
        .mcp_servers
        .get(&server_name)
        .expect("server exists");
    assert!(server.env.is_empty());
    assert_eq!(server.command, command);
    assert!(server.args.is_empty());

    Ok(())
}

// TEST-INT-MCP-BROWSE-002
// Covers REQ-018
// Category: Security
#[test]
fn skip_optional_does_not_persist_without_confirmation() -> Result<()> {
    let home = TempHome::new();
    let server_name = fake_server_name();
    let env_key = fake_env_key();
    let env_value = fake_env_value();
    let command = fake_command();
    let mut env = HashMap::new();
    env.insert(env_key.clone(), env_value.clone());

    write_mcp_config(
        &home,
        vec![ServerFixture {
            name: server_name.clone(),
            description: Some(fake_description()),
            enabled: Some(true),
            source: Some("manual".to_string()),
            command: command.clone(),
            args: Vec::new(),
            env,
        }],
    )?;

    let mut screen = aiw::tui::screens::InstalledMcpScreen::new()?;
    screen.handle_key(key(KeyCode::Char('e')))?;
    screen.handle_key(key(KeyCode::Char('a')))?;
    screen.handle_key(key(KeyCode::Esc))?;

    let list_render = render_installed_screen(&mut screen);
    assert!(list_render.contains("Installed MCPs"));
    assert!(list_render.contains(&server_name));
    assert!(!list_render.contains("Saved changes"));

    let manager = McpConfigManager::load()?;
    let server = manager
        .config()
        .mcp_servers
        .get(&server_name)
        .expect("server exists");
    assert_eq!(server.env.get(&env_key), Some(&env_value));
    assert_eq!(server.env.len(), 1);
    assert_eq!(server.command, command);

    Ok(())
}

// TEST-INT-MCP-BROWSE-003
// Covers REQ-018
// Category: Negative
#[test]
fn skip_optional_noop_outside_edit_mode() -> Result<()> {
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

    let mut screen = aiw::tui::screens::InstalledMcpScreen::new()?;
    let action = screen.handle_key(key(KeyCode::Char('a')))?;
    assert!(matches!(action, ScreenAction::None));

    let list_render = render_installed_screen(&mut screen);
    assert!(list_render.contains("Installed MCPs"));
    assert!(list_render.contains(&server_name));
    assert!(!list_render.contains("Edit Env"));

    Ok(())
}

// TEST-INT-MCP-BROWSE-004
// Covers REQ-018
// Category: Boundary
#[test]
fn skip_optional_handles_empty_env_list() -> Result<()> {
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

    let mut screen = aiw::tui::screens::InstalledMcpScreen::new()?;
    screen.handle_key(key(KeyCode::Char('e')))?;

    let edit_render = render_installed_screen(&mut screen);
    assert!(edit_render.contains("No environment variables configured."));
    assert!(edit_render.contains("Press Esc to return."));
    assert!(!edit_render.contains("Press 'a' to skip optional variables"));

    Ok(())
}
