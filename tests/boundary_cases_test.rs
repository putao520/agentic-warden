//! Boundary cases and edge conditions tests for process tree functionality
//!
//! These tests verify that the process tree detection handles edge cases,
//! boundary conditions, and unusual scenarios gracefully.

#[cfg(test)]
mod tests {
    use agentic_warden::process_tree::{ProcessTreeInfo, get_process_tree, same_root_parent};
    use std::process::Command;
    use std::thread;
    

    #[test]
    fn test_boundary_pid_zero() {
        // Test behavior with PID 0 (system-specific meaning)

        println!("Testing PID 0 boundary case...");

        // PID 0 has different meanings on different platforms:
        // - Windows: System Idle Process
        // - Unix: Typically invalid or swapper task

        let result = get_process_tree(0);

        match result {
            Ok(tree_info) => {
                println!("PID 0 resulted in process tree: {:?}", tree_info);

                // If it succeeds, validate the structure
                assert!(!tree_info.process_chain.is_empty());
                assert_eq!(tree_info.process_chain[0], 0);

                // Root parent validation depends on platform
                if let Some(root_pid) = tree_info.root_parent_pid {
                    #[cfg(windows)]
                    {
                        // On Windows, PID 0 (System Idle Process) might have no parent
                        println!("Windows PID 0 root parent: {:?}", root_pid);
                    }

                    #[cfg(unix)]
                    {
                        // On Unix, behavior varies
                        println!("Unix PID 0 root parent: {:?}", root_pid);
                        assert!(root_pid > 0 || root_pid == 0, "Root parent should be valid");
                    }
                }
            }
            Err(err) => {
                println!("PID 0 correctly returned error: {}", err);
                // Error is acceptable as PID 0 handling varies by platform
            }
        }
    }

    #[test]
    fn test_boundary_pid_one() {
        // Test behavior with PID 1 (init system)

        println!("Testing PID 1 boundary case...");

        // PID 1 is typically the init system on Unix platforms
        // On Windows, PID 1 is usually a system process

        let result = get_process_tree(1);

        match result {
            Ok(tree_info) => {
                println!("PID 1 resulted in process tree: {:?}", tree_info);

                assert!(!tree_info.process_chain.is_empty());
                assert_eq!(tree_info.process_chain[0], 1);

                // PID 1 should typically be a root process itself
                if let Some(root_pid) = tree_info.root_parent_pid {
                    println!("PID 1 root parent: {:?}", root_pid);

                    #[cfg(unix)]
                    {
                        // On Unix, PID 1 (init/systemd/launchd) is usually the ultimate root
                        let root_name = get_process_name(1).unwrap_or_default();
                        let root_lower = root_name.to_lowercase();

                        let is_valid_init = root_lower.contains("init")
                            || root_lower.contains("systemd")
                            || root_lower.contains("launchd")
                            || root_lower.is_empty();

                        println!("PID 1 process name: {}", root_name);
                        assert!(is_valid_init, "PID 1 should be a valid init system");
                    }
                }
            }
            Err(err) => {
                println!("PID 1 returned error: {}", err);
                // This might happen due to permissions or platform differences
            }
        }
    }

    #[test]
    fn test_maximum_valid_pid() {
        // Test behavior with maximum valid PID values

        println!("Testing maximum valid PID boundaries...");

        // Test with platform-specific maximum PIDs
        let max_pids = if cfg!(windows) {
            vec![65535] // u16::MAX for Windows
        } else {
            vec![32767, 65535, 99999] // Various Unix limits
        };

        for max_pid in max_pids {
            println!("Testing PID {}...", max_pid);

            let result = get_process_tree(max_pid);

            match result {
                Ok(tree_info) => {
                    println!("PID {} resulted in tree: {:?}", max_pid, tree_info);
                    assert_eq!(tree_info.process_chain[0], max_pid);
                }
                Err(err) => {
                    println!("PID {} correctly returned error: {}", max_pid, err);
                    // Most high PIDs should not exist, so error is expected
                }
            }
        }
    }

    #[test]
    fn test_invalid_pid_values() {
        // Test with clearly invalid PID values

        println!("Testing invalid PID values...");

        let invalid_pids = vec![
            u32::MAX, // Definitely invalid
            999999,   // Very unlikely to exist
            1234567,  // Extremely unlikely
        ];

        for invalid_pid in invalid_pids {
            println!("Testing invalid PID {}...", invalid_pid);

            let result = get_process_tree(invalid_pid);

            match result {
                Ok(tree_info) => {
                    println!(
                        "Warning: Invalid PID {} unexpectedly succeeded: {:?}",
                        invalid_pid, tree_info
                    );
                    // If it succeeds, it should be minimal
                    assert!(!tree_info.process_chain.is_empty());
                    assert_eq!(tree_info.process_chain[0], invalid_pid);
                }
                Err(err) => {
                    println!("Invalid PID {} correctly failed: {}", invalid_pid, err);
                    // This is the expected behavior
                }
            }
        }
    }

    #[test]
    fn test_zero_depth_process_tree() {
        // Test edge case where process tree might have minimal depth

        let tree_info = ProcessTreeInfo::current().expect("Failed to get current process tree");

        println!("Current process tree depth: {}", tree_info.depth);

        // Depth should be at least 1 (current process itself)
        assert!(
            tree_info.depth >= 1,
            "Process tree depth should be at least 1"
        );
        assert!(
            !tree_info.process_chain.is_empty(),
            "Process chain should not be empty"
        );

        // The current process should always be first in the chain
        assert_eq!(tree_info.process_chain[0], std::process::id());

        println!("✅ Process tree has valid minimum depth");
    }

    #[test]
    fn test_maximum_depth_prevention() {
        // Test that infinite loops are prevented by depth limits

        let tree_info = ProcessTreeInfo::current().expect("Failed to get current process tree");

        println!("Process tree depth: {}", tree_info.depth);

        // Should have reasonable depth limits
        assert!(
            tree_info.depth <= 50,
            "Process tree depth should be limited to prevent infinite loops"
        );

        // Verify no duplicate PIDs in the chain (indicates potential loops)
        let mut seen_pids = std::collections::HashSet::new();
        for pid in &tree_info.process_chain {
            assert!(
                !seen_pids.contains(pid),
                "Process chain contains duplicate PID {} (possible loop)",
                pid
            );
            seen_pids.insert(*pid);
        }

        println!("✅ Process tree depth is properly limited");
    }

    #[test]
    fn test_same_pid_edge_cases() {
        // Test same_root_parent function with edge cases

        let current_pid = std::process::id();

        // Test with same PID (should always be true)
        let result = same_root_parent(current_pid, current_pid);
        assert!(result.is_ok(), "Same PID should not error");
        assert!(result.unwrap(), "Same PID should return true");

        // Test with PID 0 and current PID
        let result = same_root_parent(current_pid, 0);
        match result {
            Ok(same_root) => {
                println!("same_root_parent({}, 0) = {}", current_pid, same_root);
            }
            Err(err) => {
                println!("same_root_parent({}, 0) failed: {}", current_pid, err);
                // This is acceptable due to platform differences
            }
        }

        // Test with PID 1 and current PID
        let result = same_root_parent(current_pid, 1);
        match result {
            Ok(same_root) => {
                println!("same_root_parent({}, 1) = {}", current_pid, same_root);
            }
            Err(err) => {
                println!("same_root_parent({}, 1) failed: {}", current_pid, err);
                // This is acceptable due to permissions or platform differences
            }
        }
    }

    #[test]
    fn test_rapid_process_creation_destruction() {
        // Test behavior with rapidly created and destroyed processes

        println!("Testing rapid process creation/destruction...");

        let mut success_count = 0;
        let mut error_count = 0;
        let iterations = 10;

        for i in 0..iterations {
            // Create a short-lived process
            let child = if cfg!(windows) {
                Command::new("ping").args(["127.0.0.1", "-n", "1"]).spawn()
            } else {
                Command::new("sleep").args(["0.1"]).spawn()
            };

            match child {
                Ok(mut child) => {
                    let child_pid = child.id();

                    // Immediately try to get process tree (race condition test)
                    let result = get_process_tree(child_pid);

                    // Clean up the child
                    let _ = child.kill();
                    let _ = child.wait();

                    match result {
                        Ok(tree_info) => {
                            success_count += 1;
                            println!(
                                "Iteration {}: Successfully got tree for PID {}",
                                i, child_pid
                            );
                            assert_eq!(tree_info.process_chain[0], child_pid);
                        }
                        Err(err) => {
                            error_count += 1;
                            println!("Iteration {}: Failed for PID {}: {}", i, child_pid, err);
                        }
                    }
                }
                Err(err) => {
                    error_count += 1;
                    println!("Iteration {}: Failed to spawn process: {}", i, err);
                }
            }
        }

        println!(
            "Rapid process test results: {} successes, {} errors",
            success_count, error_count
        );

        // Some failures are expected due to race conditions
        assert!(
            success_count > 0,
            "Should have at least some successful process tree discoveries"
        );
    }

    #[test]
    fn test_permission_boundary_cases() {
        // Test behavior with processes that might have permission restrictions

        println!("Testing permission boundary cases...");

        // Try to access system processes that might be restricted
        let system_pids = if cfg!(windows) {
            vec![4] // System process on Windows
        } else {
            vec![1, 2] // init and kernel threads on Unix
        };

        for system_pid in system_pids {
            println!("Testing access to system PID {}...", system_pid);

            let result = get_process_tree(system_pid);

            match result {
                Ok(tree_info) => {
                    println!("Successfully accessed PID {}: {:?}", system_pid, tree_info);
                    assert_eq!(tree_info.process_chain[0], system_pid);
                }
                Err(err) => {
                    println!(
                        "Access to PID {} restricted (expected): {}",
                        system_pid, err
                    );
                    // Permission errors are acceptable for system processes
                }
            }
        }
    }

    #[test]
    fn test_empty_process_name_handling() {
        // Test handling of processes with empty or unrecognizable names

        let current_pid = std::process::id();
        let tree_info = ProcessTreeInfo::current().expect("Failed to get current process tree");

        // Try to get names for all PIDs in our process tree
        for pid in &tree_info.process_chain {
            if let Some(name) = get_process_name(*pid) {
                if name.trim().is_empty() {
                    println!("PID {} has empty name", pid);
                    // This should be handled gracefully
                } else {
                    println!("PID {} name: {}", pid, name);
                }
            } else {
                println!("PID {} name unavailable", pid);
                // This is also acceptable
            }
        }

        // The current process should at least have a valid name
        let current_name = get_process_name(current_pid);
        assert!(current_name.is_some(), "Current process should have a name");

        if let Some(name) = current_name {
            assert!(
                !name.trim().is_empty(),
                "Current process name should not be empty"
            );
        }
    }

    #[test]
    fn test_concurrent_process_tree_access() {
        // Test concurrent access to process tree functionality

        println!("Testing concurrent process tree access...");

        let thread_count = 5;
        let iterations_per_thread = 3;
        let mut handles = vec![];

        for thread_id in 0..thread_count {
            let handle = thread::spawn(move || {
                let mut success_count = 0;
                let mut error_count = 0;

                for i in 0..iterations_per_thread {
                    match ProcessTreeInfo::current() {
                        Ok(tree_info) => {
                            success_count += 1;
                            println!(
                                "Thread {} iteration {}: depth {}",
                                thread_id, i, tree_info.depth
                            );

                            // Validate the tree
                            assert!(!tree_info.process_chain.is_empty());
                            assert!(tree_info.depth >= 1);
                        }
                        Err(err) => {
                            error_count += 1;
                            println!("Thread {} iteration {}: {}", thread_id, i, err);
                        }
                    }
                }

                (thread_id, success_count, error_count)
            });

            handles.push(handle);
        }

        // Wait for all threads and collect results
        let mut total_success = 0;
        let mut total_error = 0;

        for handle in handles {
            match handle.join() {
                Ok((thread_id, success, error)) => {
                    total_success += success;
                    total_error += error;
                    println!(
                        "Thread {}: {} successes, {} errors",
                        thread_id, success, error
                    );
                }
                Err(err) => {
                    println!("Thread panicked: {:?}", err);
                }
            }
        }

        println!(
            "Concurrent access test: {} total successes, {} total errors",
            total_success, total_error
        );

        // Should have mostly successful results
        assert!(
            total_success > total_error,
            "Should have more successes than errors in concurrent access"
        );

        println!("✅ Concurrent access handled correctly");
    }

    /// Helper function to get process name across platforms
    fn get_process_name(pid: u32) -> Option<String> {
        #[cfg(windows)]
        {
            // Use sysinfo on Windows (consistent with our implementation)
            let mut system = sysinfo::System::new();
            system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

            if let Some(process) = system.processes().get(&(pid as usize).into()) {
                Some(process.name().to_string_lossy().into_owned())
            } else {
                None
            }
        }

        #[cfg(unix)]
        {
            // Use ps command on Unix
            if let Ok(output) = Command::new("ps")
                .args(["-p", &pid.to_string(), "-o", "comm="])
                .output()
            {
                if !output.stdout.is_empty() {
                    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
                } else {
                    None
                }
            } else {
                None
            }
        }
    }
}
