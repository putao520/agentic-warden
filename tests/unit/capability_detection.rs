//! Tests for MCP client capability detection.

use aiw::mcp::capability_detector::ClientCapabilities;
use rmcp::model::{Implementation, InitializeRequestParam, ProtocolVersion};

#[test]
fn test_from_init_request() {
    let init_request = InitializeRequestParam {
        protocol_version: ProtocolVersion::LATEST,
        capabilities: Default::default(),
        client_info: Implementation {
            name: "claude-code".to_string(),
            title: None,
            version: "2.0.1".to_string(),
            icons: None,
            website_url: None,
        },
    };

    let capabilities = ClientCapabilities::from_init_request(&init_request);

    assert_eq!(capabilities.client_name, "claude-code");
    assert_eq!(capabilities.client_version, "2.0.1");
    // Initially false until tested
    assert!(!capabilities.supports_dynamic_tools);
}

#[test]
fn test_from_init_request_different_clients() {
    let clients = vec![
        ("claude-code", "2.0.1"),
        ("cursor", "0.42.3"),
        ("vscode-mcp", "1.0.0"),
        ("custom-client", "0.1.0"),
    ];

    for (name, version) in clients {
        let init_request = InitializeRequestParam {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: Default::default(),
            client_info: Implementation {
                name: name.to_string(),
                title: None,
                version: version.to_string(),
                icons: None,
                website_url: None,
            },
        };

        let capabilities = ClientCapabilities::from_init_request(&init_request);

        assert_eq!(capabilities.client_name, name);
        assert_eq!(capabilities.client_version, version);
        assert!(!capabilities.supports_dynamic_tools);
    }
}

#[test]
fn test_client_capabilities_clone() {
    let caps = ClientCapabilities {
        supports_dynamic_tools: true,
        client_name: "test-client".to_string(),
        client_version: "1.0.0".to_string(),
    };

    let cloned = caps.clone();

    assert_eq!(cloned.supports_dynamic_tools, caps.supports_dynamic_tools);
    assert_eq!(cloned.client_name, caps.client_name);
    assert_eq!(cloned.client_version, caps.client_version);
}

#[test]
fn test_client_capabilities_debug() {
    let caps = ClientCapabilities {
        supports_dynamic_tools: true,
        client_name: "claude-code".to_string(),
        client_version: "2.0.1".to_string(),
    };

    let debug_str = format!("{:?}", caps);
    assert!(debug_str.contains("claude-code"));
    assert!(debug_str.contains("2.0.1"));
    assert!(debug_str.contains("true"));
}

#[test]
fn test_capabilities_mutation() {
    let mut caps = ClientCapabilities {
        supports_dynamic_tools: false,
        client_name: "test".to_string(),
        client_version: "1.0".to_string(),
    };

    // Simulate successful capability test
    assert!(!caps.supports_dynamic_tools);

    caps.supports_dynamic_tools = true;
    assert!(caps.supports_dynamic_tools);
}
