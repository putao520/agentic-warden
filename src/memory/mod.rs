//! Dual-mode memory integration with SahomeDB persistence.

mod config;
mod history;

pub use config::MemoryConfig;
pub use history::{
    ConversationHistoryStore, ConversationRecord, ConversationSearchResult, TodoItem,
};
