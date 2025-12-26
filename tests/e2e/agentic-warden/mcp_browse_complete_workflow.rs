mod support {
    include!("../../mcp_browse_test_support.rs");
}

use std::collections::HashMap;

use aiw::mcp_routing::config::McpConfigManager;
use aiw::tui::screens::InstalledMcpScreen;
use aiw::tui::Screen;
use anyhow::Result;
use crossterm::event::KeyCode;

use support::*;

// TEST-E2E-AIW-001
// Covers REQ-018, REQ-019, REQ-020
// Category: Happy path
#[test]
fn complete_browse_search_edit_skip_save_flow() -> Result<()> {
    let home = TempHome::new();
    let name_a = format!("alpha-{}", fake_word());
    let name_b = format!("beta-{}", fake_word());
    let env_key = fake_env_key();
    let env_value = fake_env_value();
    let mut env = HashMap::new();
    env.insert(env_key.clone(), env_value.clone());

    write_mcp_config(
        &home,
        vec![
            ServerFixture {
                name: name_a.clone(),
                description: Some(fake_description()),
                enabled: Some(true),
                source: Some("local".to_string()),
                command: fake_command(),
                args: Vec::new(),
                env: HashMap::new(),
            },
            ServerFixture {
                name: name_b.clone(),
                description: Some(fake_description()),
                enabled: Some(false),
                source: Some("manual".to_string()),
                command: fake_command(),
                args: Vec::new(),
                env,
            },
        ],
    )?;

    let mut screen = InstalledMcpScreen::new()?;
    let initial_render = render_installed_screen(&mut screen);
    assert!(initial_render.contains("Installed MCPs (2/2)"));
    assert!(initial_render.contains(&name_a));
    assert!(initial_render.contains(&name_b));

    screen.handle_key(key(KeyCode::Char('/')))?;
    send_text(&mut screen, &name_b)?;

    let search_render = render_installed_screen(&mut screen);
    assert!(search_render.contains("Installed MCPs (1/2)"));
    assert!(search_render.contains(&name_b));
    assert!(!search_render.contains(&name_a));

    screen.handle_key(key(KeyCode::Enter))?;
    screen.handle_key(key(KeyCode::Enter))?;

    let details_render = render_installed_screen(&mut screen);
    assert!(details_render.contains("Details"));
    assert!(details_render.contains(&name_b));
    assert!(details_render.contains(&env_key));

    screen.handle_key(key(KeyCode::Esc))?;
    let list_render = render_installed_screen(&mut screen);
    assert!(list_render.contains("Installed MCPs"));
    assert!(list_render.contains(&name_b));
    assert!(!list_render.contains("Details"));

    screen.handle_key(key(KeyCode::Char('e')))?;
    let edit_render = render_installed_screen(&mut screen);
    assert!(edit_render.contains("Edit Env"));
    assert!(edit_render.contains(&env_value));
    assert!(edit_render.contains("Press 'a' to skip optional variables"));

    screen.handle_key(key(KeyCode::Char('a')))?;
    let completed_render = render_installed_screen(&mut screen);
    assert!(completed_render.contains("Editing complete"));
    assert!(completed_render.contains("s: save"));
    assert!(!completed_render.contains("Press 'a' to skip optional variables"));

    screen.handle_key(key(KeyCode::Char('s')))?;
    screen.handle_key(key(KeyCode::Char('y')))?;

    let final_render = render_installed_screen(&mut screen);
    assert!(final_render.contains("Saved changes to"));
    assert!(final_render.contains(&name_b));
    assert!(final_render.contains("Installed MCPs"));

    let manager = McpConfigManager::load()?;
    let server = manager
        .config()
        .mcp_servers
        .get(&name_b)
        .expect("server exists");
    assert!(server.env.is_empty());
    assert_eq!(server.args.len(), 0);
    assert_eq!(server.enabled, Some(false));

    Ok(())
}

// TEST-E2E-AIW-002
// Covers REQ-019, REQ-020
// Category: Security
#[test]
fn search_then_cancel_edit_keeps_original_values() -> Result<()> {
    let home = TempHome::new();
    let server_name = format!("mcp-{}", fake_word());
    let env_key = fake_env_key();
    let env_value = fake_env_value();
    let new_value = format!("new_{}", fake_env_value());

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
    screen.handle_key(key(KeyCode::Char('/')))?;
    send_text(&mut screen, &server_name)?;
    screen.handle_key(key(KeyCode::Enter))?;

    let search_render = render_installed_screen(&mut screen);
    assert!(search_render.contains("Installed MCPs (1/1)"));
    assert!(search_render.contains(&server_name));
    assert!(search_render.contains("Search:"));

    screen.handle_key(key(KeyCode::Char('e')))?;
    send_backspaces(&mut screen, env_value.chars().count())?;
    send_text(&mut screen, &new_value)?;
    screen.handle_key(key(KeyCode::Enter))?;
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
    assert_ne!(server.env.get(&env_key), Some(&new_value));
    assert_eq!(server.env.len(), 1);

    Ok(())
}
