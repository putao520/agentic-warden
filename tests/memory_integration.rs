use agentic_warden::memory::{ConversationHistoryStore, ConversationRecord, MemoryConfig};
use chrono::Utc;

#[test]
fn test_memory_config_defaults() {
    let config = MemoryConfig::default();
    assert!(!config.fastembed_model.is_empty());
    assert!(config.sahome_db_path.is_absolute());
    assert!(config.llm_endpoint.starts_with("http"));
    assert!(config.validate().is_ok());
}

#[tokio::test]
async fn test_conversation_history_roundtrip() {
    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("sahome.db");
    let store = ConversationHistoryStore::new(&db_path, 4).expect("history store");

    let record = ConversationRecord {
        id: "test".into(),
        session_id: Some("session-123".into()),
        role: "user".into(),
        content: "integration test conversation".into(),
        timestamp: Utc::now(),
        tools_used: vec!["mcp::echo".into()],
    };

    store
        .append(record.clone(), vec![0.1, 0.2, 0.3, 0.4])
        .expect("append");

    let results = store
        .top_conversations(vec![0.1, 0.2, 0.3, 0.4], 1)
        .expect("search");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].session_id, record.session_id);
    assert_eq!(results[0].content, record.content);
}
