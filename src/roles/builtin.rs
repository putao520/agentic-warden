//! Built-in role definitions
//!
//! This module provides pre-built roles that are bundled with AIW.
//! Role content is embedded directly in the binary using `include_str!`.

use super::Role;
use super::RoleError;
use std::collections::HashMap;

// Chinese (zh-CN) builtin roles
const BUILTIN_ROLES_ZH_CN: &[(&str, &str)] = &[
    ("assistant-programmer", include_str!("builtin/zh-CN/assistant-programmer.md")),
    ("big-data-standards", include_str!("builtin/zh-CN/big-data-standards.md")),
    ("blockchain", include_str!("builtin/zh-CN/blockchain.md")),
    ("common", include_str!("builtin/zh-CN/common.md")),
    ("database-standards", include_str!("builtin/zh-CN/database-standards.md")),
    ("debugger", include_str!("builtin/zh-CN/debugger.md")),
    ("deployment", include_str!("builtin/zh-CN/deployment.md")),
    ("devops", include_str!("builtin/zh-CN/devops.md")),
    ("embedded", include_str!("builtin/zh-CN/embedded.md")),
    ("frontend-standards", include_str!("builtin/zh-CN/frontend-standards.md")),
    ("game", include_str!("builtin/zh-CN/game.md")),
    ("game-unity", include_str!("builtin/zh-CN/game-unity.md")),
    ("game-unreal", include_str!("builtin/zh-CN/game-unreal.md")),
    ("graphics", include_str!("builtin/zh-CN/graphics.md")),
    ("iot", include_str!("builtin/zh-CN/iot.md")),
    ("ml", include_str!("builtin/zh-CN/ml.md")),
    ("mobile-android", include_str!("builtin/zh-CN/mobile-android.md")),
    ("mobile-ios", include_str!("builtin/zh-CN/mobile-ios.md")),
    ("multimedia", include_str!("builtin/zh-CN/multimedia.md")),
    ("quality", include_str!("builtin/zh-CN/quality.md")),
    ("security", include_str!("builtin/zh-CN/security.md")),
    ("testing-standards", include_str!("builtin/zh-CN/testing-standards.md")),
];

// English (en) builtin roles
const BUILTIN_ROLES_EN: &[(&str, &str)] = &[
    ("assistant-programmer", include_str!("builtin/en/assistant-programmer.md")),
    ("big-data-standards", include_str!("builtin/en/big-data-standards.md")),
    ("blockchain", include_str!("builtin/en/blockchain.md")),
    ("common", include_str!("builtin/en/common.md")),
    ("database-standards", include_str!("builtin/en/database-standards.md")),
    ("debugger", include_str!("builtin/en/debugger.md")),
    ("deployment", include_str!("builtin/en/deployment.md")),
    ("devops", include_str!("builtin/en/devops.md")),
    ("embedded", include_str!("builtin/en/embedded.md")),
    ("frontend-standards", include_str!("builtin/en/frontend-standards.md")),
    ("game", include_str!("builtin/en/game.md")),
    ("game-unity", include_str!("builtin/en/game-unity.md")),
    ("game-unreal", include_str!("builtin/en/game-unreal.md")),
    ("graphics", include_str!("builtin/en/graphics.md")),
    ("iot", include_str!("builtin/en/iot.md")),
    ("ml", include_str!("builtin/en/ml.md")),
    ("mobile-android", include_str!("builtin/en/mobile-android.md")),
    ("mobile-ios", include_str!("builtin/en/mobile-ios.md")),
    ("multimedia", include_str!("builtin/en/multimedia.md")),
    ("quality", include_str!("builtin/en/quality.md")),
    ("security", include_str!("builtin/en/security.md")),
    ("testing-standards", include_str!("builtin/en/testing-standards.md")),
];

/// Get multiple builtin roles by names and language
///
/// Returns a tuple of (valid_roles, invalid_role_names).
/// Invalid roles are skipped with their names collected for reporting.
pub fn get_builtin_roles(names: &[&str], lang: &str) -> (Vec<Role>, Vec<String>) {
    let mut valid_roles = Vec::new();
    let mut invalid_names = Vec::new();

    for name in names {
        match get_builtin_role(name, lang) {
            Ok(role) => valid_roles.push(role),
            Err(_) => invalid_names.push(name.to_string()),
        }
    }

    (valid_roles, invalid_names)
}

/// Get a builtin role by name and language
///
/// # Arguments
/// * `name` - Role name (e.g., "common", "debugger")
/// * `lang` - Language code: "en" for English, "zh-CN" for Chinese (default)
///
/// # Returns
/// * `Ok(Role)` - Role with embedded content
/// * `Err(RoleError)` - If role name not found
///
/// # Language Fallback
/// If the requested role is not available in the requested language,
/// this function will fallback to Chinese (zh-CN) version.
pub fn get_builtin_role(name: &str, lang: &str) -> Result<Role, RoleError> {
    // Select role array based on language
    let roles = if lang == "en" {
        BUILTIN_ROLES_EN
    } else {
        BUILTIN_ROLES_ZH_CN
    };

    // Try to find role in requested language
    if let Some((_, content)) = roles.iter().find(|(role_name, _)| *role_name == name) {
        return Ok(parse_role_content(name, content, lang));
    }

    // Fallback to Chinese if not found in English
    if lang == "en" {
        if let Some((_, content)) = BUILTIN_ROLES_ZH_CN.iter().find(|(role_name, _)| *role_name == name) {
            return Ok(parse_role_content(name, content, "zh-CN"));
        }
    }

    Err(RoleError::NotFound(name.to_string()))
}

/// Parse role content and create Role struct
fn parse_role_content(name: &str, content: &str, lang: &str) -> Role {
    // Extract description (first line, remove # prefix)
    let description = content
        .lines()
        .next()
        .unwrap_or("Role")
        .trim_start_matches('#')
        .trim()
        .to_string();

    Role {
        name: name.to_string(),
        description,
        content: content.to_string(),
        file_path: format!("builtin:{}:{}", lang, name).into(),
    }
}

/// List all available builtin roles (from Chinese version as base)
pub fn list_builtin_roles() -> Vec<String> {
    BUILTIN_ROLES_ZH_CN
        .iter()
        .map(|(name, _)| name.to_string())
        .collect()
}

/// Get all builtin roles as a HashMap (Chinese version as base)
pub fn get_all_builtin_roles() -> HashMap<String, String> {
    BUILTIN_ROLES_ZH_CN
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
        // Verify file_path indicates builtin role with language
        let path_str = role.file_path.display().to_string();
        assert!(path_str.starts_with("builtin:zh-CN:"));
    }

    #[test]
    fn test_english_role() {
        let role = get_builtin_role("common", "en").unwrap();
        assert_eq!(role.name, "common");
        assert!(!role.content.is_empty());
        // Verify it's the English version
        assert!(role.content.contains("Common Programming Standards"));
    }

    #[test]
    fn test_all_english_roles_available() {
        // All 22 roles now have English translations
        let role_names = list_builtin_roles();
        for role_name in role_names {
            let role = get_builtin_role(&role_name, "en").unwrap();
            assert_eq!(role.name, role_name);
            // Verify it's the English version (path indicates "en")
            let path_str = role.file_path.display().to_string();
            assert!(path_str.starts_with("builtin:en:"), "{} should use English version", role_name);
        }
    }

    #[test]
    fn test_all_roles_accessible() {
        let all_roles = list_builtin_roles();
        for role_name in all_roles {
            assert!(
                get_builtin_role(&role_name, "zh-CN").is_ok(),
                "Role {} should be accessible in Chinese",
                role_name
            );
        }
    }

    #[test]
    fn test_get_builtin_roles_all_valid() {
        let (valid_roles, invalid_names) = get_builtin_roles(&["common", "security", "debugger"], "en");
        assert_eq!(valid_roles.len(), 3);
        assert!(invalid_names.is_empty());
        assert_eq!(valid_roles[0].name, "common");
        assert_eq!(valid_roles[1].name, "security");
        assert_eq!(valid_roles[2].name, "debugger");
    }

    #[test]
    fn test_get_builtin_roles_some_invalid() {
        let (valid_roles, invalid_names) = get_builtin_roles(&["common", "nonexistent", "security"], "en");
        assert_eq!(valid_roles.len(), 2);
        assert_eq!(invalid_names.len(), 1);
        assert_eq!(invalid_names[0], "nonexistent");
    }

    #[test]
    fn test_get_builtin_roles_all_invalid() {
        let (valid_roles, invalid_names) = get_builtin_roles(&["fake1", "fake2"], "en");
        assert!(valid_roles.is_empty());
        assert_eq!(invalid_names.len(), 2);
    }

    #[test]
    fn test_get_builtin_roles_empty() {
        let (valid_roles, invalid_names) = get_builtin_roles(&[], "en");
        assert!(valid_roles.is_empty());
        assert!(invalid_names.is_empty());
    }
}
