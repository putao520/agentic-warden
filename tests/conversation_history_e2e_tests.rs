//! Conversation History E2E Tests
//! Tests REQ-010: Claude Code会话历史集成

mod common;

use agentic_warden::memory::ConversationHistoryStore;
use anyhow::Result;
use common::{
    conversation_store, create_test_transcript, process_hook_stdin, reset_conversation_store,
    setup_test_conversations, simple_embed,
};
use serial_test::serial;
use serde_json::json;

#[tokio::test]
#[serial]
async fn test_claude_code_hook_integration() -> Result<()> {
    reset_conversation_store()?;
    let session_id = "test-session-123";
    let transcript_path = create_test_transcript(
        session_id,
        vec![
            ("user", "Help me implement auth"),
            (
                "assistant",
                "I'll help you...\n- [ ] Create login endpoint\n- [ ] Add JWT validation",
            ),
        ],
    )
    .await?;

    process_hook_stdin(json!({
        "session_id": session_id,
        "transcript_path": transcript_path.to_string_lossy(),
        "hook_event_name": "SessionEnd",
        "cwd": "/tmp",
        "permission_mode": "normal"
    }))
    .await?;

    let store = conversation_store()?;
    let results = store.search_with_scores(simple_embed("implement auth"), 10)?;
    assert!(!results.is_empty(), "should index conversation history");

    let has_login_todo = results.iter().any(|res| {
        res.record
            .todo_items
            .iter()
            .any(|t| t.description.to_lowercase().contains("login endpoint"))
    });
    assert!(has_login_todo, "todos should include login endpoint");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_todo_extraction_from_transcript() -> Result<()> {
    reset_conversation_store()?;
    let transcript = create_test_transcript(
        "session-456",
        vec![
            ("user", "Create API"),
            (
                "assistant",
                "TODO: Design schema\nAction Items:\n- Implement endpoints\n- Write tests",
            ),
        ],
    )
    .await?;

    process_hook_stdin(json!({
        "session_id": "session-456",
        "transcript_path": transcript.to_string_lossy(),
        "hook_event_name": "SessionEnd",
    }))
    .await?;

    let store: ConversationHistoryStore = conversation_store()?;
    let results = store.search_with_scores(simple_embed("API"), 5)?;
    assert!(!results.is_empty());
    let todos: Vec<_> = results
        .iter()
        .flat_map(|r| r.record.todo_items.iter())
        .collect();
    assert!(todos.iter().any(|t| t.description.contains("Design schema")));
    assert!(todos
        .iter()
        .any(|t| t.description.contains("Implement endpoints")));
    assert!(todos.iter().any(|t| t.description.contains("Write tests")));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_search_history_mcp_tool() -> Result<()> {
    reset_conversation_store()?;
    setup_test_conversations(vec![
        ("session-1", "Implement authentication with JWT"),
        ("session-2", "Create user registration API"),
        ("session-3", "Fix database connection pooling"),
    ])
    .await?;

    let store = conversation_store()?;
    let results = store.search_with_scores(simple_embed("authentication JWT"), 5)?;
    assert!(!results.is_empty());
    let matched = results.iter().any(|res| res
        .record
        .content
        .to_lowercase()
        .contains("authentication"));
    assert!(matched);

    Ok(())
}
