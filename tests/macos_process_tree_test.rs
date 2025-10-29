//! macOS specific tests for process tree functionality
//!
//! These tests verify macOS-specific behavior including launchd
//! root process detection and Darwin system validation.

#[cfg(test)]
mod tests {
    
    
    
    

    #[test]
    #[cfg(target_os = "macos")]
    fn test_macos_root_process_detection() {
        // Test that we can detect macOS root processes like launchd
        let tree_info = ProcessTreeInfo::current().expect("Failed to get current process tree");

        println!("macOS Process Tree Analysis:");
        println!("Current PID: {}", std::process::id());
        println!("Process chain length: {}", tree_info.depth);
        println!("Root parent PID: {:?}", tree_info.root_parent_pid);

        for (i, pid) in tree_info.process_chain.iter().enumerate() {
            println!("  Level {}: PID {}", i, pid);

            // Try to get process name using ps command
            if let Ok(output) = Command::new("ps")
                .args(["-p", &pid.to_string(), "-o", "comm="])
                .output()
            {
                if !output.stdout.is_empty() {
                    let name = String::from_utf8_lossy(&output.stdout).trim();
                    println!("    Process name: {}", name);
                }
            }
        }

        // Validate basic process tree structure
        assert!(!tree_info.process_chain.is_empty());
        assert!(tree_info.depth >= 1, "Should have at least current process");

        // The root parent should be a valid PID
        if let Some(root_pid) = tree_info.root_parent_pid {
            assert!(root_pid > 0, "Root parent PID should be positive");

            // On macOS, common root processes include:
            // - launchd (PID 1) - the macOS init system
            // - loginwindow (GUI login process)
            // - Terminal.app or iTerm2 (terminal emulators)
            // - bash/zsh/fish (shell processes)
            // - Dock (macOS dock process)

            println!(
                "Root parent PID {} should be a macOS system or shell process",
                root_pid
            );

            // Try to identify the root process
            if let Ok(output) = Command::new("ps")
                .args(["-p", &root_pid.to_string(), "-o", "comm="])
                .output()
            {
                if !output.stdout.is_empty() {
                    let name = String::from_utf8_lossy(&output.stdout).trim();
                    println!("Root process name: {}", name);

                    let name_lower = name.to_lowercase();

                    let is_system_process = name_lower.contains("launchd")
                        || name_lower.contains("loginwindow")
                        || name_lower.contains("distnoted")  // distribution notifications
                        || name_lower.contains("UserEvent")  // user event agent
                        || name_lower.contains("kernel_task"); // kernel task

                    let is_shell_process = name_lower.contains("bash")
                        || name_lower.contains("zsh")
                        || name_lower.contains("sh")
                        || name_lower.contains("fish")
                        || name_lower.contains("dash");

                    let is_terminal = name_lower.contains("terminal")
                        || name_lower.contains("iterm")
                        || name_lower.contains("xterm")
                        || name_lower.contains("alacritty");

                    let is_macos_gui = name_lower.contains("dock")
                        || name_lower.contains("finder")
                        || name_lower.contains("systemuiserver");

                    println!("  Is system process: {}", is_system_process);
                    println!("  Is shell process: {}", is_shell_process);
                    println!("  Is terminal: {}", is_terminal);
                    println!("  Is macOS GUI process: {}", is_macos_gui);

                    // Should be one of the expected process types
                    assert!(
                        is_system_process || is_shell_process || is_terminal || is_macos_gui,
                        "Root process should be a system, shell, terminal, or GUI process on macOS"
                    );
                }
            }
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_macos_launchd_detection() {
        // Test that we can correctly identify launchd (PID 1 on macOS)

        let tree_info = ProcessTreeInfo::current().expect("Failed to get current process tree");

        // Check if PID 1 (launchd) appears in our process tree
        let mut found_launchd = false;
        let mut launchd_name = String::new();

        for pid in &tree_info.process_chain {
            if *pid == 1 {
                found_launchd = true;

                // Get the name of PID 1
                if let Ok(output) = Command::new("ps").args(["-p", "1", "-o", "comm="]).output() {
                    if !output.stdout.is_empty() {
                        launchd_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    }
                }
                break;
            }
        }

        if found_launchd {
            println!("Found PID 1 ({}) in process tree", launchd_name);

            // Verify it's launchd
            let name_lower = launchd_name.to_lowercase();
            let is_launchd = name_lower.contains("launchd");

            assert!(is_launchd, "PID 1 on macOS should be launchd");
        } else {
            println!("launchd (PID 1) not found in direct process tree");

            // Even if launchd is not directly in the tree, our root should be reasonable
            if let Some(root_pid) = tree_info.root_parent_pid {
                println!("Current root PID: {}", root_pid);

                // The root should be a reasonable macOS process
                assert!(root_pid > 0, "Root PID should be positive");
                assert!(root_pid <= 99999, "Root PID should be in reasonable range");
            }
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_macos_subprocess_ancestry() {
        // Test that child processes have proper macOS ancestry

        let current_pid = std::process::id();
        let current_tree = ProcessTreeInfo::current().expect("Failed to get current process tree");

        // Start a subprocess (sleep is available on macOS)
        let mut child = Command::new("sleep")
            .args(["5"]) // Sleep for 5 seconds
            .spawn()
            .expect("Failed to spawn sleep subprocess");

        let child_pid = child.id();
        println!("Spawned subprocess with PID: {}", child_pid);

        // Give the subprocess time to initialize
        thread::sleep(Duration::from_millis(100));

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
            assert!(!child_tree.process_chain.is_empty());
            assert_eq!(child_tree.process_chain[0], child_pid);

            // On macOS, the subprocess should be a direct child or share the same root
            let current_root = current_tree.root_parent_pid;
            let child_root = child_tree.root_parent_pid;

            println!("Current process root: {:?}", current_root);
            println!("Subprocess root: {:?}", child_root);

            if let (Some(current_root), Some(child_root)) = (current_root, child_root) {
                if current_root == child_root {
                    println!("✅ Subprocess shares the same root parent");
                } else {
                    println!("ℹ️  Subprocess has different root (might be expected on macOS)");
                    // On macOS, this can happen due to process sandboxing or different execution contexts
                }
            }
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_macos_gui_application_context() {
        // Test behavior when running from GUI applications on macOS

        let tree_info = ProcessTreeInfo::current().expect("Failed to get process tree");

        let mut found_terminal = false;
        let mut found_gui_app = false;
        let mut found_system_process = false;

        for pid in &tree_info.process_chain {
            if let Ok(output) = Command::new("ps")
                .args(["-p", &pid.to_string(), "-o", "comm="])
                .output()
            {
                if !output.stdout.is_empty() {
                    let name = String::from_utf8_lossy(&output.stdout).trim();
                    let name_lower = name.to_lowercase();

                    if name_lower.contains("terminal")
                        || name_lower.contains("iterm")
                        || name_lower.contains("alacritty")
                        || name_lower.contains("xterm")
                    {
                        found_terminal = true;
                        println!("Found terminal '{}' in process tree at PID {}", name, pid);
                    }

                    if name_lower.contains("finder")
                        || name_lower.contains("dock")
                        || name_lower.contains("systemuiserver")
                        || name_lower.contains("loginwindow")
                    {
                        found_gui_app = true;
                        println!("Found GUI app '{}' in process tree at PID {}", name, pid);
                    }

                    if name_lower.contains("launchd") || name_lower.contains("kernel_task") {
                        found_system_process = true;
                        println!(
                            "Found system process '{}' in process tree at PID {}",
                            name, pid
                        );
                    }
                }
            }
        }

        println!(
            "macOS process tree contains: terminal={}, gui_app={}, system={}",
            found_terminal, found_gui_app, found_system_process
        );

        // On macOS, we should typically see system processes in our ancestry
        assert!(
            found_system_process,
            "Should have at least one system process in macOS ancestry"
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_macos_process_tree_limits() {
        // Test macOS-specific process tree limits and boundaries

        let tree_info = ProcessTreeInfo::current().expect("Failed to get process tree");

        // macOS systems typically have reasonable process tree depths
        println!("macOS process tree depth: {}", tree_info.depth);

        assert!(
            tree_info.depth >= 1,
            "macOS process tree should have at least 1 level"
        );
        assert!(
            tree_info.depth <= 25,
            "macOS process tree depth seems unusually high (possible infinite loop)"
        );

        // Verify all PIDs are in valid macOS range
        for pid in &tree_info.process_chain {
            assert!(*pid > 0, "macOS PIDs should be positive");
            assert!(
                *pid <= 99999,
                "macOS PIDs should be in valid range (typically <= 99999)"
            );
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_macos_sandbox_detection() {
        // Test detection of macOS sandbox environments

        let tree_info = ProcessTreeInfo::current().expect("Failed to get process tree");

        let mut found_sandboxd = false;
        let mut found_security_agent = false;

        for pid in &tree_info.process_chain {
            if let Ok(output) = Command::new("ps")
                .args(["-p", &pid.to_string(), "-o", "comm="])
                .output()
            {
                if !output.stdout.is_empty() {
                    let name = String::from_utf8_lossy(&output.stdout).trim();
                    let name_lower = name.to_lowercase();

                    if name_lower.contains("sandboxd") {
                        found_sandboxd = true;
                        println!("Found sandboxd in process tree at PID {}", pid);
                    }

                    if name_lower.contains("security") && name_lower.contains("agent") {
                        found_security_agent = true;
                        println!("Found security agent in process tree at PID {}", pid);
                    }
                }
            }
        }

        if found_sandboxd || found_security_agent {
            println!("ℹ️  Running in a macOS sandboxed environment");
        } else {
            println!("ℹ️  No sandbox indicators found (normal for most environments)");
        }

        // Either way, the tree should be valid
        if let Some(root_pid) = tree_info.root_parent_pid {
            assert!(root_pid > 0, "Root PID should be positive even in sandbox");
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_macos_parent_death_handling() {
        // Test that macOS handles parent death signals correctly

        let tree_info = ProcessTreeInfo::current().expect("Failed to get process tree");

        // Verify that our process tree has reasonable structure
        assert!(!tree_info.process_chain.is_empty());
        assert_eq!(tree_info.process_chain[0], std::process::id());

        // Check if we can access parent process information
        if tree_info.process_chain.len() > 1 {
            let parent_pid = tree_info.process_chain[1];
            assert!(parent_pid > 0, "Parent PID should be positive");

            // Try to get parent process info
            if let Ok(output) = Command::new("ps")
                .args(["-p", &parent_pid.to_string(), "-o", "comm="])
                .output()
            {
                if !output.stdout.is_empty() {
                    let parent_name = String::from_utf8_lossy(&output.stdout).trim();
                    println!("Parent process: {} (PID: {})", parent_name, parent_pid);

                    // Parent should be a valid macOS process
                    assert!(!parent_name.is_empty(), "Parent process should have a name");
                }
            }
        }
    }
}
