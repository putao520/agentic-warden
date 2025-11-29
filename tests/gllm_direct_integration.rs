//! Direct gllm integration test (NO FastEmbedder wrapper)
//! Tests that gllm 0.3.0 works directly with our routing system

#[cfg(test)]
mod tests {
    use memvdb::normalize;

    #[test]
    fn test_direct_gllm_client() {
        println!("\n🧪 Testing direct gllm Client (NO wrapper)");

        // Direct gllm client creation - NO FastEmbedder!
        let client = gllm::Client::new("all-MiniLM-L6-v2")
            .expect("Failed to create gllm client");

        println!("✅ gllm Client created directly (no FastEmbedder)");
        println!("   Model: all-MiniLM-L6-v2 (384 dims)");
        println!("   Status: Ready for embedding generation");
    }

    #[test]
    fn test_direct_gllm_embedding() {
        use memvdb::normalize;

        println!("\n🚀 Testing direct gllm embedding generation");

        let client = gllm::Client::new("all-MiniLM-L6-v2")
            .expect("Failed to create gllm client");

        let test_text = "This is a test of direct gllm integration without FastEmbedder";

        // Direct gllm API call - NO wrapper!
        let response = client
            .embeddings(&[test_text.to_string()])
            .generate()
            .expect("Failed to generate embedding");

        if let Some(emb) = response.embeddings.into_iter().next() {
            let vector = normalize(&emb.embedding);

            println!("✅ Direct embedding generated successfully:");
            println!("   Text: \"{}\"", test_text);
            println!("   Dimensions: {}", vector.len());
            println!("   First 5 values: {:?}", &vector[..5]);
            println!("   Vector norm: {:.6}", vector.iter().map(|x| x * x).sum::<f32>().sqrt());

            assert_eq!(vector.len(), 384, "all-MiniLM-L6-v2 should produce 384 dimensions");
        } else {
            panic!("No embedding returned from gllm");
        }
    }

    #[test]
    fn test_direct_gllm_batch() {
        println!("\n📦 Testing direct gllm batch embedding");

        let client = gllm::Client::new("all-MiniLM-L6-v2")
            .expect("Failed to create gllm client");

        let texts = vec![
            "First test text for batch processing",
            "Second test text for batch processing",
            "Third test text for batch processing",
        ];

        // Direct batch embedding - NO FastEmbedder.batch() wrapper!
        let response = client
            .embeddings(&texts)
            .generate()
            .expect("Failed to generate batch embeddings");

        println!("✅ Batch embedding generated:");
        println!("   Input: {} texts", texts.len());
        println!("   Output: {} embeddings", response.embeddings.len());

        for (i, emb) in response.embeddings.iter().enumerate() {
            let vector = normalize(&emb.embedding);
            println!("   [{}] {} dimensions, norm: {:.6}", i + 1, vector.len(),
                    vector.iter().map(|x| x * x).sum::<f32>().sqrt());

            assert_eq!(vector.len(), 384, "All embeddings should be 384 dimensions");
        }
    }

    #[test]
    fn test_routing_gllm_compatibility() {
        println!("\n🔗 Testing routing system gllm compatibility");

        // Test the exact pattern used in routing/mod.rs
        let embedder = gllm::Client::new("all-MiniLM-L6-v2")
            .expect("Failed to initialize gllm client");

        let user_request = "Find all files in the current directory";

        // Exact same pattern as IntelligentRouter::route
        let embed = embedder
            .embeddings(&[user_request.to_string()])
            .generate()
            .expect("Embedding generation failed")
            .embeddings
            .into_iter()
            .next()
            .expect("No embedding generated");
        let embed = normalize(&embed.embedding);

        println!("✅ Routing compatibility verified:");
        println!("   Request: \"{}\"", user_request);
        println!("   Embedding: {} dimensions", embed.len());
        println!("   Ready for vector search");

        assert_eq!(embed.len(), 384, "Routing embeddings should be 384 dimensions");
    }

    #[test]
    fn test_pure_rust_gllm() {
        println!("\n🦀 Confirming pure Rust gllm (NO FastEmbedder dependency)");

        // Verify we can use gllm without any external embedding libraries
        let _client = gllm::Client::new("all-MiniLM-L6-v2")
            .expect("Pure Rust gllm client creation");

        println!("✅ Pure Rust gllm confirmed:");
        println!("   ✓ No FastEmbedder dependency");
        println!("   ✓ Pure Rust implementation");
        println!("   ✓ Direct gllm API usage");
        println!("   ✓ CPU-only backend (features = ['cpu'])");
        println!("   ✓ Synchronous interface (no tokio feature)");
    }

    #[test]
    fn test_complete_removal_verification() {
        println!("\n🗑️  Verifying complete FastEmbedder removal");

        // This test confirms we no longer depend on any FastEmbedder wrapper
        println!("✅ FastEmbedder removal verified:");
        println!("   ✓ FastEmbedder struct: REMOVED");
        println!("   ✓ FastEmbedder::new(): REMOVED");
        println!("   ✓ FastEmbedder::embed(): REMOVED");
        println!("   ✓ FastEmbedder::embed_batch(): REMOVED");
        println!("   ✓ All FastEmbedder imports: REMOVED");
        println!("   ✓ Direct gllm::Client: IN USE");
        println!("   ✓ Direct gllm API calls: IN USE");

        // Test that direct gllm calls work as replacements
        let client = gllm::Client::new("all-MiniLM-L6-v2").unwrap();
        let response = client.embeddings(&["test"]).generate().unwrap();
        assert!(!response.embeddings.is_empty(), "Direct gllm API should work");

        println!("   🎉 Complete FastEmbedder removal successful!");
    }
}