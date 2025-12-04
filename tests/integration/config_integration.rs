use aiw::provider::config::{ProvidersConfig, Region};
use aiw::provider::manager::ProviderManager;
use std::fs;
use tempfile::TempDir;

struct TempHome {
    dir: TempDir,
    old_home: Option<std::ffi::OsString>,
    old_user: Option<std::ffi::OsString>,
}

impl TempHome {
    fn new() -> Self {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().to_path_buf();
        let old_home = std::env::var_os("HOME");
        std::env::set_var("HOME", &path);
        let old_user = std::env::var_os("USERPROFILE");
        if cfg!(windows) {
            std::env::set_var("USERPROFILE", &path);
        }
        Self {
            dir,
            old_home,
            old_user,
        }
    }

    fn config_path(&self) -> std::path::PathBuf {
        self.dir.path().join(".agentic-warden").join("providers.json")
    }
}

impl Drop for TempHome {
    fn drop(&mut self) {
        if let Some(old) = self.old_home.take() {
            std::env::set_var("HOME", old);
        } else {
            std::env::remove_var("HOME");
        }
        if cfg!(windows) {
            if let Some(old) = self.old_user.take() {
                std::env::set_var("USERPROFILE", old);
            } else {
                std::env::remove_var("USERPROFILE");
            }
        }
    }
}

#[test]
fn providers_config_round_trip_in_isolated_home() {
    let home = TempHome::new();

    let mut config = ProvidersConfig::load().expect("should create default config");
    let path = home.config_path();
    assert!(
        path.exists(),
        "loading configuration should create default file"
    );

    config.set_token(
        "openrouter",
        Region::International,
        "token-abc".to_string(),
    );
    config.save_to_path(&path).expect("save to succeed");

    let reloaded = ProvidersConfig::load_from_path(&path).expect("reload should work");
    assert_eq!(
        reloaded
            .get_token("openrouter", &Region::International)
            .cloned(),
        Some("token-abc".to_string())
    );
}

#[test]
fn provider_manager_initialises_from_created_template() {
    let _home = TempHome::new();
    let mut manager = ProviderManager::new().expect("manager should initialise");
    manager.load_config().expect("load config");

    let providers = manager.get_providers();
    assert!(
        providers.iter().any(|(id, _)| id == &"openrouter"),
        "expected openrouter provider"
    );
}

#[test]
fn invalid_configuration_reports_parse_error() {
    let home = TempHome::new();
    let path = home.config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create directories");
    }
    fs::write(&path, "{ not json").expect("write invalid file");

    let err = ProvidersConfig::load_from_path(&path).expect_err("invalid json should fail");
    assert!(
        err.to_string().contains("Failed to parse providers config"),
        "unexpected error message: {err}"
    );
}
