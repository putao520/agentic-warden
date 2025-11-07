use agentic_warden::TaskRegistry;

fn main() {
    println!("开始连接 TaskRegistry...");
    let start = std::time::Instant::now();

    match TaskRegistry::connect() {
        Ok(registry) => {
            let duration = start.elapsed();
            println!("TaskRegistry 连接成功，耗时: {:?}", duration);
        }
        Err(e) => {
            let duration = start.elapsed();
            println!("TaskRegistry 连接失败，耗时: {:?}, 错误: {}", duration, e);
        }
    }
}