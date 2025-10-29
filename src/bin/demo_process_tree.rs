//! Process Tree Demo Utility
//!
//! This is a simple demonstration program that shows the process tree functionality
//! of Codex-Warden. It can be used to verify that process tree detection works correctly
//! on Windows systems.

use agentic_warden::process_tree::{ProcessTreeInfo, get_process_tree};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌳 Codex-Warden Process Tree Demo");
    println!("=====================================");

    // Test 1: Current process tree
    println!("\n1️⃣ Current Process Tree Analysis:");
    match ProcessTreeInfo::current() {
        Ok(tree_info) => {
            println!("✅ Successfully retrieved process tree!");
            println!("   Process Chain: {:?}", tree_info.process_chain);
            println!("   Root Parent PID: {:?}", tree_info.root_parent_pid);
            println!("   Tree Depth: {}", tree_info.depth);

            // Print formatted tree
            println!("\n   📊 Process Tree Structure:");
            for (i, pid) in tree_info.process_chain.iter().enumerate() {
                let indent = "  ".repeat(i);
                println!("   {}├─ PID {}", indent, pid);
                if i == 0 {
                    println!("   {}   (Current process)", indent);
                } else if tree_info.root_parent_pid == Some(*pid) {
                    println!("   {}   (Root parent)", indent);
                }
            }
        }
        Err(err) => {
            println!("❌ Failed to get process tree: {}", err);
            return Err(err.into());
        }
    }

    // Test 2: Process comparison
    println!("\n2️⃣ Process Comparison Test:");
    let current_pid = std::process::id();
    println!(
        "   Process ID: {} (same_root_parent test removed)",
        current_pid
    );
    println!(
        "   Note: same_root_parent function has been removed as it was only used in this demo"
    );

    // Test 3: Subprocess analysis
    println!("\n3️⃣ Subprocess Analysis:");
    println!("   Creating a subprocess to analyze its process tree...");

    let mut child = Command::new("ping")
        .args(["127.0.0.1", "-n", "5"]) // Ping for ~5 seconds
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let child_pid = child.id();
    println!("   Subprocess PID: {}", child_pid);

    // Give subprocess time to initialize
    thread::sleep(Duration::from_millis(200));

    match get_process_tree(child_pid) {
        Ok(subprocess_tree) => {
            println!("✅ Successfully retrieved subprocess process tree!");
            println!("   Subprocess Chain: {:?}", subprocess_tree.process_chain);
            println!(
                "   Subprocess Root Parent: {:?}",
                subprocess_tree.root_parent_pid
            );

            // Check if current process and subprocess share root parent
            if let (Some(current_root), Some(sub_root)) = (
                ProcessTreeInfo::current()?.root_parent_pid,
                subprocess_tree.root_parent_pid,
            ) {
                if current_root == sub_root {
                    println!(
                        "✅ Current process and subprocess share the same root parent: {}",
                        current_root
                    );
                } else {
                    println!("ℹ️  Different root parents:");
                    println!("     Current process root: {}", current_root);
                    println!("     Subprocess root: {}", sub_root);
                }
            }
        }
        Err(err) => {
            println!("⚠️  Could not get subprocess process tree: {}", err);
            println!("   This can happen due to permissions or timing issues");
        }
    }

    // Clean up subprocess
    let _ = child.kill();
    let _ = child.wait();

    // Test 4: Performance test
    println!("\n4️⃣ Performance Test:");
    let start = std::time::Instant::now();
    let iterations = 10;

    for i in 0..iterations {
        match ProcessTreeInfo::current() {
            Ok(_) => {}
            Err(err) => {
                println!("⚠️  Iteration {} failed: {}", i + 1, err);
            }
        }
    }

    let duration = start.elapsed();
    let avg_time = duration / iterations;
    println!("   Completed {} iterations in {:?}", iterations, duration);
    println!("   Average time per process tree discovery: {:?}", avg_time);

    if avg_time.as_millis() < 100 {
        println!("✅ Performance is excellent!");
    } else if avg_time.as_millis() < 500 {
        println!("✅ Performance is good!");
    } else {
        println!("⚠️  Performance could be improved");
    }

    // Test 5: Error handling
    println!("\n5️⃣ Error Handling Test:");
    let invalid_pid = 999999; // Very unlikely to exist

    match get_process_tree(invalid_pid) {
        Ok(tree_info) => {
            println!(
                "ℹ️  Invalid PID resulted in minimal tree: {:?}",
                tree_info.process_chain
            );
        }
        Err(err) => {
            println!("✅ Invalid PID correctly returned error: {}", err);
        }
    }

    println!("\n🎉 Demo completed!");
    println!("   Process tree functionality appears to be working correctly.");
    println!("   You can now use Codex-Warden with process tree isolation enabled.");

    Ok(())
}
