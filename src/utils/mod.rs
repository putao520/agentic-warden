//! 工具模块
//!
//! 提供各种工具函数和辅助功能

pub mod config_paths;
pub mod env;
pub mod logger;
pub mod version;

// Re-exports removed - not used in current implementation
// pub use config_paths::ConfigPaths;
// pub use logger::init_logger;
// pub use version::BuildInfo;

/// 获取实例ID
pub fn get_instance_id() -> u32 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let pid = std::process::id();
    let mut hasher = DefaultHasher::new();
    pid.hash(&mut hasher);

    // Ensure the ID is within a small range (1-100) for faster scanning
    ((hasher.finish() % 100) + 1) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_instance_id() {
        let id1 = get_instance_id();
        let id2 = get_instance_id();

        // Should return the same value within the same process
        assert_eq!(id1, id2);

        // Should be within the expected range
        assert!(id1 >= 1);
        assert!(id1 <= 100);
    }
}
