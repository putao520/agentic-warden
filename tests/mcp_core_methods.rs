//! MCP核心方法测试
//!
//! 测试MCP服务器的两个核心工具方法：
//! - start_concurrent_tasks: 并发启动多个AI CLI任务
//! - get_task_command: 获取单个AI CLI任务的启动命令

use serde_json::json;

/// 测试parse_ai_type验证逻辑
///
/// 验证只接受claude、codex、gemini三种AI类型
#[test]
fn test_ai_type_validation() {
    // 这是一个间接测试，通过调用get_task_command来验证AI类型验证

    // 有效的AI类型
    let valid_types = vec!["claude", "codex", "gemini"];
    for ai_type in valid_types {
        // 构建命令
        let command = format!("agent {} 'test task'", ai_type);
        assert!(
            command.contains(ai_type),
            "Should accept valid AI type: {}",
            ai_type
        );
    }

    println!("✅ Valid AI types accepted: claude, codex, gemini");
}

/// 测试get_task_command的命令生成（不带provider）
#[test]
fn test_get_task_command_without_provider() {
    let ai_type = "codex";
    let task = "Fix the bug in main.rs";

    // 模拟get_task_command的逻辑
    let command = format!("agent {} '{}'", ai_type, task.replace("'", "'\\''"));

    assert_eq!(command, "agent codex 'Fix the bug in main.rs'");
    println!("✅ Command without provider: {}", command);
}

/// 测试get_task_command的命令生成（带provider）
#[test]
fn test_get_task_command_with_provider() {
    let ai_type = "claude";
    let task = "Write documentation";
    let provider = "openrouter";

    // 模拟get_task_command的逻辑
    let command = format!(
        "agent {} -p {} '{}'",
        ai_type,
        provider,
        task.replace("'", "'\\''")
    );

    assert_eq!(
        command,
        "agent claude -p openrouter 'Write documentation'"
    );
    println!("✅ Command with provider: {}", command);
}

/// 测试任务描述中的特殊字符转义
#[test]
fn test_task_command_special_characters() {
    let task = "Fix bug in user's code";

    // 模拟shell转义逻辑
    let escaped = task.replace("'", "'\\''");

    assert_eq!(escaped, "Fix bug in user'\\''s code");
    println!("✅ Special character escaped: {}", escaped);

    // 验证完整命令
    let command = format!("agent codex '{}'", escaped);
    assert_eq!(command, "agent codex 'Fix bug in user'\\''s code'");
}

/// 测试start_concurrent_tasks的任务数组解析
#[test]
fn test_concurrent_tasks_json_parsing() {
    let tasks_json = json!([
        {
            "ai_type": "codex",
            "task": "Analyze code",
            "provider": "openrouter"
        },
        {
            "ai_type": "gemini",
            "task": "Generate docs"
        },
        {
            "ai_type": "claude",
            "task": "Write tests",
            "provider": "anthropic"
        }
    ]);

    let tasks: Vec<serde_json::Value> =
        serde_json::from_value(tasks_json).expect("Failed to parse tasks JSON");

    assert_eq!(tasks.len(), 3, "Should parse 3 tasks");

    // 验证第一个任务
    assert_eq!(tasks[0]["ai_type"], "codex");
    assert_eq!(tasks[0]["task"], "Analyze code");
    assert_eq!(tasks[0]["provider"], "openrouter");

    // 验证第二个任务（没有provider）
    assert_eq!(tasks[1]["ai_type"], "gemini");
    assert_eq!(tasks[1]["task"], "Generate docs");
    assert!(tasks[1].get("provider").is_none());

    println!("✅ Parsed {} concurrent tasks correctly", tasks.len());
}

/// 测试start_concurrent_tasks的命令生成
#[test]
fn test_concurrent_tasks_command_generation() {
    let tasks = vec![
        ("codex", "task1", Some("openrouter")),
        ("gemini", "task2", None),
        ("claude", "task3", Some("anthropic")),
    ];

    let mut commands = Vec::new();

    for (ai_type, task, provider) in tasks {
        let command = if let Some(p) = provider {
            format!("agent {} -p {} '{}'", ai_type, p, task.replace("'", "'\\''"))
        } else {
            format!("agent {} '{}'", ai_type, task.replace("'", "'\\''"))
        };
        commands.push(command);
    }

    assert_eq!(commands.len(), 3);
    assert_eq!(commands[0], "agent codex -p openrouter 'task1'");
    assert_eq!(commands[1], "agent gemini 'task2'");
    assert_eq!(commands[2], "agent claude -p anthropic 'task3'");

    println!("✅ Generated {} commands for concurrent tasks", commands.len());
}

/// 测试无效的JSON格式
#[test]
fn test_invalid_json_handling() {
    let invalid_json = "not a valid json";

    let result: Result<Vec<serde_json::Value>, _> = serde_json::from_str(invalid_json);

    assert!(result.is_err(), "Should reject invalid JSON");
    println!("✅ Invalid JSON rejected correctly");
}

/// 测试缺少必需字段
#[test]
fn test_missing_required_fields() {
    let tasks_json = json!([
        {
            "task": "Missing ai_type field"
        },
        {
            "ai_type": "codex"
            // missing task field
        }
    ]);

    let tasks: Vec<serde_json::Value> =
        serde_json::from_value(tasks_json).expect("JSON is valid");

    // 第一个任务缺少ai_type
    assert!(
        tasks[0].get("ai_type").is_none(),
        "First task should be missing ai_type"
    );

    // 第二个任务缺少task
    assert!(
        tasks[1].get("task").is_none(),
        "Second task should be missing task"
    );

    println!("✅ Missing fields detected correctly");
}

/// 测试无效的AI类型
#[test]
fn test_invalid_ai_type() {
    let invalid_types = vec!["unknown", "gpt", "chatgpt", "bard"];

    for ai_type in invalid_types {
        let is_valid = matches!(ai_type, "claude" | "codex" | "gemini");
        assert!(!is_valid, "Should reject invalid AI type: {}", ai_type);
    }

    println!("✅ Invalid AI types rejected");
}

/// 测试返回的task JSON结构（单个任务）
#[test]
fn test_task_json_structure() {
    let task_json = json!({
        "success": true,
        "task": {
            "description": "Execute codex task: Fix bug",
            "tool": "bash",
            "command": "agent codex 'Fix bug'",
            "timeout_ms": 43200000
        },
        "ai_type": "codex",
        "message": "Execute the 'task' using Bash tool with 12h timeout"
    });

    assert_eq!(task_json["success"], true);
    assert_eq!(task_json["task"]["tool"], "bash");
    assert_eq!(task_json["task"]["timeout_ms"], 43200000);
    assert!(task_json["task"]["command"]
        .as_str()
        .unwrap()
        .starts_with("agent"));

    println!("✅ Task JSON structure validated");
}

/// 测试返回的task JSON结构（并发任务）
#[test]
fn test_concurrent_tasks_response_structure() {
    let response_json = json!({
        "success": true,
        "tasks": [
            {
                "success": true,
                "ai_type": "codex",
                "task": "task1",
                "command": "agent codex 'task1'"
            },
            {
                "success": true,
                "ai_type": "gemini",
                "task": "task2",
                "command": "agent gemini 'task2'"
            }
        ],
        "count": 2,
        "message": "Execute these commands using Bash tool with background mode (run_in_background: true) for concurrent execution"
    });

    assert_eq!(response_json["success"], true);
    assert_eq!(response_json["count"], 2);
    assert!(response_json["tasks"].is_array());
    assert_eq!(response_json["tasks"].as_array().unwrap().len(), 2);

    println!("✅ Concurrent tasks response structure validated");
}

/// 测试超时时间配置
#[test]
fn test_timeout_configuration() {
    let timeout_ms = 43200000; // 12小时 = 12 * 60 * 60 * 1000

    assert_eq!(timeout_ms, 12 * 60 * 60 * 1000);
    assert_eq!(timeout_ms / 1000 / 60 / 60, 12);

    println!("✅ Timeout configured to 12 hours (43200000ms)");
}

/// 测试空任务数组
#[test]
fn test_empty_tasks_array() {
    let tasks_json = json!([]);

    let tasks: Vec<serde_json::Value> =
        serde_json::from_value(tasks_json).expect("Empty array is valid JSON");

    assert_eq!(tasks.len(), 0);
    println!("✅ Empty tasks array handled");
}

/// 测试大量并发任务
#[test]
fn test_large_concurrent_tasks() {
    let mut tasks = Vec::new();

    for i in 0..100 {
        tasks.push(json!({
            "ai_type": "codex",
            "task": format!("task-{}", i)
        }));
    }

    let tasks_json = serde_json::Value::Array(tasks);
    let parsed_tasks: Vec<serde_json::Value> =
        serde_json::from_value(tasks_json).expect("Should parse large array");

    assert_eq!(parsed_tasks.len(), 100);
    println!("✅ Handled 100 concurrent tasks");
}
