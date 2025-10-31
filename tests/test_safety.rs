// 测试安全机制
// 为真实集成测试提供安全保障

use anyhow::{Context, Result};
use parking_lot::Mutex;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;
use tracing::{error, info, warn};

/// 测试安全配置
#[derive(Debug, Clone)]
pub struct TestSafetyConfig {
    /// 测试数据根目录
    pub test_root_dir: PathBuf,
    /// 是否启用真实API调用
    pub real_api_enabled: bool,
    /// 备份原始配置路径
    pub backup_dir: PathBuf,
    /// 隔离命名空间
    pub namespace: String,
}

impl Default for TestSafetyConfig {
    fn default() -> Self {
        let test_root = PathBuf::from("target/test_data").join(format!(
            "test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ));

        Self {
            test_root_dir: test_root.clone(),
            real_api_enabled: std::env::var("AGENTIC_WARDEN_REAL_TESTS")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            backup_dir: test_root.join("backups"),
            namespace: format!(
                "agentic_warden_test_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            ),
        }
    }
}

/// 测试安全上下文
#[derive(Debug)]
pub struct TestSafetyContext {
    config: TestSafetyConfig,
    temp_dirs: Arc<Mutex<Vec<PathBuf>>>,
    backed_up_files: Arc<Mutex<Vec<PathBuf>>>,
    created_test_files: Arc<Mutex<Vec<PathBuf>>>,
}

impl TestSafetyContext {
    /// 创建新的测试安全上下文
    pub fn new() -> Result<Self> {
        let config = TestSafetyConfig::default();

        // 确保测试目录存在
        fs::create_dir_all(&config.test_root_dir).with_context(|| {
            format!(
                "Failed to create test root directory: {:?}",
                config.test_root_dir
            )
        })?;

        fs::create_dir_all(&config.backup_dir).with_context(|| {
            format!("Failed to create backup directory: {:?}", config.backup_dir)
        })?;

        info!(
            "Creating test safety context with namespace: {}",
            config.namespace
        );

        Ok(Self {
            config,
            temp_dirs: Arc::new(Mutex::new(Vec::new())),
            backed_up_files: Arc::new(Mutex::new(Vec::new())),
            created_test_files: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// 创建隔离的测试目录
    pub fn create_test_dir(&self, name: &str) -> Result<PathBuf> {
        let test_dir = self.config.test_root_dir.join(format!(
            "{}_{}",
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        fs::create_dir_all(&test_dir)
            .with_context(|| format!("Failed to create test directory: {:?}", test_dir))?;

        self.temp_dirs.lock().push(test_dir.clone());
        info!("Created isolated test directory: {:?}", test_dir);

        Ok(test_dir)
    }

    /// 创建临时目录
    pub fn create_temp_dir(&self) -> Result<TempDir> {
        let temp_dir = TempDir::new_in(&self.config.test_root_dir)
            .context("Failed to create temporary directory")?;

        let path = temp_dir.path().to_path_buf();
        self.temp_dirs.lock().push(path);
        info!("Created temporary directory: {:?}", temp_dir.path());

        Ok(temp_dir)
    }

    /// 安全备份文件
    pub fn backup_file(&self, file_path: &Path) -> Result<PathBuf> {
        if !file_path.exists() {
            warn!("File to backup does not exist: {:?}", file_path);
            return Ok(file_path.to_path_buf());
        }

        let backup_name = format!(
            "{}.backup.{}",
            file_path.file_name().unwrap().to_string_lossy(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );

        let backup_path = self.config.backup_dir.join(&backup_name);

        fs::copy(file_path, &backup_path).with_context(|| {
            format!(
                "Failed to backup file from {:?} to {:?}",
                file_path, backup_path
            )
        })?;

        self.backed_up_files.lock().push(backup_path.clone());
        info!("Backed up file: {:?} -> {:?}", file_path, backup_path);

        Ok(backup_path)
    }

    /// 恢复文件
    pub fn restore_file(&self, backup_path: &Path, original_path: &Path) -> Result<()> {
        if !backup_path.exists() {
            warn!("Backup file does not exist: {:?}", backup_path);
            return Ok(());
        }

        fs::copy(backup_path, original_path).with_context(|| {
            format!(
                "Failed to restore file from {:?} to {:?}",
                backup_path, original_path
            )
        })?;

        info!("Restored file: {:?} -> {:?}", backup_path, original_path);
        Ok(())
    }

    /// 创建测试文件
    pub fn create_test_file(&self, path: &Path, content: &str) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent directory: {:?}", parent))?;
        }

        fs::write(path, content)
            .with_context(|| format!("Failed to write test file: {:?}", path))?;

        self.created_test_files.lock().push(path.to_path_buf());
        info!("Created test file: {:?}", path);
        Ok(())
    }

    /// 检查是否可以运行真实API测试
    pub fn can_run_real_api_tests(&self) -> bool {
        if !self.config.real_api_enabled {
            warn!("Real API tests are disabled. Set AGENTIC_WARDEN_REAL_TESTS=true to enable.");
            return false;
        }

        // 检查必要的环境变量
        let required_vars = [
            "GOOGLE_CLIENT_ID",
            "GOOGLE_CLIENT_SECRET",
            "GOOGLE_TEST_REFRESH_TOKEN",
        ];

        let mut missing_vars = Vec::new();
        for var in &required_vars {
            if std::env::var(var).is_err() {
                missing_vars.push(*var);
            }
        }

        if !missing_vars.is_empty() {
            error!(
                "Missing required environment variables for real API tests: {:?}",
                missing_vars
            );
            return false;
        }

        true
    }

    /// 安全清理
    pub fn cleanup(&self) -> Result<()> {
        info!(
            "Starting test safety cleanup for namespace: {}",
            self.config.namespace
        );

        let mut errors = Vec::new();

        // 清理临时目录
        let temp_dirs = self.temp_dirs.lock();
        for temp_dir in temp_dirs.iter() {
            if temp_dir.exists() {
                if let Err(e) = fs::remove_dir_all(temp_dir) {
                    warn!("Failed to remove temporary directory {:?}: {}", temp_dir, e);
                    errors.push(e);
                } else {
                    info!("Removed temporary directory: {:?}", temp_dir);
                }
            }
        }
        drop(temp_dirs);

        // 清理测试文件
        let test_files = self.created_test_files.lock();
        for test_file in test_files.iter() {
            if test_file.exists() {
                if let Err(e) = fs::remove_file(test_file) {
                    warn!("Failed to remove test file {:?}: {}", test_file, e);
                    errors.push(e);
                } else {
                    info!("Removed test file: {:?}", test_file);
                }
            }
        }
        drop(test_files);

        // 可选：保留备份文件用于调试
        let keep_backups = std::env::var("AGENTIC_WARDEN_KEEP_TEST_BACKUPS")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        if !keep_backups {
            let backed_up_files = self.backed_up_files.lock();
            for backup_file in backed_up_files.iter() {
                if backup_file.exists() {
                    if let Err(e) = fs::remove_file(backup_file) {
                        warn!("Failed to remove backup file {:?}: {}", backup_file, e);
                        errors.push(e);
                    } else {
                        info!("Removed backup file: {:?}", backup_file);
                    }
                }
            }
            drop(backed_up_files);
        }

        if errors.is_empty() {
            info!("Test safety cleanup completed successfully");
            Ok(())
        } else {
            error!("Test safety cleanup completed with {} errors", errors.len());
            Err(anyhow::anyhow!(
                "Cleanup completed with {} errors",
                errors.len()
            ))
        }
    }

    /// 获取配置引用
    pub fn config(&self) -> &TestSafetyConfig {
        &self.config
    }

    /// 获取命名空间
    pub fn namespace(&self) -> &str {
        &self.config.namespace
    }
}

impl Drop for TestSafetyContext {
    fn drop(&mut self) {
        if let Err(e) = self.cleanup() {
            error!("Auto cleanup failed: {}", e);
        }
    }
}

/// 测试断言辅助函数
pub mod assertions {
    use super::*;

    /// 断言条件为真
    pub fn assert(condition: bool, message: &str) -> Result<()> {
        assert!(condition, "{}", message);
        Ok(())
    }

    /// 断言相等
    pub fn assert_eq<T: std::fmt::Debug + PartialEq>(
        actual: T,
        expected: T,
        message: &str,
    ) -> Result<()> {
        assert_eq!(actual, expected, "{}", message);
        Ok(())
    }

    /// 断言匹配模式
    pub fn assert_matches<T: std::fmt::Debug>(
        value: T,
        pattern: &str,
        message: &str,
    ) -> Result<()> {
        // 简化的匹配检查，这里我们直接检查值的字符串表示
        let value_str = format!("{:?}", value);
        assert!(
            value_str.contains(pattern) || pattern == "*",
            "Expected pattern '{}' not found in value '{:?}': {}",
            pattern,
            value,
            message
        );
        Ok(())
    }

    /// 断言字符串不为空
    pub fn assert_not_empty(value: &str, message: &str) -> Result<()> {
        assert!(!value.is_empty(), "{}", message);
        Ok(())
    }

    /// 断言文件存在
    pub fn assert_file_exists(path: &Path) -> Result<()> {
        assert!(path.exists(), "File should exist: {:?}", path);
        Ok(())
    }

    /// 断言文件不存在
    pub fn assert_file_not_exists(path: &Path) -> Result<()> {
        assert!(!path.exists(), "File should not exist: {:?}", path);
        Ok(())
    }

    /// 断言文件内容匹配
    pub fn assert_file_content(path: &Path, expected_content: &str) -> Result<()> {
        let actual_content =
            fs::read_to_string(path).with_context(|| format!("Failed to read file: {:?}", path))?;

        assert_eq!(
            actual_content, expected_content,
            "File content mismatch in {:?}",
            path
        );
        Ok(())
    }

    /// 断言目录存在
    pub fn assert_dir_exists(path: &Path) -> Result<()> {
        assert!(
            path.exists() && path.is_dir(),
            "Directory should exist: {:?}",
            path
        );
        Ok(())
    }

    /// 断言目录不存在
    pub fn assert_dir_not_exists(path: &Path) -> Result<()> {
        assert!(
            !path.exists() || !path.is_dir(),
            "Directory should not exist: {:?}",
            path
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_safety_context_creation() {
        let context = TestSafetyContext::new().unwrap();
        assert!(context.config().test_root_dir.exists());
        assert!(context.config().backup_dir.exists());
        assert!(!context.namespace().is_empty());
    }

    #[test]
    fn test_create_test_dir() {
        let context = TestSafetyContext::new().unwrap();
        let test_dir = context.create_test_dir("my_test").unwrap();
        assert!(test_dir.exists());
        assert!(test_dir.is_dir());
        assert!(test_dir.starts_with(&context.config().test_root_dir));
    }

    #[test]
    fn test_backup_and_restore_file() -> Result<()> {
        let context = TestSafetyContext::new()?;
        let test_file = context.create_test_dir("backup_test")?.join("test.txt");

        // 创建测试文件
        context.create_test_file(&test_file, "original content")?;

        // 备份文件
        let backup_path = context.backup_file(&test_file)?;
        assert!(backup_path.exists());

        // 修改原文件
        fs::write(&test_file, "modified content")?;

        // 恢复文件
        context.restore_file(&backup_path, &test_file)?;

        // 验证内容已恢复
        let content = fs::read_to_string(&test_file)?;
        assert_eq!(content, "original content");

        Ok(())
    }
}
