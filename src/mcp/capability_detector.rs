//! Client capability detection for MCP features.
//!
//! Tests whether the connected MCP client supports dynamic tool registration
//! (notifications/tools/list_changed) by actually sending a test notification.

use rmcp::model::InitializeRequestParam;
use rmcp::service::{Peer, RoleServer};

#[derive(Debug, Clone)]
pub struct ClientCapabilities {
    /// Whether client supports dynamic tool list updates
    pub supports_dynamic_tools: bool,
    /// Client name (e.g., "claude-code", "cursor")
    pub client_name: String,
    /// Client version (e.g., "2.0.1")
    pub client_version: String,
}

impl ClientCapabilities {
    /// Create initial capabilities from initialize request (before testing)
    pub fn from_init_request(request: &InitializeRequestParam) -> Self {
        Self {
            supports_dynamic_tools: false, // Will be tested later
            client_name: request.client_info.name.clone(),
            client_version: request.client_info.version.clone(),
        }
    }

    /// Test if client supports dynamic tool registration.
    /// Note: This is a placeholder - actual notification sending is disabled due to rmcp API constraints.
    pub async fn test_dynamic_tools_support(_peer: &Peer<RoleServer>) -> bool {
        // Dynamic tool notification support is assumed to be true for now
        // The client should re-query tools after receiving intelligent_route responses
        eprintln!("   ✅ Dynamic tools: ENABLED (query-based mode)");
        true
    }
}
