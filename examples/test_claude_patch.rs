//! 测试 Claude CLI 内存补丁功能

use aiw::patcher::{PatchError, PatchResult, RuntimePatcher};

fn main() -> Result<(), PatchError> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // 查找 Claude 进程
    let pid = find_claude_pid()?;
    println!("Found Claude process with PID: {}", pid);

    // 创建补丁器
    let patcher = RuntimePatcher::new(pid)?;
    println!("Created patcher for PID {}", pid);

    // 测试搜索模式
    test_search_pattern(&patcher)?;

    // 应用补丁
    match patcher.apply_claude_toolsearch_patch() {
        Ok(addr) => {
            println!("✅ Patch successfully applied at address: 0x{:x}", addr);
        }
        Err(e) => {
            println!("❌ Patch failed: {}", e);
        }
    }

    Ok(())
}

fn find_claude_pid() -> Result<u32, PatchError> {
    use std::process::Command;
    
    let output = Command::new("pgrep")
        .arg("-n")
        .arg("claude")
        .output()
        .map_err(|e| PatchError::ReadFailed {
            reason: format!("Failed to run pgrep: {}", e),
        })?;

    if output.status.success() {
        let pid_str = String::from_utf8_lossy(&output.stdout);
        let pid = pid_str.trim().parse::<u32>().map_err(|_| PatchError::ReadFailed {
            reason: "Failed to parse PID".to_string(),
        })?;
        Ok(pid)
    } else {
        Err(PatchError::ProcessNotFound { pid: 0 })
    }
}

fn test_search_pattern(patcher: &RuntimePatcher) -> PatchResult<()> {
    // 测试搜索完整模式
    let pattern = b"O8()===\"firstParty\"&&!JB()";
    println!("\nSearching for pattern: {:?}", std::str::from_utf8(pattern).unwrap());
    
    match patcher.search_pattern(pattern) {
        Ok(Some(addr)) => {
            println!("✅ Found pattern at address: 0x{:x}", addr);
            
            // 读取并显示上下文
            let mut context = vec![0u8; 60];
            if patcher.read_memory(addr.saturating_sub(10), &mut context).is_ok() {
                println!("Context: {}", String::from_utf8_lossy(&context));
            }
        }
        Ok(None) => {
            println!("❌ Pattern not found in memory");
        }
        Err(e) => {
            println!("❌ Error searching pattern: {}", e);
        }
    }
    
    Ok(())
}
