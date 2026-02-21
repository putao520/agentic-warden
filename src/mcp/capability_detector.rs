//! Client capability detection for MCP features.
//!
//! Tests whether the connected MCP client supports dynamic tool registration
//! (notifications/tools/list_changed) by actually sending a test notification.

use rmcp::model::InitializeRequestParams;
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
    pub fn from_init_request(request: &InitializeRequestParams) -> Self {
        Self {
            supports_dynamic_tools: false, // Will be tested later
            client_name: request.client_info.name.clone(),
            client_version: request.client_info.version.clone(),
        }
    }

    /// Test if client supports dynamic tool registration.
    ///
    /// # Current Implementation (Placeholder)
    ///
    /// This function currently always returns `true` without performing actual capability testing.
    ///
    /// **Why**: The rmcp library does not provide an API to send test notifications to clients.
    /// Attempting to send notifications during initialization could interfere with the
    /// connection handshake.
    ///
    /// **Workaround**: We assume all clients support dynamic tools and rely on a query-based
    /// approach:
    /// - After generating new tools via `intelligent_route`, clients should re-query the
    ///   tool list using the standard MCP `tools/list` method
    /// - This is compatible with all MCP clients as per the protocol specification
    ///
    /// **Future**: If rmcp adds a proper notification API or capability negotiation protocol,
    /// this function should be updated to perform actual testing.
    pub async fn test_dynamic_tools_support(_peer: &Peer<RoleServer>) -> bool {
        eprintln!("   🔍 [DEBUG] test_dynamic_tools_support called!");
        eprintln!("   ✅ Dynamic tools: ENABLED (query-based mode)");
        true
    }
}
