//! 任务注册表工厂
//!
//! 提供单例模式的注册表管理，根据任务来源自动选择使用不同的注册表实现

use crate::{
    error::RegistryError,
    process_registry::InProcessRegistry,
    registry::TaskRegistry,
};
use parking_lot::Mutex;
use std::sync::{Arc, OnceLock};

/// 任务来源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskSource {
    /// CLI命令启动的任务（跨进程共享）
    Cli,
    /// MCP启动的任务（进程内独享）
    Mcp,
}

/// 注册表工厂 - 管理进程内的两个独立任务注册表
///
/// # 架构
///
/// ```text
/// ┌─────────────────────────────────────────┐
/// │       RegistryFactory (Singleton)       │
/// ├─────────────────────────────────────────┤
/// │                                         │
/// │  ┌─────────────┐    ┌────────────────┐ │
/// │  │TaskRegistry │    │InProcessRegistry│ │
/// │  │(SharedMem)  │    │  (HashMap)     │ │
/// │  │跨进程共享    │    │  进程内独享    │ │
/// │  └─────────────┘    └────────────────┘ │
/// │       ↑                    ↑           │
/// │       │                    │           │
/// │  CLI任务                 MCP任务       │
/// └─────────────────────────────────────────┘
/// ```
pub struct RegistryFactory {
    /// CLI任务注册表（跨进程共享内存）
    cli_registry: Mutex<Option<TaskRegistry>>,
    /// MCP任务注册表（进程内HashMap）
    mcp_registry: Arc<InProcessRegistry>,
}

impl RegistryFactory {
    /// 获取全局单例实例
    ///
    /// # Example
    ///
    /// ```rust
    /// use agentic_warden::registry_factory::{RegistryFactory, TaskSource};
    ///
    /// let factory = RegistryFactory::instance();
    /// let cli_registry = factory.get_cli_registry().unwrap();
    /// let mcp_registry = factory.get_mcp_registry();
    /// ```
    pub fn instance() -> &'static Self {
        static INSTANCE: OnceLock<RegistryFactory> = OnceLock::new();
        INSTANCE.get_or_init(|| Self {
            cli_registry: Mutex::new(None),
            mcp_registry: Arc::new(InProcessRegistry::new()),
        })
    }

    /// 获取CLI任务注册表（跨进程共享）
    ///
    /// 延迟初始化：第一次调用时连接到共享内存
    pub fn get_cli_registry(&self) -> Result<TaskRegistry, RegistryError> {
        let mut guard = self.cli_registry.lock();
        if guard.is_none() {
            *guard = Some(TaskRegistry::connect()?);
        }
        Ok(guard.as_ref().unwrap().clone())
    }

    /// 获取MCP任务注册表（进程内独享）
    ///
    /// 总是返回同一个InProcessRegistry实例
    pub fn get_mcp_registry(&self) -> Arc<InProcessRegistry> {
        Arc::clone(&self.mcp_registry)
    }

    /// 根据任务来源获取对应的注册表
    ///
    /// # Arguments
    ///
    /// * `source` - 任务来源（CLI或MCP）
    ///
    /// # Example
    ///
    /// ```rust
    /// use agentic_warden::registry_factory::{RegistryFactory, TaskSource};
    ///
    /// let factory = RegistryFactory::instance();
    ///
    /// // CLI任务
    /// match factory.get_registry_for(TaskSource::Cli) {
    ///     RegistryType::Cli(registry) => {
    ///         // 使用TaskRegistry
    ///     }
    ///     _ => {}
    /// }
    ///
    /// // MCP任务
    /// match factory.get_registry_for(TaskSource::Mcp) {
    ///     RegistryType::Mcp(registry) => {
    ///         // 使用InProcessRegistry
    ///     }
    ///     _ => {}
    /// }
    /// ```
    pub fn get_registry_for(&self, source: TaskSource) -> Result<RegistryType, RegistryError> {
        match source {
            TaskSource::Cli => Ok(RegistryType::Cli(self.get_cli_registry()?)),
            TaskSource::Mcp => Ok(RegistryType::Mcp(self.get_mcp_registry())),
        }
    }

    /// 重置CLI注册表连接（用于测试或错误恢复）
    #[cfg(test)]
    pub fn reset_cli_registry(&self) {
        let mut guard = self.cli_registry.lock();
        *guard = None;
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> FactoryStats {
        FactoryStats {
            cli_initialized: self.cli_registry.lock().is_some(),
            mcp_initialized: true, // MCP注册表总是初始化的
        }
    }
}

/// 注册表类型枚举
///
/// 包装不同的注册表实现，提供统一的接口
pub enum RegistryType {
    /// CLI任务注册表
    Cli(TaskRegistry),
    /// MCP任务注册表
    Mcp(Arc<InProcessRegistry>),
}

impl RegistryType {
    /// 获取任务来源类型
    pub fn source(&self) -> TaskSource {
        match self {
            RegistryType::Cli(_) => TaskSource::Cli,
            RegistryType::Mcp(_) => TaskSource::Mcp,
        }
    }

    /// 转换为TaskRegistry引用（如果是CLI类型）
    pub fn as_cli(&self) -> Option<&TaskRegistry> {
        match self {
            RegistryType::Cli(r) => Some(r),
            _ => None,
        }
    }

    /// 转换为InProcessRegistry引用（如果是MCP类型）
    pub fn as_mcp(&self) -> Option<&Arc<InProcessRegistry>> {
        match self {
            RegistryType::Mcp(r) => Some(r),
            _ => None,
        }
    }
}

/// 工厂统计信息
#[derive(Debug, Clone)]
pub struct FactoryStats {
    /// CLI注册表是否已初始化
    pub cli_initialized: bool,
    /// MCP注册表是否已初始化
    pub mcp_initialized: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_singleton_instance() {
        let factory1 = RegistryFactory::instance();
        let factory2 = RegistryFactory::instance();

        // 应该是同一个实例
        assert!(std::ptr::eq(factory1, factory2));
    }

    #[test]
    fn test_mcp_registry_always_available() {
        let factory = RegistryFactory::instance();
        let mcp1 = factory.get_mcp_registry();
        let mcp2 = factory.get_mcp_registry();

        // 应该是同一个Arc
        assert!(Arc::ptr_eq(&mcp1, &mcp2));
    }

    #[test]
    fn test_cli_registry_lazy_init() {
        let factory = RegistryFactory::instance();

        // 重置状态
        factory.reset_cli_registry();

        let stats = factory.get_stats();
        assert!(!stats.cli_initialized);

        // 第一次获取会初始化
        let _cli = factory.get_cli_registry();

        let stats = factory.get_stats();
        assert!(stats.cli_initialized);
    }

    #[test]
    fn test_get_registry_for_cli() {
        let factory = RegistryFactory::instance();
        let registry = factory.get_registry_for(TaskSource::Cli).unwrap();

        assert_eq!(registry.source(), TaskSource::Cli);
        assert!(registry.as_cli().is_some());
        assert!(registry.as_mcp().is_none());
    }

    #[test]
    fn test_get_registry_for_mcp() {
        let factory = RegistryFactory::instance();
        let registry = factory.get_registry_for(TaskSource::Mcp).unwrap();

        assert_eq!(registry.source(), TaskSource::Mcp);
        assert!(registry.as_cli().is_none());
        assert!(registry.as_mcp().is_some());
    }

    #[test]
    fn test_multiple_mcp_registries_share_instance() {
        let factory = RegistryFactory::instance();

        let reg1 = factory.get_registry_for(TaskSource::Mcp).unwrap();
        let reg2 = factory.get_registry_for(TaskSource::Mcp).unwrap();

        if let (RegistryType::Mcp(r1), RegistryType::Mcp(r2)) = (reg1, reg2) {
            assert!(Arc::ptr_eq(&r1, &r2));
        } else {
            panic!("Expected MCP registries");
        }
    }
}
