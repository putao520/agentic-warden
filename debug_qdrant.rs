use qdrant_client::Qdrant;
use std::time::Duration;

#[tokio::main]
async fn main() {
    println!("🔍 调试 Qdrant 连接问题...");

    let urls = vec![
        "http://localhost:6333",
        "http://localhost:26333"
    ];

    for url in urls {
        println!("\n测试 URL: {}", url);

        // 测试基本 HTTP 连接
        match reqwest::get(format!("{}/collections", url)).await {
            Ok(response) => {
                println!("  ✅ HTTP 连接正常: {}", response.status());
            }
            Err(e) => {
                println!("  ❌ HTTP 连接失败: {}", e);
                continue;
            }
        }

        // 测试 qdrant-client 连接
        println!("  测试 qdrant-client 连接...");
        match test_qdrant_client(url).await {
            Ok(_) => println!("  ✅ qdrant-client 连接成功"),
            Err(e) => println!("  ❌ qdrant-client 连接失败: {}", e),
        }
    }
}

async fn test_qdrant_client(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("    尝试创建客户端...");
    let client = Qdrant::from_url(url)
        .skip_compatibility_check()
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(5))
        .build()?;

    println!("    尝试列出集合...");
    let collections = client.list_collections().await?;

    println!("    成功！发现 {} 个集合", collections.collections.len());

    for collection in &collections.collections {
        println!("      - {}", collection.name);
    }

    Ok(())
}