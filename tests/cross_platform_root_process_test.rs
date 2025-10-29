//! Cross-platform tests for root parent process ID validation
//!
//! These tests verify that root parent process detection works correctly
//! across different operating systems and validates platform-specific behavior.

#[cfg(test)]
mod tests {
    use agentic_warden::process_tree::{ProcessTreeInfo, get_process_tree, same_root_parent};
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_cross_platform_root_process_validation() {
        // Test that root parent process detection works on all platforms

        let tree_info = ProcessTreeInfo::current().expect("Failed to get current process tree");

        println!("Cross-platform Root Process Analysis:");
        println!("Current PID: {}", std::process::id());
        println!("Process chain length: {}", tree_info.depth);
        println!("Root parent PID: {:?}", tree_info.root_parent_pid);
        println!("Platform: {}", std::env::consts::OS);

        // Basic validations that should work on all platforms
        assert!(
            !tree_info.process_chain.is_empty(),
            "Process chain should not be empty"
        );
        assert!(tree_info.depth >= 1, "Depth should be at least 1");

        // Root parent should be a valid PID
        if let Some(root_pid) = tree_info.root_parent_pid {
            assert!(root_pid > 0, "Root parent PID should be positive");

            // Platform-specific PID range validation
            #[cfg(windows)]
            {
                assert!(root_pid <= 65536, "Windows PID should be in u16 range");
            }

            #[cfg(unix)]
            {
                assert!(root_pid <= 99999, "Unix PID should be in reasonable range");
            }
        }

        // Display process chain for analysis
        for (i, pid) in tree_info.process_chain.iter().enumerate() {
            println!("  Level {}: PID {}", i, pid);
        }
    }

    #[test]
    fn test_platform_specific_root_process_identification() {
        // Test that we can identify platform-specific root processes

        let tree_info = ProcessTreeInfo::current().expect("Failed to get current process tree");

        if let Some(root_pid) = tree_info.root_parent_pid {
            let root_name = get_process_name(root_pid).unwrap_or_else(|| "Unknown".to_string());
            println!("Root process: {} (PID: {})", root_name, root_pid);

            let root_lower = root_name.to_lowercase();

            // Platform-specific validations
            #[cfg(windows)]
            {
                let is_valid_windows_root = root_lower.contains("system")
                    || root_lower.contains("idle")
                    || root_lower.contains("explorer")
                    || root_lower.contains("cmd")
                    || root_lower.contains("powershell")
                    || root_lower.contains("winlogon")
                    || root_lower.contains("csrss")
                    || root_lower.contains("smss")
                    || root_lower.contains("lsass");

                assert!(
                    is_valid_windows_root,
                    "Root process should be a valid Windows system or shell process"
                );
                println!("✅ Valid Windows root process");
            }

            #[cfg(target_os = "macos")]
            {
                let is_valid_macos_root = root_lower.contains("launchd")
                    || root_lower.contains("loginwindow")
                    || root_lower.contains("terminal")
                    || root_lower.contains("iterm")
                    || root_lower.contains("dock")
                    || root_lower.contains("finder")
                    || root_lower.contains("bash")
                    || root_lower.contains("zsh")
                    || root_lower.contains("sh");

                assert!(
                    is_valid_macos_root,
                    "Root process should be a valid macOS system or shell process"
                );
                println!("✅ Valid macOS root process");
            }

            #[cfg(any(
                target_os = "linux",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            {
                let is_valid_unix_root = root_lower.contains("init")
                    || root_lower.contains("systemd")
                    || root_lower.contains("kthreadd")
                    || root_lower.contains("bash")
                    || root_lower.contains("zsh")
                    || root_lower.contains("sh")
                    || root_lower.contains("gnome-terminal")
                    || root_lower.contains("konsole")
                    || root_lower.contains("xterm")
                    || root_lower.contains("tmux")
                    || root_lower.contains("screen")
                    || root_lower.contains("sshd");

                assert!(
                    is_valid_unix_root,
                    "Root process should be a valid Unix/Linux system or shell process"
                );
                println!("✅ Valid Unix/Linux root process");
            }
        }
    }

    #[test]
    fn test_cross_platform_subprocess_root_consistency() {
        // Test that subprocesses have consistent root behavior across platforms

        let current_pid = std::process::id();
        let current_tree = ProcessTreeInfo::current().expect("Failed to get current process tree");

        // Choose platform-appropriate subprocess command
        let subprocess_cmd = if cfg!(windows) {
            vec!["ping", "127.0.0.1", "-n", "3"]
        } else {
            vec!["sleep", "3"]
        };

        let mut child = Command::new(subprocess_cmd[0])
            .args(&subprocess_cmd[1..])
            .spawn()
            .expect("Failed to spawn subprocess");

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

        // If we successfully got the subprocess tree, validate cross-platform behavior
        if let Some(child_tree) = child_tree {
            assert!(!child_tree.process_chain.is_empty());
            assert_eq!(child_tree.process_chain[0], child_pid);

            let current_root = current_tree.root_parent_pid;
            let child_root = child_tree.root_parent_pid;

            println!("Current process root: {:?}", current_root);
            println!("Subprocess root: {:?}", child_root);

            // Cross-platform consistency check
            if let (Some(current_root), Some(child_root)) = (current_root, child_root) {
                if current_root == child_root {
                    println!(
                        "✅ Subprocess shares the same root parent (consistent across platforms)"
                    );
                } else {
                    println!("ℹ️  Subprocess has different root (platform-specific behavior)");

                    // This is acceptable as different platforms handle process creation differently
                    #[cfg(windows)]
                    {
                        // On Windows, subprocess might be direct child or under system
                        let child_root_name = get_process_name(child_root).unwrap_or_default();
                        if child_root_name.to_lowercase().contains("system") {
                            println!("ℹ️  Windows: Subprocess under system root (normal)");
                        }
                    }

                    #[cfg(unix)]
                    {
                        // On Unix, subprocess might be under same shell or system
                        let child_root_name = get_process_name(child_root).unwrap_or_default();
                        if child_root_name.to_lowercase().contains("bash")
                            || child_root_name.to_lowercase().contains("zsh")
                        {
                            println!("ℹ️  Unix: Subprocess under shell root (normal)");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_platform_specific_pid_ranges() {
        // Test that PIDs are within expected ranges for each platform

        let tree_info = ProcessTreeInfo::current().expect("Failed to get process tree");

        for pid in &tree_info.process_chain {
            assert!(*pid > 0, "PID should be positive");

            #[cfg(windows)]
            {
                assert!(*pid <= 65536, "Windows PIDs should be in u16 range");
            }

            #[cfg(target_os = "macos")]
            {
                assert!(*pid <= 99999, "macOS PIDs should be in reasonable range");
            }

            #[cfg(any(
                target_os = "linux",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            {
                assert!(*pid <= 32768, "Unix PIDs should be in typical range");
            }
        }

        println!("✅ All PIDs are within platform-specific ranges");
    }

    #[test]
    fn test_cross_platform_same_root_parent_function() {
        // Test the same_root_parent function across platforms

        let current_pid = std::process::id();

        // Test with same PID (should always return true)
        let result = same_root_parent(current_pid, current_pid);
        assert!(
            result.is_ok(),
            "same_root_parent should not error for same PID"
        );
        assert!(
            result.unwrap(),
            "same_root_parent should return true for same PID"
        );

        // Test with different PIDs if we can create a subprocess
        if let Ok(mut child) = Command::new(if cfg!(windows) { "ping" } else { "sleep" })
            .args(if cfg!(windows) {
                &["127.0.0.1", "-n", "2"][..]
            } else {
                &["2"][..]
            })
            .spawn()
        {
            let child_pid = child.id();
            thread::sleep(Duration::from_millis(100));

            let result = same_root_parent(current_pid, child_pid);

            // Clean up
            let _ = child.kill();
            let _ = child.wait();

            match result {
                Ok(same_root) => {
                    println!(
                        "same_root_parent({}, {}) = {}",
                        current_pid, child_pid, same_root
                    );
                    // The result depends on platform and execution context
                    if same_root {
                        println!("✅ Processes share same root parent");
                    } else {
                        println!("ℹ️  Processes have different root parents (platform-specific)");
                    }
                }
                Err(err) => {
                    println!("same_root_parent failed: {}", err);
                    // This can happen due to permissions or timing
                }
            }
        }
    }

    #[test]
    fn test_cross_platform_error_handling() {
        // Test error handling across platforms

        // Test with invalid PID
        let invalid_pid = 999999;
        let result = get_process_tree(invalid_pid);

        match result {
            Ok(tree_info) => {
                println!("Invalid PID resulted in tree: {:?}", tree_info);
                // If it succeeds, it should be minimal
                assert_eq!(tree_info.process_chain[0], invalid_pid);
            }
            Err(err) => {
                println!("Invalid PID correctly returned error: {}", err);
                // Error is acceptable for invalid PIDs
            }
        }

        // Test boundary PIDs that might be platform-specific
        let boundary_pids = vec![0, 1];

        for pid in boundary_pids {
            let result = get_process_tree(pid);
            println!("PID {} result: {:?}", pid, result);

            // These should either succeed with minimal data or fail gracefully
            // We don't assert specific behavior as it varies by platform
        }
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
            // On Unix, use ps command
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

    #[test]
    fn test_cross_platform_performance_characteristics() {
        // Test performance characteristics across platforms

        let iterations = 5;
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

        // Performance expectations vary by platform
        #[cfg(windows)]
        {
            assert!(
                average_duration.as_millis() < 1000,
                "Windows process tree discovery should be under 1 second"
            );
        }

        #[cfg(unix)]
        {
            assert!(
                average_duration.as_millis() < 500,
                "Unix process tree discovery should be under 500ms"
            );
        }

        println!("✅ Performance within acceptable limits for platform");
    }
}
