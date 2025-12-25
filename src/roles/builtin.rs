//! Built-in role definitions
//!
//! This module provides pre-built roles that are bundled with AIW.
//! Role content is embedded directly in the binary using `include_str!`.

use super::Role;
use super::RoleError;
use std::collections::HashMap;

// Builtin role content (embedded at compile time)
const BUILTIN_ROLES: &[(&str, &str)] = &[
    ("assistant-programmer", include_str!("builtin/assistant-programmer.md")),
    ("big-data-standards", include_str!("builtin/big-data-standards.md")),
    ("blockchain", include_str!("builtin/blockchain.md")),
    ("common", include_str!("builtin/common.md")),
    ("database-standards", include_str!("builtin/database-standards.md")),
    ("debugger", include_str!("builtin/debugger.md")),
    ("deployment", include_str!("builtin/deployment.md")),
    ("devops", include_str!("builtin/devops.md")),
    ("embedded", include_str!("builtin/embedded.md")),
    ("frontend-standards", include_str!("builtin/frontend-standards.md")),
    ("game", include_str!("builtin/game.md")),
    ("game-unity", include_str!("builtin/game-unity.md")),
    ("game-unreal", include_str!("builtin/game-unreal.md")),
    ("graphics", include_str!("builtin/graphics.md")),
    ("iot", include_str!("builtin/iot.md")),
    ("ml", include_str!("builtin/ml.md")),
    ("mobile-android", include_str!("builtin/mobile-android.md")),
    ("mobile-ios", include_str!("builtin/mobile-ios.md")),
    ("multimedia", include_str!("builtin/multimedia.md")),
    ("quality", include_str!("builtin/quality.md")),
    ("security", include_str!("builtin/security.md")),
    ("testing-standards", include_str!("builtin/testing-standards.md")),
];

/// Get a builtin role by name
///
/// # Arguments
/// * `name` - Role name (e.g., "common", "debugger")
/// * `_lang` - Language code (reserved for future use, currently only "zh-CN" supported)
///
/// # Returns
/// * `Ok(Role)` - Role with embedded content
/// * `Err(RoleError)` - If role name not found
pub fn get_builtin_role(name: &str, _lang: &str) -> Result<Role, RoleError> {
    BUILTIN_ROLES
        .iter()
        .find(|(role_name, _)| *role_name == name)
        .map(|(_, content)| {
            // Extract description (first line, remove # prefix)
            let description = content
                .lines()
                .next()
                .unwrap_or("Role")
                .trim_start_matches('#')
                .trim()
                .to_string();

            Ok(Role {
                name: name.to_string(),
                description,
                content: content.to_string(),
                file_path: format!("builtin:{}", name).into(), // Indicate this is a builtin role
            })
        })
        .unwrap_or_else(|| Err(RoleError::NotFound(name.to_string())))
}

/// List all available builtin roles
pub fn list_builtin_roles() -> Vec<String> {
    BUILTIN_ROLES
        .iter()
        .map(|(name, _)| name.to_string())
        .collect()
}

/// Get all builtin roles as a HashMap
pub fn get_all_builtin_roles() -> HashMap<String, String> {
    BUILTIN_ROLES
        .iter()
        .map(|(name, content)| (name.to_string(), content.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_builtin_roles() {
        let roles = list_builtin_roles();
        // Should have 22 builtin roles
        assert_eq!(roles.len(), 22);
        // Should contain common role
        assert!(roles.contains(&"common".to_string()));
    }

    #[test]
    fn test_get_builtin_role() {
        let role = get_builtin_role("common", "zh-CN").unwrap();
        assert_eq!(role.name, "common");
        assert!(!role.content.is_empty());
        assert!(!role.description.is_empty());
    }

    #[test]
    fn test_get_nonexistent_role() {
        let result = get_builtin_role("nonexistent", "zh-CN");
        assert!(result.is_err());
    }

    #[test]
    fn test_role_content_embedded() {
        let role = get_builtin_role("debugger", "zh-CN").unwrap();
        // Verify content contains expected keywords
        assert!(role.content.contains("调试"));
        // Verify file_path indicates builtin role
        assert!(role.file_path.display().to_string().starts_with("builtin:"));
    }

    #[test]
    fn test_all_roles_accessible() {
        let all_roles = list_builtin_roles();
        for role_name in all_roles {
            assert!(
                get_builtin_role(&role_name, "zh-CN").is_ok(),
                "Role {} should be accessible",
                role_name
            );
        }
    }
}
