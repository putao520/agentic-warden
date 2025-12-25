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
// Note: Currently 4 roles have English translations, others fallback to zh-CN
const BUILTIN_ROLES_EN: &[(&str, &str)] = &[
    ("assistant-programmer", include_str!("builtin/en/assistant-programmer.md")),
    ("common", include_str!("builtin/en/common.md")),
    ("deployment", include_str!("builtin/en/deployment.md")),
    ("quality", include_str!("builtin/en/quality.md")),
    // Other roles will be added as translations are completed
    // For now, missing English roles will fallback to Chinese version
];

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
    fn test_english_fallback_to_chinese() {
        // debugger is not yet translated to English, should fallback to Chinese
        let role = get_builtin_role("debugger", "en").unwrap();
        assert_eq!(role.name, "debugger");
        // Should be Chinese content since English doesn't exist
        assert!(role.content.contains("调试"));
        // Path should indicate it's the Chinese fallback
        let path_str = role.file_path.display().to_string();
        assert!(path_str.starts_with("builtin:zh-CN:"));
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
}
