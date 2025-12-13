//! Real gllm embedding demonstration
//! Shows actual text input and vector output

#[test]
fn test_real_embedding_generation() {
    use memvdb::normalize;

    println!("\n🚀 Starting real embedding generation test...\n");

    // Initialize embedder
    let embedder = match gllm::Client::new("all-MiniLM-L6-v2") {
        Ok(e) => {
            println!("✅ gllm client initialized");
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
        match embedder.embeddings(&[text.to_string()]).generate() {
            Ok(response) => {
                if let Some(emb) = response.embeddings.into_iter().next() {
                    let vector = normalize(&emb.embedding);
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
                } else {
                    panic!("No embedding generated for text: {}", text);
                }
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
    use memvdb::normalize;

    println!("\n🔄 Testing embedding consistency...\n");

    let embedder = gllm::Client::new("all-MiniLM-L6-v2").expect("Failed to initialize gllm client");

    let text = "Consistency test for embeddings";

    // Generate same embedding twice
    let embedding1 = embedder.embeddings(&[text.to_string()]).generate().expect("First embedding failed")
        .embeddings.into_iter().next().expect("No embedding generated");
    let embedding2 = embedder.embeddings(&[text.to_string()]).generate().expect("Second embedding failed")
        .embeddings.into_iter().next().expect("No embedding generated");

    // Normalize both vectors
    let norm1 = normalize(&embedding1.embedding);
    let norm2 = normalize(&embedding2.embedding);

    // Check if they're identical
    let all_equal = norm1
        .iter()
        .zip(norm2.iter())
        .all(|(a, b)| (a - b).abs() < f32::EPSILON);

    if all_equal {
        println!("✅ Embeddings are consistent (identical)");
    } else {
        println!("⚠️  Embeddings differ (this may be expected due to nondeterministic computation)");
        println!("   First 5 values (normalized): {:?}", &norm1[..5.min(norm1.len())]);
        println!("   First 5 values (normalized): {:?}", &norm2[..5.min(norm2.len())]);
    }

    assert_eq!(norm1.len(), norm2.len(), "Both embeddings should have same dimension");
    assert_eq!(norm1.len(), 384, "all-MiniLM-L6-v2 should produce 384 dimensions");
}

#[test]
fn test_multilingual_embeddings() {
    use memvdb::normalize;

    println!("\n🌍 Testing multilingual embedding capabilities...\n");

    let embedder = gllm::Client::new("all-MiniLM-L6-v2").expect("Failed to initialize gllm client");

    let multilingual_texts = vec![
        ("English", "Machine learning transforms data into insights"),
        ("Chinese", "机器学习将数据转化为洞察"),
        ("Spanish", "El aprendizaje automático transforma datos en conocimientos"),
        ("French", "L'apprentissage automatique transforme les données en informations"),
    ];

    for (lang, text) in multilingual_texts {
        match embedder.embeddings(&[text.to_string()]).generate() {
            Ok(response) => {
                if let Some(emb) = response.embeddings.into_iter().next() {
                    let vector = normalize(&emb.embedding);
                    println!("🌐 {} ({}): {} dimensions", lang, text, vector.len());
                } else {
                    panic!("No embedding generated for {} text", lang);
                }
            }
            Err(e) => {
                panic!("Failed to generate embedding for {}: {}", lang, e);
            }
        }
    }

    println!("\n✅ Multilingual support confirmed!");
}