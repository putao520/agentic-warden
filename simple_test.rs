#!/usr/bin/env rust-script

use std::env;

fn main() {
    println!("=== Simple OAuth Debug Test ===");

    // Check environment variables first
    let client_id = env::var("AIW_OAUTH_CLIENT_ID").unwrap_or_else(|_| "NONE".to_string());
    let client_secret = env::var("AIW_OAUTH_CLIENT_SECRET").unwrap_or_else(|_| "NONE".to_string());

    println!("Environment variables:");
    println!("  AIW_OAUTH_CLIENT_ID: {}", if client_id != "NONE" { "***" } else { "NOT_SET" });
    println!("  AIW_OAUTH_CLIENT_SECRET: {}", if client_secret != "NONE" { "***" } else { "NOT_SET" });

    // Check OAuthConfig::default()
    println!("\n=== Testing OAuthConfig::default() ===");
    match aiw::sync::oauth_client::OAuthConfig::default() {
        Ok(config) => {
            println!("✅ OAuthConfig::default() succeeded");
            println!("  Client ID: {}", config.client_id);
            println!("  Client Secret: {}", if config.client_secret.len() > 0 { "***" } else { "EMPTY" });
            println!("  Has valid credentials: {}", config.has_valid_credentials());
        }
        Err(e) => {
            println!("❌ OAuthConfig::default() failed: {}", e);
        }
    }

    println!("\n=== Test Complete ===");
}