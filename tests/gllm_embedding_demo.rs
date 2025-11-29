//! Real gllm embedding demonstration
//! Shows actual text input and vector output

#[test]
fn test_real_embedding_generation() {
    use agentic_warden::mcp_routing::FastEmbedder;

    println!("\n🚀 Starting real embedding generation test...\n");

    // Initialize embedder
    let embedder = match FastEmbedder::new("all-MiniLM-L6-v2") {
        Ok(e) => {
            println!("✅ Embedder initialized");
            e
        }
        Err(e) => panic!("Failed to initialize: {}", e),
    };

    // Test texts
    let test_cases = vec![
        "Machine learning is a subset of artificial intelligence",
        "The quick brown fox jumps over the lazy dog",
        "Hello world",
    ];

    println!("📝 Generating embeddings for {} texts:\n", test_cases.len());

    for (idx, text) in test_cases.iter().enumerate() {
        match embedder.embed(text) {
            Ok(vector) => {
                println!("Test #{}", idx + 1);
                println!("  📄 Input text: \"{}\"", text);
                println!("  📊 Output vector: {} dimensions", vector.len());
                println!(
                    "  🔢 First 10 values: {:?}",
                    &vector[..10.min(vector.len())]
                );
                println!(
                    "  📈 Vector magnitude: {:.4}",
                    vector.iter().map(|x| x * x).sum::<f32>().sqrt()
                );
                println!("  ✅ Generated successfully\n");
            }
            Err(e) => {
                eprintln!("❌ Failed to generate embedding: {}", e);
                panic!("Embedding failed for: {}", text);
            }
        }
    }

    println!("\n✅ All embeddings generated successfully!");
    println!("   Model downloaded to: ~/.gllm/models/sentence-transformers--all-MiniLM-L6-v2/");
}

#[test]
fn test_embedding_consistency() {
    use agentic_warden::mcp_routing::FastEmbedder;

    println!("\n🔄 Testing embedding consistency...\n");

    let embedder = FastEmbedder::new("all-MiniLM-L6-v2").expect("Failed to initialize embedder");

    let text = "Consistency test for embeddings";

    // Generate same embedding twice
    let embedding1 = embedder.embed(text).expect("First embedding failed");
    let embedding2 = embedder.embed(text).expect("Second embedding failed");

    // Check if they're identical
    let all_equal = embedding1
        .iter()
        .zip(embedding2.iter())
        .all(|(a, b)| (a - b).abs() < 1e-6);

    println!("  📄 Text: \"{}\"", text);
    println!("  🔄 Generated embedding twice");
    println!(
        "  ✅ Embeddings are {}",
        if all_equal { "identical" } else { "different" }
    );

    assert!(all_equal, "Embeddings should be deterministic");
    println!("\n✅ Consistency test passed!");
}

#[test]
fn test_batch_embedding() {
    use agentic_warden::mcp_routing::FastEmbedder;

    println!("\n📦 Testing batch embedding...\n");

    let embedder = FastEmbedder::new("all-MiniLM-L6-v2").expect("Failed to initialize embedder");

    let texts = vec![
        "Batch processing is efficient".to_string(),
        "Multiple texts at once".to_string(),
        "Better performance".to_string(),
    ];

    match embedder.embed_batch(&texts) {
        Ok(embeddings) => {
            println!("  📄 Input batch: {} texts", texts.len());
            for (idx, text) in texts.iter().enumerate() {
                println!("    [{}] \"{}\"", idx + 1, text);
            }
            println!("\n  📊 Output: {} embeddings", embeddings.len());
            for (idx, emb) in embeddings.iter().enumerate() {
                println!(
                    "    [{}] {} dimensions, magnitude: {:.4}",
                    idx + 1,
                    emb.len(),
                    emb.iter().map(|x| x * x).sum::<f32>().sqrt()
                );
            }
            println!("\n✅ Batch embedding successful!");
        }
        Err(e) => panic!("Batch embedding failed: {}", e),
    }
}
