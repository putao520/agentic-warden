use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

fn main() {
    let model = TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::AllMiniLML6V2)
            .with_show_download_progress(true),
    )
    .unwrap();

    let texts = vec![
        "take a screenshot of the current page".to_string(),
        "I love eating pizza for dinner".to_string(),
        "quantum physics explains particle behavior".to_string(),
        "the cat sat on the mat".to_string(),
        "browser_take_screenshot: capture page image".to_string(),
    ];

    let embeddings = model.embed(texts.clone(), None).unwrap();

    let query = &embeddings[0];
    let norm_q: f32 = query.iter().map(|x| x * x).sum::<f32>().sqrt();

    for (i, (text, emb)) in texts.iter().zip(embeddings.iter()).enumerate() {
        let norm: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        eprintln!(
            "Text {}: dim={}, norm={:.4}, first5={:.4?}",
            i,
            emb.len(),
            norm,
            &emb[..5]
        );

        if i > 0 {
            let dot: f32 = query.iter().zip(emb).map(|(a, b)| a * b).sum();
            let cosine = dot / (norm_q * norm);
            eprintln!("  Cosine(query, text{}): {:.4}", i, cosine);
        }
    }
}
