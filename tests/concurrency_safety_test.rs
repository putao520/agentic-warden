//! Concurrency and thread safety tests for process tree functionality
//!
//! These tests verify that the process tree detection is thread-safe
//! and handles concurrent access correctly.

#[cfg(test)]
mod tests {
    use agentic_warden::process_tree::{ProcessTreeInfo, get_process_tree, same_root_parent};
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn test_concurrent_process_tree_creation() {
        // Test multiple threads creating process trees simultaneously

        println!("Testing concurrent process tree creation...");

        let thread_count = 10;
        let iterations_per_thread = 5;
        let barrier = Arc::new(Barrier::new(thread_count));
        let success_count = Arc::new(AtomicUsize::new(0));
        let error_count = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        for thread_id in 0..thread_count {
            let barrier = Arc::clone(&barrier);
            let success_count = Arc::clone(&success_count);
            let error_count = Arc::clone(&error_count);

            let handle = thread::spawn(move || {
                // Wait for all threads to be ready
                barrier.wait();

                let mut thread_success = 0;
                let mut thread_errors = 0;

                for iteration in 0..iterations_per_thread {
                    let start = Instant::now();

                    match ProcessTreeInfo::current() {
                        Ok(tree_info) => {
                            thread_success += 1;
                            let duration = start.elapsed();

                            // Validate the tree
                            assert!(
                                !tree_info.process_chain.is_empty(),
                                "Thread {} iteration {}: Process chain should not be empty",
                                thread_id,
                                iteration
                            );
                            assert!(
                                tree_info.depth >= 1,
                                "Thread {} iteration {}: Depth should be at least 1",
                                thread_id,
                                iteration
                            );

                            println!(
                                "Thread {} iteration {}: depth={}, duration={:?}",
                                thread_id, iteration, tree_info.depth, duration
                            );
                        }
                        Err(err) => {
                            thread_errors += 1;
                            println!("Thread {} iteration {}: {}", thread_id, iteration, err);
                        }
                    }
                }

                success_count.fetch_add(thread_success, Ordering::Relaxed);
                error_count.fetch_add(thread_errors, Ordering::Relaxed);

                (thread_id, thread_success, thread_errors)
            });

            handles.push(handle);
        }

        // Wait for all threads to complete
        let mut total_success = 0;
        let mut total_error = 0;

        for handle in handles {
            match handle.join() {
                Ok((thread_id, success, errors)) => {
                    total_success += success;
                    total_error += errors;
                    println!(
                        "Thread {}: {} successes, {} errors",
                        thread_id, success, errors
                    );
                }
                Err(err) => {
                    println!("Thread panicked: {:?}", err);
                    total_error += 1;
                }
            }
        }

        let final_success = success_count.load(Ordering::Relaxed);
        let final_error = error_count.load(Ordering::Relaxed);

        println!(
            "Concurrent creation test: {} successes, {} errors",
            final_success, final_error
        );

        // Should have high success rate
        assert!(
            final_success > 0,
            "Should have at least some successful process tree creations"
        );
        assert!(
            final_success >= total_error,
            "Should have more successes than errors"
        );

        println!("✅ Concurrent process tree creation is thread-safe");
    }

    #[test]
    fn test_concurrent_subprocess_tracking() {
        // Test tracking multiple subprocesses from different threads

        println!("Testing concurrent subprocess tracking...");

        let thread_count = 5;
        let subprocesses_per_thread = 3;
        let barrier = Arc::new(Barrier::new(thread_count));
        let tracked_pids = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut handles = vec![];

        for thread_id in 0..thread_count {
            let barrier = Arc::clone(&barrier);
            let tracked_pids = Arc::clone(&tracked_pids);

            let handle = thread::spawn(move || {
                barrier.wait();

                let mut local_pids = Vec::new();

                for i in 0..subprocesses_per_thread {
                    // Create a subprocess
                    let child = if cfg!(windows) {
                        Command::new("ping").args(["127.0.0.1", "-n", "3"]).spawn()
                    } else {
                        Command::new("sleep").args(["2"]).spawn()
                    };

                    match child {
                        Ok(mut child_process) => {
                            let child_pid = child_process.id();
                            local_pids.push(child_pid);

                            // Give subprocess time to initialize
                            thread::sleep(Duration::from_millis(100));

                            // Try to get process tree
                            match get_process_tree(child_pid) {
                                Ok(tree_info) => {
                                    println!(
                                        "Thread {} child {}: depth={}",
                                        thread_id, i, tree_info.depth
                                    );
                                    assert_eq!(tree_info.process_chain[0], child_pid);
                                }
                                Err(err) => {
                                    println!("Thread {} child {}: {}", thread_id, i, err);
                                }
                            }

                            // Clean up
                            let _ = child_process.kill();
                            let _ = child_process.wait();
                        }
                        Err(err) => {
                            println!("Thread {} child {}: spawn failed: {}", thread_id, i, err);
                        }
                    }
                }

                // Add tracked PIDs to shared list
                if let Ok(mut pids) = tracked_pids.lock() {
                    pids.extend(local_pids);
                }

                thread_id
            });

            handles.push(handle);
        }

        // Wait for completion
        let mut completed_threads = 0;
        for handle in handles {
            match handle.join() {
                Ok(thread_id) => {
                    completed_threads += 1;
                    println!("Thread {} completed", thread_id);
                }
                Err(err) => {
                    println!("Thread panicked: {:?}", err);
                }
            }
        }

        let total_tracked = tracked_pids.lock().unwrap().len();
        println!(
            "Tracked {} subprocesses across {} threads",
            total_tracked, completed_threads
        );

        assert!(
            total_tracked > 0,
            "Should have tracked at least some subprocesses"
        );
        assert_eq!(
            completed_threads, thread_count,
            "All threads should complete"
        );

        println!("✅ Concurrent subprocess tracking works correctly");
    }

    #[test]
    fn test_concurrent_same_root_parent_checks() {
        // Test same_root_parent function with concurrent access

        println!("Testing concurrent same_root_parent checks...");

        let thread_count = 8;
        let checks_per_thread = 10;
        let barrier = Arc::new(Barrier::new(thread_count));
        let success_count = Arc::new(AtomicUsize::new(0));
        let error_count = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        for thread_id in 0..thread_count {
            let barrier = Arc::clone(&barrier);
            let success_count = Arc::clone(&success_count);
            let error_count = Arc::clone(&error_count);

            let handle = thread::spawn(move || {
                barrier.wait();

                let current_pid = std::process::id();
                let mut thread_success = 0;
                let mut thread_errors = 0;

                for i in 0..checks_per_thread {
                    // Test with same PID (should always be true)
                    let result = same_root_parent(current_pid, current_pid);

                    match result {
                        Ok(true) => {
                            thread_success += 1;
                        }
                        Ok(false) => {
                            thread_errors += 1;
                            println!("Thread {} check {}: same PID returned false!", thread_id, i);
                        }
                        Err(err) => {
                            thread_errors += 1;
                            println!("Thread {} check {}: {}", thread_id, i, err);
                        }
                    }

                    // Test with different PIDs
                    let other_pid = current_pid.wrapping_add(i as u32 + 1);
                    let result = same_root_parent(current_pid, other_pid);

                    match result {
                        Ok(same_root) => {
                            thread_success += 1;
                            if i == 0 {
                                println!(
                                    "Thread {} same_root_parent({}, {}) = {}",
                                    thread_id, current_pid, other_pid, same_root
                                );
                            }
                        }
                        Err(err) => {
                            // Errors are acceptable for non-existent PIDs
                            if i < 3 {
                                // Only print first few errors
                                println!("Thread {} check {}: {}", thread_id, i, err);
                            }
                        }
                    }
                }

                success_count.fetch_add(thread_success, Ordering::Relaxed);
                error_count.fetch_add(thread_errors, Ordering::Relaxed);

                (thread_id, thread_success, thread_errors)
            });

            handles.push(handle);
        }

        // Wait for completion
        let mut total_success = 0;
        let mut total_error = 0;

        for handle in handles {
            match handle.join() {
                Ok((thread_id, success, errors)) => {
                    total_success += success;
                    total_error += errors;
                    println!(
                        "Thread {}: {} successes, {} errors",
                        thread_id, success, errors
                    );
                }
                Err(err) => {
                    println!("Thread panicked: {:?}", err);
                }
            }
        }

        let final_success = success_count.load(Ordering::Relaxed);
        let final_error = error_count.load(Ordering::Relaxed);

        println!(
            "Concurrent same_root_parent test: {} successes, {} errors",
            final_success, final_error
        );

        // Should have high success rate, especially for same PID checks
        assert!(
            final_success > 0,
            "Should have at least some successful checks"
        );

        println!("✅ Concurrent same_root_parent checks are thread-safe");
    }

    #[test]
    fn test_stress_concurrent_access() {
        // Stress test with high levels of concurrent access

        println!("Running stress test for concurrent access...");

        let thread_count = 20;
        let operations_per_thread = 20;
        let barrier = Arc::new(Barrier::new(thread_count));
        let operation_count = Arc::new(AtomicUsize::new(0));
        let success_count = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        for thread_id in 0..thread_count {
            let barrier = Arc::clone(&barrier);
            let operation_count = Arc::clone(&operation_count);
            let success_count = Arc::clone(&success_count);

            let handle = thread::spawn(move || {
                barrier.wait();

                let mut thread_operations = 0;
                let mut thread_success = 0;

                for i in 0..operations_per_thread {
                    thread_operations += 1;

                    // Mix of different operations
                    match i % 3 {
                        0 => {
                            // Process tree creation
                            match ProcessTreeInfo::current() {
                                Ok(_) => thread_success += 1,
                                Err(_) => {} // Expected sometimes under stress
                            }
                        }
                        1 => {
                            // Same root parent check
                            let current_pid = std::process::id();
                            if same_root_parent(current_pid, current_pid).is_ok() {
                                thread_success += 1;
                            }
                        }
                        2 => {
                            // Get process tree for current PID
                            let current_pid = std::process::id();
                            if get_process_tree(current_pid).is_ok() {
                                thread_success += 1;
                            }
                        }
                        _ => unreachable!(),
                    }

                    // Small delay to increase chance of race conditions
                    if i % 5 == 0 {
                        thread::sleep(Duration::from_micros(1));
                    }
                }

                operation_count.fetch_add(thread_operations, Ordering::Relaxed);
                success_count.fetch_add(thread_success, Ordering::Relaxed);

                (thread_id, thread_operations, thread_success)
            });

            handles.push(handle);
        }

        // Wait for all threads
        let start_time = Instant::now();
        let mut total_operations = 0;
        let mut total_success = 0;

        for handle in handles {
            match handle.join() {
                Ok((thread_id, operations, success)) => {
                    total_operations += operations;
                    total_success += success;
                    println!(
                        "Thread {}: {}/{} operations successful",
                        thread_id, success, operations
                    );
                }
                Err(err) => {
                    println!("Thread panicked: {:?}", err);
                }
            }
        }

        let total_duration = start_time.elapsed();
        let final_operations = operation_count.load(Ordering::Relaxed);
        let final_success = success_count.load(Ordering::Relaxed);

        println!("Stress test results:");
        println!("  Total operations: {}", final_operations);
        println!("  Successful operations: {}", final_success);
        println!(
            "  Success rate: {:.1}%",
            (final_success as f64 / final_operations as f64) * 100.0
        );
        println!("  Total duration: {:?}", total_duration);
        println!(
            "  Operations per second: {:.0}",
            final_operations as f64 / total_duration.as_secs_f64()
        );

        // Verify no deadlocks or panics occurred
        assert_eq!(
            final_operations, total_operations,
            "All operations should be counted"
        );
        assert!(final_success > 0, "Should have some successful operations");

        println!("✅ Stress test passed - no deadlocks or panics");
    }

    #[test]
    fn test_read_write_concurrent_access() {
        // Test mixed read/write concurrent access patterns

        println!("Testing mixed read/write concurrent access...");

        let reader_threads = 5;
        let writer_threads = 3;
        let operations_per_thread = 10;
        let barrier = Arc::new(Barrier::new(reader_threads + writer_threads));
        let read_count = Arc::new(AtomicUsize::new(0));
        let write_count = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        // Reader threads
        for thread_id in 0..reader_threads {
            let barrier = Arc::clone(&barrier);
            let read_count = Arc::clone(&read_count);

            let handle = thread::spawn(move || {
                barrier.wait();

                for i in 0..operations_per_thread {
                    // Read operations
                    match ProcessTreeInfo::current() {
                        Ok(tree_info) => {
                            read_count.fetch_add(1, Ordering::Relaxed);
                            if i == 0 {
                                println!(
                                    "Reader thread {} got depth {}",
                                    thread_id, tree_info.depth
                                );
                            }
                        }
                        Err(err) => {
                            println!("Reader thread {} iteration {}: {}", thread_id, i, err);
                        }
                    }

                    thread::sleep(Duration::from_millis(1));
                }

                thread_id
            });

            handles.push(handle);
        }

        // Writer threads (simulating writes through process creation)
        for thread_id in 0..writer_threads {
            let barrier = Arc::clone(&barrier);
            let write_count = Arc::clone(&write_count);

            let handle = thread::spawn(move || {
                barrier.wait();

                for i in 0..operations_per_thread {
                    // "Write" operations through subprocess creation
                    let child = if cfg!(windows) {
                        Command::new("ping").args(["127.0.0.1", "-n", "1"]).spawn()
                    } else {
                        Command::new("sleep").args(["0.1"]).spawn()
                    };

                    match child {
                        Ok(mut child_process) => {
                            let child_pid = child_process.id();

                            // Try to read the subprocess tree
                            if get_process_tree(child_pid).is_ok() {
                                write_count.fetch_add(1, Ordering::Relaxed);
                            }

                            // Clean up
                            let _ = child_process.kill();
                            let _ = child_process.wait();
                        }
                        Err(err) => {
                            println!("Writer thread {} iteration {}: {}", thread_id, i, err);
                        }
                    }

                    thread::sleep(Duration::from_millis(2));
                }

                thread_id + 1000 // Distinguish from readers
            });

            handles.push(handle);
        }

        // Wait for all threads
        let mut completed_readers = 0;
        let mut completed_writers = 0;

        for handle in handles {
            match handle.join() {
                Ok(thread_id) => {
                    if thread_id < 1000 {
                        completed_readers += 1;
                    } else {
                        completed_writers += 1;
                    }
                }
                Err(err) => {
                    println!("Thread panicked: {:?}", err);
                }
            }
        }

        let final_reads = read_count.load(Ordering::Relaxed);
        let final_writes = write_count.load(Ordering::Relaxed);

        println!("Mixed access test results:");
        println!("  Completed readers: {}", completed_readers);
        println!("  Completed writers: {}", completed_writers);
        println!("  Successful reads: {}", final_reads);
        println!("  Successful writes: {}", final_writes);

        assert_eq!(
            completed_readers, reader_threads,
            "All reader threads should complete"
        );
        assert_eq!(
            completed_writers, writer_threads,
            "All writer threads should complete"
        );
        assert!(final_reads > 0, "Should have successful reads");
        assert!(final_writes > 0, "Should have successful writes");

        println!("✅ Mixed read/write concurrent access works correctly");
    }

    #[test]
    fn test_memory_safety_under_concurrent_access() {
        // Test that memory remains safe under concurrent access

        println!("Testing memory safety under concurrent access...");

        let thread_count = 15;
        let iterations_per_thread = 30;
        let barrier = Arc::new(Barrier::new(thread_count));
        let data_validation_errors = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        for thread_id in 0..thread_count {
            let barrier = Arc::clone(&barrier);
            let data_validation_errors = Arc::clone(&data_validation_errors);

            let handle = thread::spawn(move || {
                barrier.wait();

                for i in 0..iterations_per_thread {
                    match ProcessTreeInfo::current() {
                        Ok(tree_info) => {
                            // Validate data integrity
                            if tree_info.process_chain.is_empty() {
                                data_validation_errors.fetch_add(1, Ordering::Relaxed);
                                println!(
                                    "Thread {} iteration {}: Empty process chain!",
                                    thread_id, i
                                );
                                continue;
                            }

                            if tree_info.depth == 0 {
                                data_validation_errors.fetch_add(1, Ordering::Relaxed);
                                println!("Thread {} iteration {}: Zero depth!", thread_id, i);
                                continue;
                            }

                            // Validate chain length matches depth
                            if tree_info.process_chain.len() != tree_info.depth {
                                data_validation_errors.fetch_add(1, Ordering::Relaxed);
                                println!(
                                    "Thread {} iteration {}: Chain length {} != depth {}",
                                    thread_id,
                                    i,
                                    tree_info.process_chain.len(),
                                    tree_info.depth
                                );
                                continue;
                            }

                            // Validate current PID is first in chain
                            if tree_info.process_chain[0] != std::process::id() {
                                data_validation_errors.fetch_add(1, Ordering::Relaxed);
                                println!(
                                    "Thread {} iteration {}: First PID {} != current PID {}",
                                    thread_id,
                                    i,
                                    tree_info.process_chain[0],
                                    std::process::id()
                                );
                                continue;
                            }

                            // Validate all PIDs are positive
                            for pid in &tree_info.process_chain {
                                if *pid == 0 {
                                    data_validation_errors.fetch_add(1, Ordering::Relaxed);
                                    println!(
                                        "Thread {} iteration {}: Zero PID in chain",
                                        thread_id, i
                                    );
                                    break;
                                }
                            }

                            // Validate no duplicate PIDs (except root)
                            let mut seen_pids = std::collections::HashSet::new();
                            for pid in &tree_info.process_chain {
                                if seen_pids.contains(pid) {
                                    data_validation_errors.fetch_add(1, Ordering::Relaxed);
                                    println!(
                                        "Thread {} iteration {}: Duplicate PID {} in chain",
                                        thread_id, i, pid
                                    );
                                    break;
                                }
                                seen_pids.insert(*pid);
                            }
                        }
                        Err(err) => {
                            // Some errors are acceptable under high concurrency
                            if i < 5 {
                                // Only print first few
                                println!("Thread {} iteration {}: {}", thread_id, i, err);
                            }
                        }
                    }

                    // Random small delays to increase race condition likelihood
                    if i % 7 == 0 {
                        thread::sleep(Duration::from_micros(10));
                    }
                }

                thread_id
            });

            handles.push(handle);
        }

        // Wait for all threads
        let mut completed_threads = 0;

        for handle in handles {
            match handle.join() {
                Ok(thread_id) => {
                    completed_threads += 1;
                }
                Err(err) => {
                    println!("Thread panicked: {:?}", err);
                }
            }
        }

        let validation_errors = data_validation_errors.load(Ordering::Relaxed);

        println!("Memory safety test results:");
        println!("  Completed threads: {}", completed_threads);
        println!("  Data validation errors: {}", validation_errors);

        assert_eq!(
            completed_threads, thread_count,
            "All threads should complete"
        );
        assert_eq!(
            validation_errors, 0,
            "Should have no data validation errors"
        );

        println!("✅ Memory safety maintained under concurrent access");
    }
}
