#!/usr/bin/env rust-script

//! Simple test to isolate OAuth initialization issue

use std::env;

fn main() {
    println!("=== OAuth Debug Test ===");

    // Check environment variables first
    let client_id = env::var("AIW_OAUTH_CLIENT_ID").unwrap_or_else(|_| "NONE".to_string());
    let client_secret = env::var("AIW_OAUTH_CLIENT_SECRET").unwrap_or_else(|_| "NONE".to_string());

    println!("Environment variables:");
    println!("  AIW_OAUTH_CLIENT_ID: {}", if client_id != "NONE" { "***" } else { "NOT_SET" });
    println!("  AIW_OAUTH_CLIENT_SECRET: {}", if client_id != "NONE" { "***" } else { "NOT_SET" });

    // Load the OAuth config directly
    println!("\n=== Testing OAuthConfig Creation ===");

    // This mimics what happens in the OAuth code
    let config = aiw::sync::oauth_client::OAuthConfig::default();
    println!("Client ID: {}", config.client_id);
    println!("Client Secret: {}", if config.client_secret.len() > 0 { "***" } else { "EMPTY" });
    println!("Public client: {}", config.is_public_client());

    if config.is_public_client() {
        println!("❌ This is the invalid public client that's causing issues!");
        println!("💡 The fix is to set custom OAuth credentials:");
        println!("   export AIW_OAUTH_CLIENT_ID=your_client_id");
        println!("   export AIW_OAUTH_CLIENT_SECRET=your_client_secret");
    }

    println!("\n=== Test Complete ===");
}