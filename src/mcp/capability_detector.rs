//! Client capability detection for MCP features.
//!
//! Tests whether the connected MCP client supports dynamic tool registration
//! (notifications/tools/list_changed) by actually sending a test notification.

use rmcp::model::{InitializeRequestParam, ToolListChangedNotification};
use rmcp::service::server::ClientSink;
use std::time::Duration;
use tokio::time::timeout;

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

    /// Test if client supports dynamic tool registration by sending a test notification.
    /// This should be called after the initialization handshake is complete.
    pub async fn test_dynamic_tools_support(peer: &ClientSink) -> bool {
        // Create a test notification
        let notification = ToolListChangedNotification::default();

        // Try to send it with a short timeout
        let test_result = timeout(
            Duration::from_millis(500),
            peer.send_notification(notification.into())
        ).await;

        match test_result {
            Ok(Ok(())) => {
                eprintln!("   ✅ Dynamic tools test: SUCCESS");
                true
            }
            Ok(Err(e)) => {
                eprintln!("   ⚠️  Dynamic tools test: FAILED - {}", e);
                false
            }
            Err(_) => {
                eprintln!("   ⚠️  Dynamic tools test: TIMEOUT");
                false
            }
        }
    }
}
