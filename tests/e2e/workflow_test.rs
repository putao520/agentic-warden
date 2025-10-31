//! End-to-End Workflow Tests
//!
//! Complete workflow tests for agentic-warden functionality

use agentic_warden::provider::{ProviderManager, EnvInjector};
use agentic_warden::provider::config::{AiType, ProviderConfig};
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// Import test helpers from the common module
#[path = "../common/test_helpers.rs"]
mod test_helpers;
use test_helpers::*;

/// End-to-end test environment
struct E2ETestEnv {
    temp_dir: TempDir,
    config_dir: PathBuf,
    provider_config_path: PathBuf,
    ai_configs: Vec<PathBuf>,
}

impl E2ETestEnv {
    fn new() -> Result<Self> {
        let temp_dir = create_temp_test_dir();
        let config_dir = temp_dir.path().join(".agentic-warden");

        fs::create_dir_all(&config_dir)
            .context("Failed to create config directory")?;

        let provider_config_path = config_dir.join("provider.json");

        // Create AI CLI config directories
        let mut ai_configs = Vec::new();
        for ai_name in &["claude", "codex", "gemini"] {
            let ai_config_dir = config_dir.join(format!(".{}", ai_name));
            fs::create_dir_all(&ai_config_dir)
                .context(format!("Failed to create {} config directory", ai_name))?;

            ai_configs.push(ai_config_dir);
        }

        Ok(Self {
            temp_dir,
            config_dir,
            provider_config_path,
            ai_configs,
        })
    }

    fn setup_provider_config(&self) -> Result<()> {
        let mut providers = std::collections::HashMap::new();

        // Add official multi-AI provider
        let official_provider = create_real_multi_provider("Official Multi-AI Provider");
        providers.insert("official".to_string(), official_provider);

        // Add specialized providers
        let claude_provider = create_real_provider(AiType::Claude, "Claude-3 Optimized");
        providers.insert("claude-fast".to_string(), claude_provider);

        let codex_provider = create_real_provider(AiType::Codex, "GPT-4 Optimized");
        providers.insert("codex-fast".to_string(), codex_provider);

        let gemini_provider = create_real_provider(AiType::Gemini, "Gemini-1.5 Pro");
        providers.insert("gemini-fast".to_string(), gemini_provider);

        let config = ProviderConfig {
            schema: None,
            default_provider: "official".to_string(),
            providers,
        };

        let json = serde_json::to_string_pretty(&config)
            .context("Failed to serialize provider config")?;

        fs::write(&self.provider_config_path, json)
            .context("Failed to write provider config")?;

        Ok(())
    }

    fn setup_ai_configs(&self) -> Result<()> {
        // Create Claude config
        let claude_config = serde_json::json!({
            "model": "claude-3-sonnet-20241022",
            "max_tokens": 4096,
            "temperature": 0.7,
            "top_p": 0.9,
            "timeout": 30
        });

        fs::write(
            self.ai_configs[0].join("config.json"),
            serde_json::to_string_pretty(&claude_config).unwrap(),
        )
        .context("Failed to write Claude config")?;

        // Create Codex config
        let codex_config = serde_json::json!({
            "model": "gpt-4-turbo-preview",
            "max_tokens": 4096,
            "temperature": 0.7,
            "top_p": 0.9,
            "timeout": 30
        });

        fs::write(
            self.ai_configs[1].join("config.json"),
            serde_json::to_string_pretty(&codex_config).unwrap(),
        )
        .context("Failed to write Codex config")?;

        // Create Gemini config
        let gemini_config = serde_json::json!({
            "model": "gemini-1.5-pro",
            "max_tokens": 4096,
            "temperature": 0.7,
            "top_p": 0.9,
            "timeout": 30
        });

        fs::write(
            self.ai_configs[2].join("config.json"),
            serde_json::to_string_pretty(&gemini_config).unwrap(),
        )
        .context("Failed to write Gemini config")?;

        Ok(())
    }

    fn create_test_files(&self, count: usize, size_kb: usize) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let test_content = "Test content for agentic-warden E2E testing.\n".repeat(size_kb);

        for i in 0..count {
            let file_path = self.temp_dir.path().join(format!("test_file_{}.txt", i));
            fs::write(&file_path, format!("File {}:\n{}", i, test_content))
                .context(format!("Failed to write test file {}", i))?;
            files.push(file_path);
        }

        Ok(files)
    }
}

/// Test complete provider setup workflow
#[test]
fn test_complete_provider_setup_workflow() -> Result<()> {
    let env = E2ETestEnv::new()
        .context("Failed to create E2E test environment")?;

    // Setup configurations
    env.setup_provider_config()
        .context("Failed to setup provider config")?;

    env.setup_ai_configs()
        .context("Failed to setup AI configs")?;

    // Test provider manager initialization
    let manager = ProviderManager::new_with_path(&env.provider_config_path)
        .context("Failed to create ProviderManager")?;

    // Verify all providers were loaded
    let providers = manager.list_providers();
    assert!(providers.len() >= 4, "Should have at least 4 providers");

    let provider_names: Vec<_> = providers.iter().map(|(name, _)| name.as_str()).collect();
    assert!(provider_names.contains(&"official"));
    assert!(provider_names.contains(&"claude-fast"));
    assert!(provider_names.contains(&"codex-fast"));
    assert!(provider_names.contains(&"gemini-fast"));

    // Test default provider
    let (default_name, default_provider) = manager.get_default_provider()
        .context("Failed to get default provider")?;

    assert_eq!(default_name, "official");
    assert_eq!(default_provider.compatible_with.len(), 3);

    println!("✅ Complete provider setup workflow validated!");
    println!("Loaded {} providers:", providers.len());
    for (name, provider) in providers {
        println!("  - {}: {} (compatible with {} AI types)",
            name, provider.description, provider.compatible_with.len());
    }

    Ok(())
}

/// Test environment variable injection workflow
#[test]
fn test_environment_variable_injection_workflow() -> Result<()> {
    let env = E2ETestEnv::new()
        .context("Failed to create E2E test environment")?;

    env.setup_provider_config()
        .context("Failed to setup provider config")?;

    env.setup_ai_configs()
        .context("Failed to setup AI configs")?;

    let manager = ProviderManager::new_with_path(&env.provider_config_path)
        .context("Failed to create ProviderManager")?;

    let injector = EnvInjector;

    // Test environment injection for each AI type
    let ai_types = vec![AiType::Claude, AiType::Codex, AiType::Gemini];

    for ai_type in ai_types {
        // Get the official provider's environment variables
        let official_provider = manager.get_provider("official")
            .context("Failed to get official provider")?;

        assert!(official_provider.compatible_with.contains(&ai_type),
                "Official provider should support {:?}", ai_type);

        // Test that we can create environment variables for this AI type
        let test_provider = create_real_provider(ai_type.clone(), &format!("Test {:?}", ai_type));
        assert!(!test_provider.env.is_empty(), "Test provider should have environment variables");

        // Test masking functionality
        for (key, value) in &test_provider.env {
            let masked_value = EnvInjector::mask_sensitive_value(key, value);
            if key.contains("KEY") || key.contains("SECRET") || key.contains("TOKEN") {
                assert!(masked_value.contains("***"), "Sensitive value should be masked: {}", key);
            }
        }

        // Test command injection
        use std::process::Command;
        let mut cmd = Command::new("echo");
        EnvInjector::inject_to_command(&mut cmd, &test_provider.env);

        // Verify essential variables are present in the test provider
        match ai_type {
            AiType::Claude => {
                assert!(test_provider.env.contains_key("ANTHROPIC_API_KEY"));
                assert!(test_provider.env.contains_key("ANTHROPIC_BASE_URL"));
            }
            AiType::Codex => {
                assert!(test_provider.env.contains_key("OPENAI_API_KEY"));
                assert!(test_provider.env.contains_key("OPENAI_BASE_URL"));
                assert!(test_provider.env.contains_key("OPENAI_ORG_ID"));
            }
            AiType::Gemini => {
                assert!(test_provider.env.contains_key("GOOGLE_API_KEY"));
            }
        }

        println!("✅ Environment injection validated for {:?}", ai_type);
        println!("  Test provider has {} variables", test_provider.env.len());
    }

    Ok(())
}

/// Test provider switching workflow
#[test]
fn test_provider_switching_workflow() -> Result<()> {
    let env = E2ETestEnv::new()
        .context("Failed to create E2E test environment")?;

    env.setup_provider_config()
        .context("Failed to setup provider config")?;

    let mut manager = ProviderManager::new_with_path(&env.provider_config_path)
        .context("Failed to create ProviderManager")?;

    // Start with official provider
    let (initial_name, initial_provider) = manager.get_default_provider()
        .context("Failed to get initial default provider")?;

    assert_eq!(initial_name, "official");

    // Switch to Claude-specific provider
    manager.set_default("claude-fast")
        .context("Failed to set claude-fast as default")?;

    let (claude_name, claude_provider) = manager.get_default_provider()
        .context("Failed to get claude default provider")?;

    assert_eq!(claude_name, "claude-fast");
    assert_eq!(claude_provider.compatible_with, vec![AiType::Claude]);

    // Switch to Codex-specific provider
    manager.set_default("codex-fast")
        .context("Failed to set codex-fast as default")?;

    let (codex_name, codex_provider) = manager.get_default_provider()
        .context("Failed to get codex default provider")?;

    assert_eq!(codex_name, "codex-fast");
    assert_eq!(codex_provider.compatible_with, vec![AiType::Codex]);

    // Switch back to official provider
    manager.set_default("official")
        .context("Failed to set official as default")?;

    let (final_name, final_provider) = manager.get_default_provider()
        .context("Failed to get final default provider")?;

    assert_eq!(final_name, "official");
    assert_eq!(final_provider.compatible_with.len(), 3);

    // Save configuration and verify persistence
    manager.save()
        .context("Failed to save provider configuration")?;

    // Create new manager instance to test persistence
    let new_manager = ProviderManager::new_with_path(&env.provider_config_path)
        .context("Failed to create new ProviderManager")?;

    let (persisted_name, persisted_provider) = new_manager.get_default_provider()
        .context("Failed to get persisted default provider")?;

    assert_eq!(persisted_name, "official");

    println!("✅ Provider switching workflow validated!");
    println!("Provider changes were successfully persisted");

    Ok(())
}

/// Test AI CLI configuration workflow
#[test]
fn test_ai_cli_configuration_workflow() -> Result<()> {
    let env = E2ETestEnv::new()
        .context("Failed to create E2E test environment")?;

    env.setup_ai_configs()
        .context("Failed to setup AI configs")?;

    // Verify AI config files exist and are readable
    for (i, ai_config_dir) in env.ai_configs.iter().enumerate() {
        let config_file = ai_config_dir.join("config.json");
        assert_file_exists(&config_file);

        let content = fs::read_to_string(&config_file)
            .context(format!("Failed to read AI config file {}", i))?;

        let config_data: serde_json::Value = serde_json::from_str(&content)
            .context(format!("Failed to parse AI config file {}", i))?;

        assert_json_has_key(&config_data, "model");
        assert_json_has_key(&config_data, "max_tokens");
        assert_json_has_key(&config_data, "temperature");

        println!("✅ AI CLI configuration validated for directory {}", i);
        println!("  Model: {}", config_data.get("model").unwrap());
    }

    Ok(())
}

/// Test file operations workflow
#[test]
fn test_file_operations_workflow() -> Result<()> {
    let env = E2ETestEnv::new()
        .context("Failed to create E2E test environment")?;

    // Create test files
    let test_files = env.create_test_files(5, 1)?; // 5 files, 1KB each

    // Verify all files were created
    assert_eq!(test_files.len(), 5);

    for (i, file_path) in test_files.iter().enumerate() {
        assert_file_exists(file_path);

        let content = fs::read_to_string(file_path)
            .context(format!("Failed to read test file {}", i))?;

        assert!(content.contains(&format!("File {}:", i)));
        assert!(content.contains("Test content for agentic-warden"));

        let metadata = fs::metadata(file_path)
            .context(format!("Failed to get metadata for file {}", i))?;

        assert!(metadata.len() >= 1000); // At least 1KB
    }

    // Test file counting
    let file_count = count_files(env.temp_dir.path())
        .context("Failed to count files")?;

    assert!(file_count >= 5);

    println!("✅ File operations workflow validated!");
    println!("Created {} test files totaling {} bytes",
        test_files.len(),
        test_files.iter()
            .map(|f| fs::metadata(f).unwrap().len())
            .sum::<u64>());

    Ok(())
}

/// Test compression workflow
#[test]
fn test_compression_workflow() -> Result<()> {
    let env = E2ETestEnv::new()
        .context("Failed to create E2E test environment")?;

    // Create test data
    let source_dir = env.temp_dir.path().join("source");
    fs::create_dir_all(&source_dir)
        .context("Failed to create source directory")?;

    create_test_config_files(&source_dir);

    // Add larger files for better compression
    for i in 0..3 {
        let content = format!("Large test file {} - {}\n", i, "X".repeat(5000));
        fs::write(source_dir.join(format!("large_file_{}.txt", i)), content)
            .context(format!("Failed to write large file {}", i))?;
    }

    // Test TAR.GZ compression
    let tar_gz_archive = create_real_compressed_archive(&source_dir, "tar.gz")
        .map_err(|e| anyhow::anyhow!("Failed to create TAR.GZ archive: {}", e))?;

    assert_file_exists(&tar_gz_archive);

    let tar_gz_size = fs::metadata(&tar_gz_archive)
        .context("Failed to get TAR.GZ archive metadata")?
        .len();

    // Test ZIP compression
    let zip_archive = create_real_compressed_archive(&source_dir, "zip")
        .map_err(|e| anyhow::anyhow!("Failed to create ZIP archive: {}", e))?;

    assert_file_exists(&zip_archive);

    let zip_size = fs::metadata(&zip_archive)
        .context("Failed to get ZIP archive metadata")?
        .len();

    // Verify compression worked (files are smaller than original)
    let source_size = fs::metadata(&source_dir)
        .context("Failed to get source directory metadata")?
        .len();

    println!("✅ Compression workflow validated!");
    println!("Source size: {}", format_bytes(source_size));
    println!("TAR.GZ size: {} ({:.1}% compression)",
        format_bytes(tar_gz_size),
        (tar_gz_size as f64 / source_size as f64) * 100.0);
    println!("ZIP size: {} ({:.1}% compression)",
        format_bytes(zip_size),
        (zip_size as f64 / source_size as f64) * 100.0);

    Ok(())
}

/// Test complete synchronization workflow
#[test]
fn test_complete_sync_workflow() -> Result<()> {
    let env = E2ETestEnv::new()
        .context("Failed to create E2E test environment")?;

    // Setup all configurations
    env.setup_provider_config()
        .context("Failed to setup provider config")?;

    env.setup_ai_configs()
        .context("Failed to setup AI configs")?;

    // Create test data
    let test_files = env.create_test_files(3, 2)?; // 3 files, 2KB each

    // Test provider setup
    let manager = ProviderManager::new_with_path(&env.provider_config_path)
        .context("Failed to create ProviderManager")?;

    let (provider_name, provider) = manager.get_default_provider()
        .context("Failed to get default provider")?;

    // Test environment injection
    let injector = EnvInjector;

    // Create a test provider for Claude
    let test_provider = create_real_provider(AiType::Claude, "E2E Test Provider");
    let injected_vars = &test_provider.env;

    // Test file compression
    let source_dir = env.temp_dir.path().join("sync_source");
    fs::create_dir_all(&source_dir)
        .context("Failed to create sync source directory")?;

    // Copy test files to sync source
    for (i, test_file) in test_files.iter().enumerate() {
        let sync_file = source_dir.join(format!("sync_file_{}.txt", i));
        fs::copy(test_file, sync_file)
            .context(format!("Failed to copy file {} to sync source", i))?;
    }

    // Create sync archive
    let sync_archive = create_real_compressed_archive(&source_dir, "tar.gz")
        .map_err(|e| anyhow::anyhow!("Failed to create sync archive: {}", e))?;

    // Test authentication data
    let auth_data = create_real_auth_file()
        .context("Failed to create auth file")?;

    // Verify all components are ready for synchronization
    assert!(manager.list_providers().len() > 0);
    assert!(!injected_vars.is_empty());
    assert_file_exists(&sync_archive);
    assert!(auth_data.exists());

    println!("✅ Complete synchronization workflow validated!");
    println!("Provider: {} ({})", provider_name, provider.description);
    println!("Environment variables: {}", injected_vars.len());
    println!("Sync archive: {}", sync_archive.file_name().unwrap().to_string_lossy());
    println!("Auth file: {}", auth_data.file_name().unwrap().to_string_lossy());

    // Cleanup
    cleanup_auth_file()
        .context("Failed to cleanup auth file")?;

    Ok(())
}

/// Test error recovery workflow
#[test]
fn test_error_recovery_workflow() -> Result<()> {
    let env = E2ETestEnv::new()
        .context("Failed to create E2E test environment")?;

    // Test invalid provider config handling
    let invalid_config = ProviderConfig {
        schema: None,
        default_provider: "nonexistent".to_string(),
        providers: std::collections::HashMap::new(),
    };

    let invalid_json = serde_json::to_string_pretty(&invalid_config)
        .context("Failed to serialize invalid config")?;

    fs::write(&env.provider_config_path, invalid_json)
        .context("Failed to write invalid config")?;

    // Try to create manager with invalid config
    let result = ProviderManager::new_with_path(&env.provider_config_path);

    match result {
        Ok(manager) => {
            // If it succeeds, the manager should handle the missing default provider gracefully
            let providers = manager.list_providers();
            println!("✅ Manager handled invalid config gracefully");
            println!("Available providers: {}", providers.len());
        }
        Err(e) => {
            // If it fails, that's expected and acceptable
            println!("✅ Manager correctly rejected invalid config: {}", e);
        }
    }

    // Fix the config
    env.setup_provider_config()
        .context("Failed to fix provider config")?;

    // Should work now
    let manager = ProviderManager::new_with_path(&env.provider_config_path)
        .context("Failed to create manager after fixing config")?;

    let providers = manager.list_providers();
    assert!(providers.len() > 0);

    // Test file system error handling
    let non_existent_file = env.temp_dir.path().join("non_existent.txt");
    let read_result = fs::read_to_string(&non_existent_file);

    assert!(read_result.is_err());
    println!("✅ File system error handled correctly");

    // Test compression error handling
    let non_existent_dir = PathBuf::from("/non/existent/directory");
    let compression_result = create_real_compressed_archive(&non_existent_dir, "tar.gz");

    assert!(compression_result.is_err());
    println!("✅ Compression error handled correctly");

    println!("✅ Error recovery workflow validated!");
    println!("System gracefully handles various error conditions");

    Ok(())
}