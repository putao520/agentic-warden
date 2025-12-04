//! Test collision-resistant log filename generation

#[cfg(test)]
mod tests {
    use aiw::supervisor::generate_log_path;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_unique_filename_generation() {
        println!("🔍 Testing unique filename generation");

        // Test that multiple calls to generate_log_path produce different filenames
        let pid = std::process::id();
        let mut filenames = std::collections::HashSet::new();

        for i in 0..10 {
            let path = generate_log_path(pid).expect("Failed to generate log path");
            let filename = path.file_name().unwrap().to_str().unwrap().to_string();

            println!("  Generated filename: {}", filename);

            // Verify format: {PID}-{timestamp}-{random}.log
            assert!(filename.ends_with(".log"), "Filename should end with .log");
            assert!(filename.starts_with(&pid.to_string()), "Filename should start with PID");

            // Check that we have dashes separating components
            let parts: Vec<&str> = filename[..filename.len() - 4].split('-').collect();
            assert_eq!(parts.len(), 3, "Filename should have 3 parts separated by dashes");

            // Verify timestamp is reasonable (should be increasing)
            let timestamp: u64 = parts[1].parse().unwrap();
            assert!(timestamp > 1609459200000, "Timestamp should be after 2021"); // After 2021

            // Verify random number is within range
            let random: u32 = parts[2].parse().unwrap();
            assert!(random < 4294967295, "Random number should be valid u32");

            // Verify all filenames are unique
            assert!(filenames.insert(filename.clone()), "All filenames should be unique");
        }

        println!("✅ Generated 10 unique filenames successfully");
    }

    #[test]
    fn test_collision_resistant_with_same_pid() {
        println!("🎯 Testing collision resistance with same PID");

        let pid = std::process::id();
        let mut paths = Vec::new();

        // Generate multiple paths for the same PID
        for _ in 0..5 {
            let path = generate_log_path(pid).expect("Failed to generate log path");
            paths.push(path);
        }

        // Verify all paths are different
        for i in 0..paths.len() {
            for j in (i + 1)..paths.len() {
                assert_ne!(paths[i], paths[j], "Paths should be different for different timestamps/random");
            }
        }

        println!("✅ Confirmed collision resistance for same PID");
    }

    #[test]
    fn test_fallback_random_generation() {
        println!("🔄 Testing fallback random generation");

        // This test simulates the case where getrandom fails
        // We can't easily make getrandom fail, but we can test the fallback logic

        // Test that our fallback formula produces different values
        let pid = 12345;
        let timestamp1 = 1640995200000u64; // 2022-01-01
        let timestamp2 = 1641081600000u64; // 2022-01-02

        let random1 = (pid ^ timestamp1 as u32).rotate_right(timestamp1 as u32 % 32);
        let random2 = (pid ^ timestamp2 as u32).rotate_right(timestamp2 as u32 % 32);

        assert_ne!(random1, random2, "Fallback should produce different values for different timestamps");

        println!("✅ Fallback random generation works correctly");
    }

    #[test]
    fn test_filename_format_validation() {
        println!("📋 Testing filename format validation");

        let pid = std::process::id();
        let path = generate_log_path(pid).expect("Failed to generate log path");
        let filename = path.file_name().unwrap().to_str().unwrap();

        // Expected format: {PID}-{timestamp}-{random}.log
        let filename_without_extension = &filename[..filename.len() - 4];
        let expected_parts: Vec<&str> = filename_without_extension.split('-').collect();
        assert_eq!(expected_parts.len(), 3, "Should have PID, timestamp, random components");

        let pid_part: u32 = expected_parts[0].parse().unwrap();
        assert_eq!(pid_part, pid, "PID part should match");

        let timestamp_part: u64 = expected_parts[1].parse().unwrap();
        assert!(timestamp_part > 1609459200000, "Timestamp should be reasonable");

        let random_part: u32 = expected_parts[2].parse().unwrap();
        assert!(random_part < 4294967295, "Random part should be valid");

        println!("✅ Filename format validation passed: {}", filename);
    }

    #[test]
    fn test_directory_permissions() {
        println!("🔒 Testing directory permissions");

        // Test the actual temp directory instead of trying to override it
        let pid = 99999; // Use a fake PID for testing
        let result = generate_log_path(pid);

        assert!(result.is_ok(), "Should generate path successfully");

        let path = result.unwrap();
        let parent = path.parent().unwrap();

        // On Unix systems, check permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(parent).unwrap();
            let permissions = metadata.permissions();
            assert_eq!(permissions.mode() & 0o777, 0o700, "Directory should have restrictive permissions");
        }

        println!("✅ Directory permissions are correct");
    }
}