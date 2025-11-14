//! Client capability detection for MCP features.
//!
//! Detects whether the connected MCP client supports dynamic tool registration
//! (notifications/tools/list_changed).

use rmcp::model::InitializeRequestParam;

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
    /// Detect client capabilities from initialize request
    pub fn from_init_request(request: &InitializeRequestParam) -> Self {
        let client_name = request.client_info.name.clone();
        let client_version = request.client_info.version.clone();

        let supports_dynamic_tools = Self::is_known_to_support(&client_name, &client_version);

        Self {
            supports_dynamic_tools,
            client_name,
            client_version,
        }
    }

    /// Check if a client is known to support dynamic tools
    fn is_known_to_support(name: &str, version: &str) -> bool {
        let name_lower = name.to_lowercase();

        // Claude Code 2.0.0+ supports dynamic tool registration
        if name_lower.contains("claude") || name_lower.contains("claude-code") {
            return Self::version_gte(version, "2.0.0");
        }

        // Cursor IDE supports dynamic tools
        if name_lower.contains("cursor") {
            return true;
        }

        // VS Code with MCP extension
        if name_lower.contains("vscode") || name_lower.contains("code") {
            return true;
        }

        // Conservative default: unknown clients use fallback mode
        false
    }

    /// Simple version comparison (major.minor.patch >= target)
    fn version_gte(version: &str, target: &str) -> bool {
        let parse_version = |v: &str| -> Option<(u32, u32, u32)> {
            let parts: Vec<&str> = v.split('.').collect();
            if parts.len() < 2 {
                return None;
            }
            Some((
                parts[0].parse().ok()?,
                parts.get(1)?.parse().ok()?,
                parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0),
            ))
        };

        match (parse_version(version), parse_version(target)) {
            (Some(v), Some(t)) => v >= t,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(ClientCapabilities::version_gte("2.0.0", "2.0.0"));
        assert!(ClientCapabilities::version_gte("2.1.0", "2.0.0"));
        assert!(ClientCapabilities::version_gte("2.0.5", "2.0.0"));
        assert!(!ClientCapabilities::version_gte("1.9.9", "2.0.0"));
    }

    #[test]
    fn test_known_clients() {
        assert!(ClientCapabilities::is_known_to_support("claude-code", "2.0.1"));
        assert!(ClientCapabilities::is_known_to_support("Claude Code", "2.1.0"));
        assert!(!ClientCapabilities::is_known_to_support("claude-code", "1.9.0"));
        assert!(ClientCapabilities::is_known_to_support("cursor", "0.1.0"));
        assert!(!ClientCapabilities::is_known_to_support("unknown-client", "1.0.0"));
    }
}
