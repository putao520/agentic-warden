//! Patch registry - generates UnifiedPatchPattern from version signatures

use crate::patcher::types::{FeatureType, PatchType, UnifiedPatchPattern};
use crate::patcher::versions::ClaudeVersion;

/// Get patches for a feature based on version signature
pub fn get_feature_patches(
    feature: FeatureType,
    version: &ClaudeVersion,
) -> Vec<UnifiedPatchPattern> {
    match feature {
        FeatureType::ToolSearch => get_toolsearch_patches(version),
        // These features are covered by ToolSearch patch or don't need patches
        FeatureType::UltraThink | FeatureType::WebSearch | FeatureType::AgentTeams => vec![],
        FeatureType::PersistentMemory => get_persistent_memory_patches(version),
    }
}

/// Generate ToolSearch patches from version signature
fn get_toolsearch_patches(version: &ClaudeVersion) -> Vec<UnifiedPatchPattern> {
    let sig = match version.signature() {
        Some(s) => s,
        None => return vec![],
    };

    vec![
        // File patch: equal-length replacement
        UnifiedPatchPattern {
            feature: FeatureType::ToolSearch,
            patch_type: PatchType::File,
            search_pattern: sig.file_search,
            replace_pattern: Some(sig.file_replace),
            patch_byte: None,
            patch_offset: None,
            description: "ToolSearch file patch: equal-length replacement",
        },
        // Memory patch 1: invert operator
        UnifiedPatchPattern {
            feature: FeatureType::ToolSearch,
            patch_type: PatchType::Memory,
            search_pattern: sig.mem_search,
            replace_pattern: None,
            patch_byte: Some(sig.mem_patch_byte),
            patch_offset: Some(sig.mem_patch_offset),
            description: "ToolSearch memory patch: invert operator",
        },
        // Memory patch 2: mutate string
        UnifiedPatchPattern {
            feature: FeatureType::ToolSearch,
            patch_type: PatchType::Memory,
            search_pattern: sig.mem_str_search,
            replace_pattern: None,
            patch_byte: Some(sig.mem_str_patch_byte),
            patch_offset: Some(0),
            description: "ToolSearch memory patch: mutate string",
        },
    ]
}

/// PersistentMemory patches (monitoring only)
fn get_persistent_memory_patches(version: &ClaudeVersion) -> Vec<UnifiedPatchPattern> {
    // Only for versions that have a known signature
    if version.signature().is_none() {
        return vec![];
    }

    vec![
        UnifiedPatchPattern {
            feature: FeatureType::PersistentMemory,
            patch_type: PatchType::Memory,
            search_pattern: b"tengu_swinburne_dune",
            replace_pattern: None,
            patch_byte: None,
            patch_offset: None,
            description: "Persistent memory feature flag (monitoring only)",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toolsearch_patches_supported() {
        let version = ClaudeVersion { major: 2, minor: 1, patch: 72 };
        let patches = get_toolsearch_patches(&version);
        assert_eq!(patches.len(), 3); // 1 file + 2 memory
        assert_eq!(patches[0].patch_type, PatchType::File);
        assert_eq!(patches[1].patch_type, PatchType::Memory);
        assert_eq!(patches[2].patch_type, PatchType::Memory);

        // Verify equal-length file patch
        let file_patch = &patches[0];
        assert_eq!(file_patch.search_pattern.len(), file_patch.replace_pattern.unwrap().len());
    }

    #[test]
    fn test_toolsearch_patches_unsupported() {
        let version = ClaudeVersion { major: 3, minor: 0, patch: 0 };
        let patches = get_toolsearch_patches(&version);
        assert!(patches.is_empty());
    }

    #[test]
    fn test_ultrathink_no_patches() {
        let version = ClaudeVersion { major: 2, minor: 1, patch: 72 };
        let patches = get_feature_patches(FeatureType::UltraThink, &version);
        assert!(patches.is_empty());
    }
}
