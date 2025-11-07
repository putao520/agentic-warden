use agentic_warden::{TaskRegistry, supervisor};
use agentic_warden::cli_type::CliType;
use std::ffi::OsString;
use tempfile::TempDir;
use std::fs;
use serde_json::json;

fn main() {
    let dir = TempDir::new().expect("temp dir");
    let config_dir = dir.path().join(".agentic-warden");
    fs::create_dir_all(&config_dir).expect("provider dir");
    let config_file = config_dir.join("providers.json");
    
    let config = json!({
        "providers": {
            "valid": {
                "name": "Valid Provider",
                "description": "A valid provider",
                "icon": "✅",
                "website": None,
                "regions": vec!["international"],
                "env": {"OPENAI_API_KEY": "demo-key"}
            }
        },
        "default_provider": "ghost",
        "user_tokens": {}
    });
    
    fs::write(&config_file, serde_json::to_string_pretty(&config).expect("serialize config"))
        .expect("write providers config");
    
    std::env::set_var("HOME", dir.path());
    std::env::set_var("USERPROFILE", dir.path());
    
    println!("Config file: {:?}", config_file);
    println!("Config content: {}", serde_json::to_string_pretty(&config).unwrap());
    
    let registry = TaskRegistry::connect().expect("task registry should connect");
    let args = vec![OsString::from("--version")];
    
    println!("Calling supervisor::execute_cli...");
    let result = supervisor::execute_cli(&registry, &CliType::Codex, &args, None);
    println!("Result: {:?}", result);
}
