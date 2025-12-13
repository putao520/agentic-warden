//! Registry creation utilities
//!
//! Simple functions to create registry instances without factory pattern overhead.

use crate::{
    error::RegistryError,
    storage::{InProcessStorage, SharedMemoryStorage},
    unified_registry::Registry,
};
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use std::sync::Arc;

/// 类型别名，提高可读性
pub type CliRegistry = Registry<SharedMemoryStorage>;
pub type McpRegistry = Registry<InProcessStorage>;

/// Global registry factory (Singleton) used by tests and runtime utilities
#[derive(Debug)]
pub struct RegistryFactory {
    cli_registry: Mutex<Option<Arc<CliRegistry>>>,
    mcp_registry: Arc<McpRegistry>,
}

impl RegistryFactory {
    fn new() -> Self {
        Self {
            cli_registry: Mutex::new(None),
            mcp_registry: Arc::new(create_mcp_registry()),
        }
    }

    /// Return the global factory instance
    pub fn instance() -> &'static Self {
        static INSTANCE: OnceCell<RegistryFactory> = OnceCell::new();
        INSTANCE.get_or_init(Self::new)
    }

    /// Get (and lazily initialize) the CLI registry. Cached after first success.
    pub fn get_cli_registry(&self) -> Result<Arc<CliRegistry>, RegistryError> {
        let mut guard = self.cli_registry.lock();
        if let Some(registry) = guard.as_ref() {
            return Ok(Arc::clone(registry));
        }

        let registry = Arc::new(create_cli_registry()?);
        *guard = Some(Arc::clone(&registry));
        Ok(registry)
    }

    /// Get the MCP registry (in-process, initialized at startup)
    pub fn get_mcp_registry(&self) -> Arc<McpRegistry> {
        Arc::clone(&self.mcp_registry)
    }
}

/// 创建CLI任务注册表（跨进程共享）
///
/// 连接到当前进程的共享内存命名空间
pub fn create_cli_registry() -> Result<CliRegistry, RegistryError> {
    Ok(Registry::new(SharedMemoryStorage::connect()?))
}

/// 创建MCP任务注册表（进程内独享）
///
/// 返回新的进程内DashMap注册表实例
pub fn create_mcp_registry() -> McpRegistry {
    Registry::new(InProcessStorage::new())
}

/// 为指定PID创建CLI任务注册表
pub fn create_cli_registry_for_pid(pid: u32) -> Result<CliRegistry, RegistryError> {
    Ok(Registry::new(SharedMemoryStorage::connect_for_pid(pid)?))
}

/// 使用自定义命名空间创建CLI任务注册表
pub fn create_cli_registry_with_namespace(namespace: String) -> Result<CliRegistry, RegistryError> {
    Ok(Registry::new(SharedMemoryStorage::connect_with_namespace(
        namespace,
    )?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_mcp_registry() {
        let _registry = create_mcp_registry();
        // MCP registry should always succeed to create
        assert!(true); // If we reach here, creation succeeded
    }

    #[test]
    fn test_create_cli_registry() {
        // This should succeed as it connects to current process's shared memory
        let result = create_cli_registry();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_cli_registry_for_current_pid() {
        let current_pid = std::process::id();
        let result = create_cli_registry_for_pid(current_pid);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_cli_registry_custom_namespace() {
        let namespace = format!("test_namespace_{}", std::process::id());
        let result = create_cli_registry_with_namespace(namespace.clone());
        assert!(result.is_ok());
    }
}
