//! Hook event handler with FastEmbed integration.

use super::{ClaudeCodeHookInput, ClaudeCodeTranscriptParser};
use crate::memory::{ConversationHistoryStore, ConversationRecord};
use anyhow::{Context, Result};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::path::PathBuf;
use std::sync::Mutex;

/// Handler for Claude Code hook events.
///
/// Coordinates the entire pipeline:
/// 1. Read hook input from stdin
/// 2. Parse JSONL transcript
/// 3. Generate embeddings via FastEmbed
/// 4. Store in SahomeDB vector database
pub struct HookHandler {
    embedder: Mutex<TextEmbedding>,
    store: ConversationHistoryStore,
}

impl HookHandler {
    /// Initialize hook handler with FastEmbed and vector database.
    pub async fn new() -> Result<Self> {
        // Initialize FastEmbed with AllMiniLML6V2 model (384 dimensions)
        let mut init_options = InitOptions::default();
        init_options.model_name = EmbeddingModel::AllMiniLML6V2;
        init_options.show_download_progress = false;
        let embedder = TextEmbedding::try_new(init_options)?;

        // Initialize conversation history store
        let db_path = Self::get_db_path()?;
        let store = ConversationHistoryStore::new(&db_path, 384)?;

        Ok(Self { embedder: Mutex::new(embedder), store })
    }

    /// Get the vector database path from config directory.
    fn get_db_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Failed to get config directory")?
            .join("agentic-warden");

        std::fs::create_dir_all(&config_dir)?;

        Ok(config_dir.join("conversation_history.db"))
    }

    /// Handle hook event from stdin.
    ///
    /// This is the main entry point called by `agentic-warden hooks handle`.
    pub async fn handle_from_stdin(&self) -> Result<()> {
        // 1. Read hook input from stdin
        let input = ClaudeCodeHookInput::from_stdin()
            .context("Failed to parse hook input from stdin")?;

        eprintln!(
            "📥 Received hook: {} for session {}",
            input.hook_event_name, input.session_id
        );

        // 2. Check if session already indexed (deduplication)
        if self.is_session_indexed(&input.session_id).await? {
            eprintln!("✓ Session {} already indexed, skipping", input.session_id);
            return Ok(());
        }

        // 3. Parse JSONL transcript
        let transcript_path = input.transcript_path_buf();
        let messages = ClaudeCodeTranscriptParser::parse_file(&transcript_path)
            .with_context(|| format!("Failed to parse transcript: {:?}", transcript_path))?;

        if messages.is_empty() {
            eprintln!("⚠ No messages found in transcript, skipping");
            return Ok(());
        }

        eprintln!("📄 Parsed {} messages from transcript", messages.len());

        // 4. Generate embeddings in batches
        let contents: Vec<String> = messages.iter().map(|m| m.content.clone()).collect();

        eprintln!("🔮 Generating embeddings...");
        let embeddings = self
            .embedder
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .embed(contents.clone(), None)
            .context("Failed to generate embeddings")?;

        // 5. Store in vector database
        eprintln!("💾 Storing to vector database...");
        for (message, embedding) in messages.iter().zip(embeddings.iter()) {
            let record = ConversationRecord {
                id: uuid::Uuid::new_v4().to_string(),
                session_id: Some(input.session_id.clone()), // Use session_id from stdin
                role: message.role.clone(),
                content: message.content.clone(),
                timestamp: message.timestamp,
                tools_used: vec![],
            };

            self.store.append(record, embedding.to_vec())?;
        }

        println!(
            "✅ Indexed {} messages for session {}",
            messages.len(),
            input.session_id
        );

        Ok(())
    }

    /// Check if a session has already been indexed.
    ///
    /// This prevents duplicate indexing when hooks are triggered multiple times.
    async fn is_session_indexed(&self, session_id: &str) -> Result<bool> {
        // Query for a single record with this session_id
        // If any result found, session is already indexed
        let dummy_embedding = vec![0.0; 384]; // Dummy embedding for query
        let results = self.store.top_conversations(dummy_embedding, 1)?;

        Ok(results
            .iter()
            .any(|r| r.session_id.as_ref() == Some(&session_id.to_string())))
    }
}
