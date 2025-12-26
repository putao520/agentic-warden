mod support {
    include!("../mcp_browse_test_support.rs");
}

use std::collections::HashMap;

use aiw::tui::screens::InstalledMcpScreen;
use aiw::tui::Screen;
use anyhow::Result;
use crossterm::event::KeyCode;

use support::*;

// TEST-INT-MCP-INSTALLED-001
// Covers REQ-019
// Category: Happy path
#[test]
fn installed_list_supports_navigation_and_details() -> Result<()> {
    let home = TempHome::new();
    let name_a = format!("alpha-{}", fake_word());
    let name_b = format!("beta-{}", fake_word());
    let env_key_b = fake_env_key();
    let env_val_b = fake_env_value();

    write_mcp_config(
        &home,
        vec![
            ServerFixture {
                name: name_a.clone(),
                description: Some(fake_description()),
                enabled: Some(true),
                source: Some("local".to_string()),
                command: fake_command(),
                args: vec!["--flag".to_string()],
                env: HashMap::new(),
            },
            ServerFixture {
                name: name_b.clone(),
                description: Some(fake_description()),
                enabled: Some(false),
                source: Some("manual".to_string()),
                command: fake_command(),
                args: Vec::new(),
                env: HashMap::from([(env_key_b.clone(), env_val_b.clone())]),
            },
        ],
    )?;

    let mut screen = InstalledMcpScreen::new()?;
    let list_render = render_installed_screen(&mut screen);
    assert!(list_render.contains("Installed MCPs (2/2)"));
    assert!(list_render.contains(&name_a));
    assert!(list_render.contains(&name_b));
    assert!(list_render.contains("ENABLED"));
    assert!(list_render.contains("DISABLED"));

    screen.handle_key(key(KeyCode::Down))?;
    screen.handle_key(key(KeyCode::Enter))?;
    let details_render = render_installed_screen(&mut screen);
    assert!(details_render.contains("Details"));
    assert!(details_render.contains(&name_b));
    assert!(details_render.contains("Status: Disabled"));
    assert!(details_render.contains(&env_key_b));

    screen.handle_key(key(KeyCode::Esc))?;
    let back_render = render_installed_screen(&mut screen);
    assert!(back_render.contains("Installed MCPs"));
    assert!(back_render.contains(&name_a));
    assert!(back_render.contains(&name_b));

    Ok(())
}

// TEST-INT-MCP-INSTALLED-002
// Covers REQ-019
// Category: Negative
#[test]
fn search_filters_to_empty_results_for_unknown_query() -> Result<()> {
    let home = TempHome::new();
    let name_a = format!("alpha-{}", fake_word());
    let name_b = format!("beta-{}", fake_word());
    let query = format!("nope-{}", fake_word());

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
                enabled: Some(true),
                source: Some("manual".to_string()),
                command: fake_command(),
                args: Vec::new(),
                env: HashMap::new(),
            },
        ],
    )?;

    let mut screen = InstalledMcpScreen::new()?;
    screen.handle_key(key(KeyCode::Char('/')))?;
    send_text(&mut screen, &query)?;

    let search_render = render_installed_screen(&mut screen);
    assert!(search_render.contains("Installed MCPs (0/2)"));
    assert!(!search_render.contains(&name_a));
    assert!(!search_render.contains(&name_b));
    assert!(search_render.contains(&query));

    Ok(())
}

// TEST-INT-MCP-INSTALLED-003
// Covers REQ-019
// Category: Boundary
#[test]
fn empty_config_shows_no_installed_message() -> Result<()> {
    let _home = TempHome::new();

    let mut screen = InstalledMcpScreen::new()?;
    let render = render_installed_screen(&mut screen);
    assert!(render.contains("Installed MCPs (0/0)"));
    assert!(render.contains("No installed MCP servers found"));
    assert!(render.contains("Installed MCPs"));

    Ok(())
}

// TEST-INT-MCP-INSTALLED-004
// Covers REQ-019
// Category: Security
#[test]
fn search_input_with_script_payload_is_treated_as_literal() -> Result<()> {
    let home = TempHome::new();
    let name = format!("mcp-{}", fake_word());
    let payload = format!("<script>{}</script>", fake_word());

    write_mcp_config(
        &home,
        vec![ServerFixture {
            name: name.clone(),
            description: Some(fake_description()),
            enabled: Some(true),
            source: Some("manual".to_string()),
            command: fake_command(),
            args: Vec::new(),
            env: HashMap::new(),
        }],
    )?;

    let mut screen = InstalledMcpScreen::new()?;
    screen.handle_key(key(KeyCode::Char('/')))?;
    send_text(&mut screen, &payload)?;

    let render = render_installed_screen(&mut screen);
    assert!(render.contains("Installed MCPs (0/1)"));
    assert!(!render.contains(&name));
    assert!(render.contains(&payload));

    Ok(())
}
