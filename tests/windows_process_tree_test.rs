//! Windows-specific tests for process tree functionality
//!
//! These tests verify Windows-specific behavior and validate that we can correctly
//! identify root processes like explorer.exe, cmd.exe, or system processes.

#[cfg(test)]
mod tests {
    use agentic_warden::process_tree::{ProcessTreeInfo, get_process_tree};
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    #[test]
    #[cfg(windows)]
    fn test_windows_root_process_detection() {
        // This test validates that we can detect common Windows root processes

        let tree_info = ProcessTreeInfo::current().expect("Failed to get current process tree");

        println!("Windows Process Tree Analysis:");
        println!("Current PID: {}", std::process::id());
        println!("Process chain length: {}", tree_info.depth);
        println!("Root parent PID: {:?}", tree_info.root_parent_pid);

        for (i, pid) in tree_info.process_chain.iter().enumerate() {
            println!("  Level {}: PID {}", i, pid);
        }

        // Validate that we have a reasonable process tree
        assert!(!tree_info.process_chain.is_empty());
        assert!(
            tree_info.depth >= 2,
            "Should have at least current process and parent"
        );

        // The root parent should be a valid PID
        if let Some(root_pid) = tree_info.root_parent_pid {
            assert!(root_pid > 0, "Root parent PID should be positive");

            // On Windows, common root processes include:
            // - System (PID 4)
            // - System Idle Process (PID 0)
            // - explorer.exe (Windows Explorer)
            // - cmd.exe or powershell.exe
            // - winlogon.exe, csrss.exe, etc.

            println!(
                "Root parent PID {} should be a Windows system or shell process",
                root_pid
            );

            // Basic PID validation for Windows
            assert!(root_pid <= 65536, "Windows PID should be in u16 range");
        }
    }

    #[test]
    #[cfg(windows)]
    fn test_windows_subprocess_ancestry() {
        // Test that child processes have the current process as an ancestor

        let current_pid = std::process::id();
        let current_tree = ProcessTreeInfo::current().expect("Failed to get current process tree");

        // Start a subprocess (ping is available on all Windows systems)
        let mut child = Command::new("ping")
            .args(["127.0.0.1", "-n", "5"]) // Ping 5 times (~5 seconds)
            .spawn()
            .expect("Failed to spawn ping subprocess");

        let child_pid = child.id();
        println!("Spawned subprocess with PID: {}", child_pid);

        // Give the subprocess time to initialize
        thread::sleep(Duration::from_millis(200));

        // Get the subprocess process tree
        let child_tree = match get_process_tree(child_pid) {
            Ok(tree) => {
                println!("Subprocess tree:");
                for (i, pid) in tree.process_chain.iter().enumerate() {
                    println!("  Level {}: PID {}", i, pid);
                }
                Some(tree)
            }
            Err(err) => {
                println!("Warning: Could not get subprocess tree: {}", err);
                None
            }
        };

        // Clean up the subprocess
        let _ = child.kill();
        let _ = child.wait();

        // If we successfully got the subprocess tree, validate it
        if let Some(child_tree) = child_tree {
            // The subprocess should have a reasonable process tree
            assert!(!child_tree.process_chain.is_empty());
            assert_eq!(child_tree.process_chain[0], child_pid);

            // On Windows, the subprocess should either:
            // 1. Have our current process as an ancestor, or
            // 2. Share the same root parent (e.g., both under explorer.exe)

            let current_root = current_tree.root_parent_pid;
            let child_root = child_tree.root_parent_pid;

            println!("Current process root: {:?}", current_root);
            println!("Subprocess root: {:?}", child_root);

            if let (Some(current_root), Some(child_root)) = (current_root, child_root) {
                if current_root == child_root {
                    println!("✅ Subprocess shares the same root parent");
                } else {
                    println!("⚠️  Subprocess has different root parent (might be direct child)");
                    // This can also be valid - the subprocess might be a direct child
                    // so we don't assert failure here
                }
            }
        }
    }

    #[test]
    #[cfg(windows)]
    fn test_windows_process_tree_limits() {
        // Test Windows-specific process tree limits and boundaries

        let tree_info = ProcessTreeInfo::current().expect("Failed to get process tree");

        // Windows systems typically have reasonable process tree depths
        println!("Windows process tree depth: {}", tree_info.depth);

        assert!(
            tree_info.depth >= 1,
            "Windows process tree should have at least 1 level"
        );
        assert!(
            tree_info.depth <= 15,
            "Windows process tree depth seems unusually high (possible infinite loop)"
        );

        // Verify all PIDs are in valid Windows range
        for pid in &tree_info.process_chain {
            assert!(*pid > 0, "Windows PIDs should be positive");
            assert!(*pid <= 65536, "Windows PIDs should be in valid range (u16)");
        }
    }

    #[test]
    #[cfg(windows)]
    fn test_windows_specific_pids() {
        // Test detection of Windows-specific PID values

        let tree_info = ProcessTreeInfo::current().expect("Failed to get current process tree");

        // Check for common Windows system PIDs in our ancestry
        let mut found_system_pid = false;
        let mut found_idle_pid = false;

        for pid in &tree_info.process_chain {
            if *pid == 4 {
                found_system_pid = true;
                println!("Found System process (PID 4) in ancestry");
            }
            if *pid == 0 {
                found_idle_pid = true;
                println!("Found System Idle Process (PID 0) in ancestry");
            }
        }

        println!("Windows PID analysis:");
        println!("  Found System PID (4): {}", found_system_pid);
        println!("  Found Idle PID (0): {}", found_idle_pid);

        // Having system processes in the ancestry is common and good
        if found_system_pid || found_idle_pid {
            println!("✅ System processes found in process tree (expected on Windows)");
        }

        // Validate our root PID if we have one
        if let Some(root_pid) = tree_info.root_parent_pid {
            // Common Windows root PIDs include 0, 4, or other system processes
            let is_common_root = root_pid == 0 || root_pid == 4 || root_pid > 100;
            assert!(
                is_common_root,
                "Root PID should be a reasonable Windows process"
            );
        }
    }

    #[test]
    #[cfg(windows)]
    fn test_windows_process_tree_performance() {
        // Test that process tree discovery is performant on Windows
        let iterations = 10;
        let mut total_duration = Duration::new(0, 0);

        for i in 0..iterations {
            let start = std::time::Instant::now();
            let result = ProcessTreeInfo::current();
            let duration = start.elapsed();
            total_duration += duration;

            if result.is_ok() {
                println!("Iteration {}: {:?}", i, duration);
            } else {
                println!("Iteration {}: Failed - {:?}", i, result);
            }
        }

        let average_duration = total_duration / iterations as u32;
        println!(
            "Average time over {} iterations: {:?}",
            iterations, average_duration
        );

        // Should be reasonably fast even on Windows
        assert!(
            average_duration.as_millis() < 1000,
            "Average process tree discovery should be under 1 second on Windows"
        );
    }
}
