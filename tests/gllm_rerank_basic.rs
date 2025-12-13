//! gllm rerank basic functionality tests
//! Tests core reranking capabilities with real gllm models

#[cfg(test)]
mod tests {
    #[test]
    fn test_rerank_client_creation() {
        println!("\n🧪 Testing gllm rerank client creation");

        let client = gllm::Client::new("all-MiniLM-L6-v2")
            .expect("Failed to create gllm client");

        println!("✅ gllm Client created successfully for reranking");
        println!("   Model: all-MiniLM-L6-v2");
        println!("   Reranking capability: Available");
    }

    #[test]
    fn test_basic_reranking() {
        println!("\n🎯 Testing basic reranking functionality");

        let client = gllm::Client::new("all-MiniLM-L6-v2")
            .expect("Failed to create gllm client");

        let query = "machine learning";
        let documents = vec![
            "Artificial intelligence and machine learning algorithms".to_string(),
            "Cooking recipes and food preparation techniques".to_string(),
            "Deep learning neural networks and AI models".to_string(),
            "Sports news and game highlights".to_string(),
            "Computer vision and image recognition".to_string(),
        ];

        // Basic reranking without additional parameters
        match client.rerank(query, &documents).generate() {
            Ok(response) => {
                println!("✅ Basic reranking successful:");
                println!("   Query: '{}'", query);
                println!("   Input documents: {}", documents.len());
                println!("   Results returned: {}", response.results.len());

                // Verify results
                assert_eq!(response.results.len(), documents.len(), "Should return results for all documents");

                let first_result = &response.results[0];
                println!("   Top result: Document #{} with score {:.6}",
                       first_result.index, first_result.score);

                // Verify result properties
                for (i, result) in response.results.iter().enumerate() {
                    assert!(result.score > 0.0, "Rerank score should be positive");

                    if let Some(doc) = &result.document {
                        assert_eq!(doc, &documents[result.index], "Document should match original");
                    }
                }

                // Note: gllm returns documents sorted by score, not original index order

                // Verify scores are sorted in descending order
                for i in 1..response.results.len() {
                    assert!(response.results[i-1].score >= response.results[i].score,
                           "Scores should be sorted in descending order");
                }

                println!("✅ All basic reranking assertions passed");
            }
            Err(e) => panic!("Basic reranking failed: {}", e),
        }
    }

    #[test]
    fn test_rerank_with_top_n() {
        println!("\n🎯 Testing reranking with top_n parameter");

        let client = gllm::Client::new("all-MiniLM-L6-v2")
            .expect("Failed to create gllm client");

        let query = "climate change";
        let documents = vec![
            "Global warming and environmental impact studies".to_string(),
            "Weather forecasting and meteorology".to_string(),
            "Renewable energy and solar power".to_string(),
            "Space exploration and astronomy".to_string(),
            "Marine biology and ocean conservation".to_string(),
            "Carbon footprint reduction strategies".to_string(),
        ];

        // Test with top_n = 3
        match client.rerank(query, &documents).top_n(3).generate() {
            Ok(response) => {
                println!("✅ Reranking with top_n successful:");
                println!("   Query: '{}'", query);
                println!("   Total documents: {}", documents.len());
                println!("   Requested top_n: 3");
                println!("   Results returned: {}", response.results.len());

                // Verify only top 3 results are returned
                assert_eq!(response.results.len(), 3, "Should return exactly top 3 results");

                // Verify the remaining results are not from the original set
                for result in &response.results {
                    assert!(result.index < documents.len(), "Result index should be valid");
                    if let Some(doc) = &result.document {
                        assert_eq!(doc, &documents[result.index], "Document should match original");
                    }
                }

                // Verify scores are still sorted
                for i in 1..response.results.len() {
                    assert!(response.results[i-1].score >= response.results[i].score,
                           "Scores should be sorted in descending order");
                }

                println!("✅ top_n filtering works correctly");
            }
            Err(e) => panic!("Reranking with top_n failed: {}", e),
        }
    }

    #[test]
    fn test_rerank_with_return_documents() {
        println!("\n📄 Testing reranking with return_documents parameter");

        let client = gllm::Client::new("all-MiniLM-L6-v2")
            .expect("Failed to create gllm client");

        let query = "artificial intelligence";
        let documents = vec![
            "Machine learning and AI algorithms".to_string(),
            "Data analysis and statistical methods".to_string(),
        ];

        // Test with return_documents = true
        match client.rerank(query, &documents).return_documents(true).generate() {
            Ok(response) => {
                println!("✅ Reranking with return_documents true:");
                println!("   Results with documents: {}", response.results.len());

                // Verify all results contain documents
                for result in &response.results {
                    assert!(result.document.is_some(), "All results should contain documents when return_documents=true");
                    if let Some(doc) = &result.document {
                        assert_eq!(doc, &documents[result.index], "Document should match original");
                    }
                }

                println!("✅ return_documents=true works correctly");
            }
            Err(e) => panic!("Reranking with return_documents true failed: {}", e),
        }

        // Test with return_documents = false
        match client.rerank(query, &documents).return_documents(false).generate() {
            Ok(response) => {
                println!("✅ Reranking with return_documents false:");
                println!("   Results without documents: {}", response.results.len());

                // Verify all results don't contain documents
                for result in &response.results {
                    assert!(result.document.is_none(), "No results should contain documents when return_documents=false");
                }

                println!("✅ return_documents=false works correctly");
            }
            Err(e) => panic!("Reranking with return_documents false failed: {}", e),
        }
    }

    #[test]
    fn test_rerank_empty_documents() {
        println!("\n❌ Testing reranking with empty documents (should error)");

        let client = gllm::Client::new("all-MiniLM-L6-v2")
            .expect("Failed to create gllm client");

        let query = "test query";
        let empty_documents: Vec<String> = vec![];

        // This should fail because no documents are provided
        match client.rerank(query, &empty_documents).generate() {
            Ok(_) => {
                println!("❌ Expected reranking to fail with empty documents, but it succeeded");
                panic!("Reranking should fail with empty documents");
            }
            Err(e) => {
                println!("✅ Reranking correctly failed with empty documents:");
                println!("   Error: {}", e);

                // Verify it's the expected error type
                let error_message = e.to_string();
                assert!(error_message.contains("At least one document is required"),
                       "Error should mention document requirement");
            }
        }
    }

    #[test]
    fn test_rerank_single_document() {
        println!("\n📝 Testing reranking with single document");

        let client = gllm::Client::new("all-MiniLM-L6-v2")
            .expect("Failed to create gllm client");

        let query = "programming language";
        let documents = vec!["Rust systems programming language".to_string()];

        match client.rerank(query, &documents).generate() {
            Ok(response) => {
                println!("✅ Single document reranking successful:");
                println!("   Documents: {}", documents.len());
                println!("   Results: {}", response.results.len());

                assert_eq!(response.results.len(), 1, "Should return exactly one result");

                let result = &response.results[0];
                assert_eq!(result.index, 0, "Single result should have index 0");
                assert!(result.score > 0.0, "Single result should have positive score");

                if let Some(doc) = &result.document {
                    assert_eq!(doc, &documents[0], "Document should match original");
                }

                println!("✅ Single document reranking works correctly");
            }
            Err(e) => panic!("Single document reranking failed: {}", e),
        }
    }

    #[test]
    fn test_rerank_identical_documents() {
        println!("\n🔄 Testing reranking with identical documents");

        let client = gllm::Client::new("all-MiniLM-L6-v2")
            .expect("Failed to create gllm client");

        let query = "machine learning";
        let documents = vec![
            "Machine learning and AI".to_string(),
            "Machine learning and AI".to_string(),
            "Machine learning and AI".to_string(),
        ];

        match client.rerank(query, &documents).generate() {
            Ok(response) => {
                println!("✅ Identical documents reranking successful:");
                println!("   Documents: {}", documents.len());
                println!("   Results: {}", response.results.len());

                assert_eq!(response.results.len(), documents.len(), "Should return results for all documents");

                // All scores should be similar (documents are identical)
                let scores: Vec<f32> = response.results.iter().map(|r| r.score).collect();
                let score_variance = scores.iter()
                    .map(|score| (*score - scores[0]).powi(2))
                    .sum::<f32>() / scores.len() as f32;

                println!("✅ Scores variance: {:.8}", score_variance);

                // Identical documents should have very similar scores
                assert!(score_variance < 0.001, "Identical documents should have very similar scores");

                // Results should still be sorted
                for i in 1..response.results.len() {
                    assert!(response.results[i-1].score >= response.results[i].score,
                           "Scores should be sorted in descending order");
                }

                println!("✅ Identical documents reranking works correctly");
            }
            Err(e) => panic!("Identical documents reranking failed: {}", e),
        }
    }

    #[test]
    fn test_rerank_long_query() {
        println!("\n📝 Testing reranking with long query");

        let client = gllm::Client::new("all-MiniLM-L6-v2")
            .expect("Failed to create gllm client");

        let long_query = "The impact of artificial intelligence and machine learning on modern software development practices and methodologies in large scale enterprise applications".to_string();
        let documents = vec![
            "AI in software development".to_string(),
            "Traditional programming methodologies".to_string(),
            "Machine learning algorithms".to_string(),
            "Enterprise application architecture".to_string(),
        ];

        match client.rerank(&long_query, &documents).generate() {
            Ok(response) => {
                println!("✅ Long query reranking successful:");
                println!("   Query length: {} chars", long_query.len());
                println!("   Documents: {}", documents.len());
                println!("   Results: {}", response.results.len());

                assert_eq!(response.results.len(), documents.len(), "Should return results for all documents");

                // Verify results are sorted
                for i in 1..response.results.len() {
                    assert!(response.results[i-1].score >= response.results[i].score,
                           "Scores should be sorted in descending order");
                }

                println!("✅ Long query reranking works correctly");
            }
            Err(e) => panic!("Long query reranking failed: {}", e),
        }
    }

    #[test]
    fn test_rerank_unicode_content() {
        println!("\n🌍 Testing reranking with unicode content");

        let client = gllm::Client::new("all-MiniLM-L6-v2")
            .expect("Failed to create gllm client");

        let query = "programming language";
        let documents = vec![
            "Python 🐍 programming language".to_string(),
            "Rust 🦀 systems programming".to_string(),
            "JavaScript 💻 web development".to_string(),
            "C++ 📚 high performance computing".to_string(),
        ];

        match client.rerank(query, &documents).generate() {
            Ok(response) => {
                println!("✅ Unicode reranking successful:");
                println!("   Documents: {}", documents.len());
                println!("   Results: {}", response.results.len());

                assert_eq!(response.results.len(), documents.len(), "Should return results for all documents");

                // Verify Unicode content is preserved
                for result in &response.results {
                    if let Some(doc) = &result.document {
                        assert_eq!(doc, &documents[result.index], "Unicode document should match original");
                    }
                }

                println!("✅ Unicode reranking works correctly");
            }
            Err(e) => panic!("Unicode reranking failed: {}", e),
        }
    }
}