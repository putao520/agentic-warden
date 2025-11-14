//! Registry creation utilities
//!
//! Simple functions to create registry instances without factory pattern overhead.

use crate::{
    error::RegistryError,
    storage::{InProcessStorage, SharedMemoryStorage},
    unified_registry::Registry,
};

/// 类型别名，提高可读性
pub type CliRegistry = Registry<SharedMemoryStorage>;
pub type McpRegistry = Registry<InProcessStorage>;

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
    Ok(Registry::new(SharedMemoryStorage::connect_with_namespace(namespace)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_mcp_registry() {
        let registry = create_mcp_registry();
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