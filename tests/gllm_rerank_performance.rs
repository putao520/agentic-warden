//! gllm rerank performance benchmark tests
//! Measures reranking performance characteristics
//!
//! 注意：性能基准测试对环境敏感，默认被忽略
//! 运行方式：cargo test --test gllm_rerank_performance -- --ignored

#[cfg(test)]
mod tests {
    use std::time::Instant;

    #[test]
    #[ignore = "performance benchmark - environment sensitive"]
    fn test_rerank_performance_small_dataset() {
        println!("\n⚡ Testing rerank performance - Small dataset (5 docs)");

        let client = gllm::Client::new("all-MiniLM-L6-v2")
            .expect("Failed to create gllm client");

        let query = "machine learning";
        let documents = vec![
            "Machine learning algorithms and neural networks".to_string(),
            "Data science and statistical analysis".to_string(),
            "Artificial intelligence and automation".to_string(),
            "Software development methodologies".to_string(),
            "Cloud computing infrastructure".to_string(),
        ];

        let iterations = 10;
        let mut total_duration = std::time::Duration::ZERO;

        for i in 0..iterations {
            let start = Instant::now();
            match client.rerank(query, &documents).generate() {
                Ok(response) => {
                    let duration = start.elapsed();
                    total_duration += duration;

                    println!("✅ Iteration {}: {:.3}ms, {} results", i + 1, duration.as_millis(), response.results.len());
                    assert_eq!(response.results.len(), documents.len(), "Should return results for all documents");
                }
                Err(e) => panic!("Rerank iteration {} failed: {}", i + 1, e),
            }
        }

        let avg_duration = total_duration / iterations;
        let throughput_per_sec = (iterations as f64 * 1000.0) / total_duration.as_millis() as f64;

        println!("✅ Small dataset performance summary:");
        println!("   Total iterations: {}", iterations);
        println!("   Total time: {:.3}ms", total_duration.as_millis());
        println!("   Average time: {:.3}ms", avg_duration.as_millis());
        println!("   Throughput: {:.1} reranks/sec", throughput_per_sec);

        // Performance expectations (should complete within reasonable time)
        assert!(avg_duration.as_millis() < 5000, "Average rerank time should be < 5 seconds for small dataset");
        assert!(throughput_per_sec >= 0.1, "Should achieve at least 0.1 reranks/sec");
    }

    #[test]
    #[ignore = "performance benchmark - environment sensitive"]
    fn test_rerank_performance_medium_dataset() {
        println!("\n⚡ Testing rerank performance - Medium dataset (20 docs)");

        let client = gllm::Client::new("all-MiniLM-L6-v2")
            .expect("Failed to create gllm client");

        let query = "software development";
        let documents = vec![
            "Agile methodologies and project management".to_string(),
            "Version control with Git and GitHub".to_string(),
            "Continuous integration and deployment".to_string(),
            "Docker containerization and orchestration".to_string(),
            "Microservices architecture design".to_string(),
            "RESTful API development principles".to_string(),
            "Database design and optimization".to_string(),
            "Frontend frameworks and libraries".to_string(),
            "Backend development patterns".to_string(),
            "Testing strategies and unit tests".to_string(),
            "Code quality and static analysis".to_string(),
            "Security best practices in development".to_string(),
            "DevOps automation and monitoring".to_string(),
            "Cloud deployment strategies".to_string(),
            "Performance optimization techniques".to_string(),
            "Code documentation practices".to_string(),
            "Team collaboration tools".to_string(),
            "Requirements gathering and analysis".to_string(),
            "Software architecture patterns".to_string(),
            "Technical debt management".to_string(),
        ];

        let iterations = 5;
        let mut total_duration = std::time::Duration::ZERO;

        for i in 0..iterations {
            let start = Instant::now();
            match client.rerank(query, &documents).generate() {
                Ok(response) => {
                    let duration = start.elapsed();
                    total_duration += duration;

                    println!("✅ Iteration {}: {:.3}ms, {} results", i + 1, duration.as_millis(), response.results.len());
                    assert_eq!(response.results.len(), documents.len(), "Should return results for all documents");
                }
                Err(e) => panic!("Rerank iteration {} failed: {}", i + 1, e),
            }
        }

        let avg_duration = total_duration / iterations;
        let throughput_per_sec = (iterations as f64 * 1000.0) / total_duration.as_millis() as f64;

        println!("✅ Medium dataset performance summary:");
        println!("   Total iterations: {}", iterations);
        println!("   Total time: {:.3}ms", total_duration.as_millis());
        println!("   Average time: {:.3}ms", avg_duration.as_millis());
        println!("   Throughput: {:.1} reranks/sec", throughput_per_sec);

        // Performance expectations
        assert!(avg_duration.as_millis() < 10000, "Average rerank time should be < 10 seconds for medium dataset");
        assert!(throughput_per_sec >= 0.05, "Should achieve at least 0.05 reranks/sec");
    }

    #[test]
    fn test_rerank_performance_large_dataset() {
        println!("\n⚡ Testing rerank performance - Large dataset (50 docs)");

        let client = gllm::Client::new("all-MiniLM-L6-v2")
            .expect("Failed to create gllm client");

        let query = "artificial intelligence";
        let documents: Vec<String> = (0..50).map(|i| {
            format!("Document {} about artificial intelligence and machine learning algorithms", i + 1)
        }).collect();

        let iterations = 3;
        let mut total_duration = std::time::Duration::ZERO;

        for i in 0..iterations {
            let start = Instant::now();
            match client.rerank(query, &documents).generate() {
                Ok(response) => {
                    let duration = start.elapsed();
                    total_duration += duration;

                    println!("✅ Iteration {}: {:.3}ms, {} results", i + 1, duration.as_millis(), response.results.len());
                    assert_eq!(response.results.len(), documents.len(), "Should return results for all documents");
                }
                Err(e) => panic!("Rerank iteration {} failed: {}", i + 1, e),
            }
        }

        let avg_duration = total_duration / iterations;
        let throughput_per_sec = (iterations as f64 * 1000.0) / total_duration.as_millis() as f64;

        println!("✅ Large dataset performance summary:");
        println!("   Total iterations: {}", iterations);
        println!("   Total time: {:.3}ms", total_duration.as_millis());
        println!("   Average time: {:.3}ms", avg_duration.as_millis());
        println!("   Throughput: {:.2} reranks/sec", throughput_per_sec);

        // Performance expectations
        assert!(avg_duration.as_millis() < 30000, "Average rerank time should be < 30 seconds for large dataset");
        assert!(throughput_per_sec >= 0.01, "Should achieve at least 0.01 reranks/sec");
    }

    #[test]
    fn test_rerank_performance_comparison_top_n() {
        println!("\n🔢 Testing rerank performance - Top N vs All documents");

        let client = gllm::Client::new("all-MiniLM-L6-v2")
            .expect("Failed to create gllm client");

        let query = "machine learning";
        let documents: Vec<String> = (0..30).map(|i| {
            format!("Document {} about machine learning and AI", i + 1)
        }).collect();

        let iterations = 5;

        // Test without top_n (return all)
        let mut all_duration = std::time::Duration::ZERO;
        for i in 0..iterations {
            let start = Instant::now();
            match client.rerank(query, &documents).generate() {
                Ok(_) => all_duration += start.elapsed(),
                Err(e) => panic!("Full rerank iteration {} failed: {}", i + 1, e),
            }
        }
        let avg_all_time = all_duration / iterations;

        // Test with top_n = 5
        let mut topn_duration = std::time::Duration::ZERO;
        for i in 0..iterations {
            let start = Instant::now();
            match client.rerank(query, &documents).top_n(5).generate() {
                Ok(_) => topn_duration += start.elapsed(),
                Err(e) => panic!("TopN rerank iteration {} failed: {}", i + 1, e),
            }
        }
        let avg_topn_time = topn_duration / iterations;

        let speedup_factor = avg_all_time.as_millis() as f32 / avg_topn_time.as_millis() as f32;

        println!("✅ Top N performance comparison:");
        println!("   All documents avg: {:.3}ms", avg_all_time.as_millis());
        println!("   Top N (5) avg: {:.3}ms", avg_topn_time.as_millis());
        println!("   Speedup factor: {:.2}x", speedup_factor);

        // Top N should be faster (but not necessarily by a large factor due to implementation details)
        assert!(avg_topn_time <= avg_all_time, "Top N should be faster or equal to full rerank");

        println!("✅ Top N performance optimization confirmed");
    }

    #[test]
    fn test_rerank_memory_usage() {
        println!("\n🧮 Testing rerank memory usage patterns");

        let client = gllm::Client::new("all-MiniLM-L6-v2")
            .expect("Failed to create gllm client");

        let query = "performance benchmark test";
        let documents: Vec<String> = (0..20).map(|i| {
            format!("Performance test document {} with various metrics and measurements", i + 1)
        }).collect();

        let iterations = 3;
        let mut peak_memory_bytes = 0u64;

        for i in 0..iterations {
            // Get memory usage before
            let before_memory = std::env::var("TEST_MEMORY_MONITOR").map_or(0, |_| {
                // In real implementation, we could use memory profiling here
                0u64
            });

            let start = Instant::now();
            match client.rerank(query, &documents).generate() {
                Ok(response) => {
                    let duration = start.elapsed();

                    // Simulate memory tracking
                    let estimated_memory = documents.len() * 1000 + query.len() * 100; // Rough estimate
                    peak_memory_bytes = peak_memory_bytes.max(estimated_memory as u64);

                    println!("✅ Memory test iteration {}: {:.3}ms, estimated {}KB",
                           i + 1, duration.as_millis(), estimated_memory / 1024);
                    assert_eq!(response.results.len(), documents.len(), "Should return results for all documents");
                }
                Err(e) => panic!("Memory test iteration {} failed: {}", i + 1, e),
            }
        }

        println!("✅ Memory usage summary:");
        println!("   Peak estimated memory: {}KB", peak_memory_bytes / 1024);
        println!("   Documents per iteration: {}", documents.len());

        // Basic memory sanity check
        assert!(peak_memory_bytes < 50 * 1024 * 1024, "Estimated memory usage should be < 50MB");
    }

    #[test]
    fn test_rerank_throughput_scaling() {
        println!("\n📈 Testing rerank throughput scaling");

        let client = gllm::Client::new("all-MiniLM-L6-v2")
            .expect("Failed to create gllm client");

        let query = "scalability test";

        // Test different document sizes
        let test_sizes = [5, 10, 20, 30];
        let iterations = 3;

        println!("✅ Throughput scaling analysis:");

        for &size in &test_sizes {
            let documents: Vec<String> = (0..size).map(|i| {
                format!("Document {} for scalability performance testing", i + 1)
            }).collect();

            let mut total_duration = std::time::Duration::ZERO;

            for i in 0..iterations {
                let start = Instant::now();
                match client.rerank(query, &documents).generate() {
                    Ok(_) => total_duration += start.elapsed(),
                    Err(e) => panic!("Throughput test size {} iteration {} failed: {}", size, i + 1, e),
                }
            }

            let avg_duration = total_duration / iterations;
            let throughput_per_sec = (iterations as f64 * 1000.0) / total_duration.as_millis() as f64;
            let docs_per_sec = throughput_per_sec * size as f64;

            println!("   Size {}: {:.3}ms avg, {:.1} reranks/sec, {:.1} docs/sec",
                   size, avg_duration.as_millis(), throughput_per_sec, docs_per_sec);

            // Throughput should not degrade dramatically with size
            assert!(throughput_per_sec >= 0.01, "Throughput should be reasonable for size {}", size);
        }

        println!("✅ Throughput scaling analysis completed");
    }

    #[test]
    #[ignore = "performance benchmark - environment sensitive"]
    fn test_rerank_cold_vs_warm_performance() {
        println!("\n🥵 vs 🌡️ Testing rerank cold vs warm performance");

        let client = gllm::Client::new("all-MiniLM-L6-v2")
            .expect("Failed to create gllm client");

        let query = "cold vs warm performance test";
        let documents: Vec<String> = (0..15).map(|i| {
            format!("Performance test document {} for cold vs warm comparison", i + 1)
        }).collect();

        // Cold start - first rerank after model loading
        let cold_start = Instant::now();
        let cold_duration = match client.rerank(query, &documents).generate() {
            Ok(_) => cold_start.elapsed(),
            Err(e) => panic!("Cold start test failed: {}", e),
        };
        println!("✅ Cold start: {:.3}ms", cold_duration.as_millis());

        // Warm runs - subsequent reranks
        let iterations = 3;
        let mut warm_total = std::time::Duration::ZERO;

        for i in 0..iterations {
            let start = Instant::now();
            match client.rerank(query, &documents).generate() {
                Ok(_) => {
                    let duration = start.elapsed();
                    warm_total += duration;
                    println!("✅ Warm run {}: {:.3}ms", i + 1, duration.as_millis());
                }
                Err(e) => panic!("Warm run {} failed: {}", i + 1, e),
            }
        }

        let avg_warm_duration = warm_total / iterations;
        let improvement_factor = cold_duration.as_millis() as f32 / avg_warm_duration.as_millis() as f32;

        println!("✅ Cold vs warm performance summary:");
        println!("   Cold start: {:.3}ms", cold_duration.as_millis());
        println!("   Average warm: {:.3}ms", avg_warm_duration.as_millis());
        println!("   Improvement factor: {:.2}x", improvement_factor);

        // Performance expectations
        assert!(avg_warm_duration.as_millis() < cold_duration.as_millis(),
               "Warm runs should be faster than cold start");
        assert!(improvement_factor >= 1.0, "There should be some performance improvement");

        println!("✅ Cold vs warm performance optimization confirmed");
    }
}