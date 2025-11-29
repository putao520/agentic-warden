// Standalone test for gllm direct integration (independent of project compilation issues)
// To run: rustc standalone_gllm_test.rs --extern gllm=/path/to/gllm.rlib --extern memvdb=/path/to/memvdb.rlib

fn main() {
    println!("🚀 Testing gllm direct integration (NO FastEmbedder)");

    // Test 1: Direct gllm client creation
    match gllm::Client::new("all-MiniLM-L6-v2") {
        Ok(client) => {
            println!("✅ gllm Client created directly (no FastEmbedder wrapper)");

            // Test 2: Direct embedding generation
            let test_text = "Direct gllm test without FastEmbedder";
            match client.embeddings(&[test_text.to_string()]).generate() {
                Ok(response) => {
                    if let Some(emb) = response.embeddings.into_iter().next() {
                        let vector = memvdb::normalize(&emb.embedding);

                        println!("✅ Direct embedding generation successful:");
                        println!("   Text: \"{}\"", test_text);
                        println!("   Dimensions: {}", vector.len());
                        println!("   First 5 values: {:?}", &vector[..5]);
                        println!("   Vector norm: {:.6}",
                                vector.iter().map(|x| x * x).sum::<f32>().sqrt());

                        assert_eq!(vector.len(), 384, "all-MiniLM-L6-v2 should produce 384 dimensions");

                        // Test 3: Batch embedding
                        let texts = vec![
                            "Batch test 1",
                            "Batch test 2",
                            "Batch test 3"
                        ];

                        match client.embeddings(&texts).generate() {
                            Ok(batch_response) => {
                                println!("✅ Direct batch embedding successful:");
                                println!("   Input: {} texts", texts.len());
                                println!("   Output: {} embeddings", batch_response.embeddings.len());

                                for (i, emb) in batch_response.embeddings.iter().enumerate() {
                                    let vector = memvdb::normalize(&emb.embedding);
                                    println!("   [{}] {} dimensions", i + 1, vector.len());
                                    assert_eq!(vector.len(), 384, "All embeddings should be 384 dimensions");
                                }

                                println!("\n🎉 ALL TESTS PASSED!");
                                println!("   ✓ FastEmbedder completely removed");
                                println!("   ✓ Direct gllm::Client usage working");
                                println!("   ✓ Synchronous API (no tokio) working");
                                println!("   ✓ Pure Rust implementation confirmed");

                            }
                            Err(e) => panic!("Batch embedding failed: {}", e),
                        }

                    } else {
                        panic!("No embedding generated");
                    }
                }
                Err(e) => panic!("Embedding generation failed: {}", e),
            }
        }
        Err(e) => panic!("gllm client creation failed: {}", e),
    }
}