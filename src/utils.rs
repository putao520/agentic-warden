/// Utility functions for the agentic-warden project
///
/// Get the instance ID for the current process
/// Uses a simple hash of the process ID to ensure uniqueness
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
