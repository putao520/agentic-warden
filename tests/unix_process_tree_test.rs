//! Unix/Linux specific tests for process tree functionality
//!
//! These tests verify Unix/Linux-specific behavior including init/systemd
//! root process detection and POSIX compliance.

#[cfg(test)]
mod tests {
    
    
    
    

    #[test]
    #[cfg(unix)]
    fn test_unix_root_process_detection() {
        // Test that we can detect Unix/Linux root processes like init/systemd
        let tree_info = ProcessTreeInfo::current().expect("Failed to get current process tree");

        println!("Unix/Linux Process Tree Analysis:");
        println!("Current PID: {}", std::process::id());
        println!("Process chain length: {}", tree_info.depth);
        println!("Root parent PID: {:?}", tree_info.root_parent_pid);

        for (i, pid) in tree_info.process_chain.iter().enumerate() {
            println!("  Level {}: PID {}", i, pid);

            // Try to get process name
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

            // On Unix/Linux, common root processes include:
            // - init (PID 1) on traditional systems
            // - systemd (PID 1) on modern systems
            // - kernel threads (PID 2 on some systems)
            // - shell processes (bash, zsh, etc.)
            // - display managers (gdm, lightdm, etc.)
            // - terminal emulators

            println!(
                "Root parent PID {} should be a Unix/Linux system or shell process",
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

                    let is_system_process = name_lower.contains("init")
                        || name_lower.contains("systemd")
                        || name_lower.contains("kthreadd")  // kernel thread daemon
                        || name_lower.contains("ksoftirqd")
                        || name_lower.contains("migration")
                        || name_lower.contains("rcu_")
                        || name.contains("0"); // kernel threads often have "0" in name

                    let is_shell_process = name_lower.contains("bash")
                        || name_lower.contains("zsh")
                        || name_lower.contains("sh")
                        || name_lower.contains("fish")
                        || name_lower.contains("dash")
                        || name_lower.contains("ksh");

                    let is_display_manager = name_lower.contains("gdm")
                        || name_lower.contains("lightdm")
                        || name_lower.contains("sddm")
                        || name_lower.contains("xdm");

                    let is_terminal = name_lower.contains("gnome-terminal")
                        || name_lower.contains("konsole")
                        || name_lower.contains("xterm")
                        || name_lower.contains("tmux")
                        || name_lower.contains("screen");

                    println!("  Is system process: {}", is_system_process);
                    println!("  Is shell process: {}", is_shell_process);
                    println!("  Is display manager: {}", is_display_manager);
                    println!("  Is terminal: {}", is_terminal);

                    // Should be one of the expected process types
                    assert!(
                        is_system_process || is_shell_process || is_display_manager || is_terminal,
                        "Root process should be a system, shell, display manager, or terminal process"
                    );
                }
            }
        }
    }

    #[test]
    #[cfg(unix)]
    fn test_unix_pid_1_detection() {
        // Test that we can correctly identify PID 1 (init/systemd)

        let tree_info = ProcessTreeInfo::current().expect("Failed to get current process tree");

        // Check if PID 1 appears in our process tree
        let mut found_pid_1 = false;
        let mut pid_1_name = String::new();

        for pid in &tree_info.process_chain {
            if *pid == 1 {
                found_pid_1 = true;

                // Get the name of PID 1
                if let Ok(output) = Command::new("ps").args(["-p", "1", "-o", "comm="]).output() {
                    if !output.stdout.is_empty() {
                        pid_1_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    }
                }
                break;
            }
        }

        if found_pid_1 {
            println!("Found PID 1 ({}) in process tree", pid_1_name);

            // Verify it's a valid init system
            let name_lower = pid_1_name.to_lowercase();
            let is_valid_init = name_lower.contains("init")
                || name_lower.contains("systemd")
                || name_lower.is_empty(); // Some systems show empty for kernel processes

            assert!(is_valid_init, "PID 1 should be init or systemd or similar");
        } else {
            println!("PID 1 not found in direct process tree (might be normal on some systems)");

            // Even if PID 1 is not directly in the tree, our root should be reasonable
            if let Some(root_pid) = tree_info.root_parent_pid {
                println!("Current root PID: {}", root_pid);

                // The root should be a reasonable Unix process
                assert!(root_pid > 0, "Root PID should be positive");
                assert!(root_pid <= 32768, "Root PID should be in reasonable range");
            }
        }
    }

    #[test]
    #[cfg(unix)]
    fn test_unix_subprocess_ancestry() {
        // Test that child processes have proper Unix ancestry

        let current_pid = std::process::id();
        let current_tree = ProcessTreeInfo::current().expect("Failed to get current process tree");

        // Start a subprocess (sleep is available on all Unix systems)
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

            // On Unix, the subprocess should be a direct child or share the same root
            let current_root = current_tree.root_parent_pid;
            let child_root = child_tree.root_parent_pid;

            println!("Current process root: {:?}", current_root);
            println!("Subprocess root: {:?}", child_root);

            if let (Some(current_root), Some(child_root)) = (current_root, child_root) {
                if current_root == child_root {
                    println!("✅ Subprocess shares the same root parent");
                } else {
                    println!("ℹ️  Subprocess has different root (might be expected)");
                    // On Unix, this can happen due to process grouping
                }
            }
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_linux_systemd_detection() {
        // Test systemd detection on Linux systems

        // Check if systemd is running
        if let Ok(output) = Command::new("ps").args(["-p", "1", "-o", "comm="]).output() {
            if !output.stdout.is_empty() {
                let init_name = String::from_utf8_lossy(&output.stdout).trim();

                if init_name.to_lowercase().contains("systemd") {
                    println!("Detected systemd as PID 1");

                    let tree_info = ProcessTreeInfo::current().expect("Failed to get process tree");

                    // On systemd systems, we should often see systemd in our ancestry
                    let mut found_systemd = false;

                    for pid in &tree_info.process_chain {
                        if let Ok(output) = Command::new("ps")
                            .args(["-p", &pid.to_string(), "-o", "comm="])
                            .output()
                        {
                            if !output.stdout.is_empty() {
                                let name = String::from_utf8_lossy(&output.stdout).trim();
                                if name.to_lowercase().contains("systemd") {
                                    found_systemd = true;
                                    println!("Found systemd in process tree at PID {}", pid);
                                    break;
                                }
                            }
                        }
                    }

                    if found_systemd {
                        println!("✅ Systemd found in process ancestry");
                    } else {
                        println!("ℹ️  Systemd not directly in ancestry (might be normal)");
                    }
                } else {
                    println!("System not using systemd (found: {})", init_name);
                }
            }
        }
    }

    #[test]
    #[cfg(unix)]
    fn test_unix_process_tree_limits() {
        // Test Unix-specific process tree limits and boundaries

        let tree_info = ProcessTreeInfo::current().expect("Failed to get process tree");

        // Unix systems typically have reasonable process tree depths
        println!("Unix process tree depth: {}", tree_info.depth);

        assert!(
            tree_info.depth >= 1,
            "Unix process tree should have at least 1 level"
        );
        assert!(
            tree_info.depth <= 20,
            "Unix process tree depth seems unusually high (possible infinite loop)"
        );

        // Verify all PIDs are in valid Unix range
        for pid in &tree_info.process_chain {
            assert!(*pid > 0, "Unix PIDs should be positive");
            assert!(
                *pid <= 32768,
                "Unix PIDs should be in valid range (typically <= 32768)"
            );
        }
    }

    #[test]
    #[cfg(unix)]
    fn test_unix_terminal_session_detection() {
        // Test detection of terminal sessions and shell processes

        let tree_info = ProcessTreeInfo::current().expect("Failed to get current process tree");

        let mut found_shell = false;
        let mut found_terminal = false;
        let mut found_ssh = false;

        for pid in &tree_info.process_chain {
            if let Ok(output) = Command::new("ps")
                .args(["-p", &pid.to_string(), "-o", "comm="])
                .output()
            {
                if !output.stdout.is_empty() {
                    let name = String::from_utf8_lossy(&output.stdout).trim();
                    let name_lower = name.to_lowercase();

                    if name_lower.contains("bash")
                        || name_lower.contains("zsh")
                        || name_lower.contains("sh")
                        || name_lower.contains("fish")
                    {
                        found_shell = true;
                        println!("Found shell '{}' in process tree at PID {}", name, pid);
                    }

                    if name_lower.contains("sshd")
                        || name_lower.contains("tmux")
                        || name_lower.contains("screen")
                    {
                        found_ssh = true;
                        println!(
                            "Found terminal/ssh '{}' in process tree at PID {}",
                            name, pid
                        );
                    }

                    if name_lower.contains("gnome-terminal")
                        || name_lower.contains("konsole")
                        || name_lower.contains("xterm")
                        || name_lower.contains("alacritty")
                    {
                        found_terminal = true;
                        println!(
                            "Found terminal emulator '{}' in process tree at PID {}",
                            name, pid
                        );
                    }
                }
            }
        }

        println!(
            "Process tree contains: shell={}, terminal={}, ssh={}",
            found_shell, found_terminal, found_ssh
        );

        // We should typically see at least a shell in our ancestry on Unix
        let has_terminal_or_shell = found_shell || found_terminal || found_ssh;
        assert!(
            has_terminal_or_shell,
            "Should have at least one terminal or shell process in Unix ancestry"
        );
    }
}
