//! 功能补丁注册表
//!
//! 为每个功能定义版本相关的补丁模式

use crate::patcher::types::{FeatureType, PatchType, UnifiedPatchPattern};
use crate::patcher::versions::ClaudeVersion;

/// 获取功能的补丁模式
pub fn get_feature_patches(
    feature: FeatureType,
    version: &ClaudeVersion,
) -> Vec<UnifiedPatchPattern> {
    match feature {
        FeatureType::ToolSearch => get_toolsearch_patches(version),
        FeatureType::UltraThink => get_ultrathink_patches(version),
        FeatureType::AgentTeams => get_agentteams_patches(version),
        FeatureType::WebSearch => get_websearch_patches(version),
        FeatureType::PersistentMemory => get_persistent_memory_patches(version),
    }
}

/// ToolSearch 补丁模式
///
/// 修复策略：让功能始终启用，而不是条件反转
///
/// 原始代码分析：
///   if (cL()==="firstParty") { ...启用功能... }
///
/// 错误的补丁方法（已废弃）：
///   if (cL()!=="firstParty") { ...启用功能... }
///   问题：当使用官方 API 时，cL() 返回 "firstParty"，条件为 false，功能被禁用！
///
/// 正确的修复方法：
///   文件补丁：将条件替换为 true
///   内存补丁：组合策略 - 反转运算符 + 修改字符串
fn get_toolsearch_patches(version: &ClaudeVersion) -> Vec<UnifiedPatchPattern> {
    match (version.major, version.minor, version.patch) {
        // 2.1.72+: cL 函数
        (2, 1, 72..) => vec![
            // 文件补丁：将整个条件替换为 true
            UnifiedPatchPattern {
                feature: FeatureType::ToolSearch,
                patch_type: PatchType::File,
                search_pattern: b"cL()==\"firstParty\"",
                replace_pattern: Some(b"true".as_slice()),
                patch_byte: None,
                patch_offset: None,
                description: "ToolSearch file patch: Replace condition with true (always enable)",
            },
            // NativeBinary 文件补丁：三等号 + 等长替换（19字节）
            UnifiedPatchPattern {
                feature: FeatureType::ToolSearch,
                patch_type: PatchType::File,
                search_pattern: b"cL()===\"firstParty\"",
                replace_pattern: Some(b"true/*           */"),
                patch_byte: None,
                patch_offset: None,
                description: "ToolSearch file patch (native binary): Equal-length replacement",
            },
            // 内存补丁：双重策略
            // 1. 将 === 改为 !== (反转比较)
            // 2. 将 "firstParty" 改为 "!irstParty" (修改第一个字符)
            // 结果：cL()!=="!irstParty"，当 cL() 返回 "firstParty" 时为 true
            UnifiedPatchPattern {
                feature: FeatureType::ToolSearch,
                patch_type: PatchType::Memory,
                search_pattern: b"cL()==\"firstParty\"",
                replace_pattern: None,
                patch_byte: Some(b'!'),
                patch_offset: Some(6),
                description: "ToolSearch memory patch: Invert === to !==",
            },
            UnifiedPatchPattern {
                feature: FeatureType::ToolSearch,
                patch_type: PatchType::Memory,
                search_pattern: b"firstParty",
                replace_pattern: None,
                patch_byte: Some(b'!'),  // 将 'f' 改为 '!'
                patch_offset: Some(0),
                description: "ToolSearch memory patch: Modify 'firstParty' to '!irstParty'",
            },
        ],
        // 2.1.71 及之前: O8 函数
        (2, 0..=1, 0..=71) => vec![
            UnifiedPatchPattern {
                feature: FeatureType::ToolSearch,
                patch_type: PatchType::File,
                search_pattern: b"O8()==\"firstParty\"",
                replace_pattern: Some(b"true".as_slice()),
                patch_byte: None,
                patch_offset: None,
                description: "ToolSearch file patch (O8): Replace with true",
            },
            UnifiedPatchPattern {
                feature: FeatureType::ToolSearch,
                patch_type: PatchType::Memory,
                search_pattern: b"O8()==\"firstParty\"",
                replace_pattern: None,
                patch_byte: Some(b'!'),
                patch_offset: Some(6),
                description: "ToolSearch memory patch (O8): Invert === to !==",
            },
            UnifiedPatchPattern {
                feature: FeatureType::ToolSearch,
                patch_type: PatchType::Memory,
                search_pattern: b"firstParty",
                replace_pattern: None,
                patch_byte: Some(b'!'),
                patch_offset: Some(0),
                description: "ToolSearch memory patch (O8): Modify firstParty string",
            },
        ],
        // 未知版本 - 返回常用模式
        _ => vec![
            UnifiedPatchPattern {
                feature: FeatureType::ToolSearch,
                patch_type: PatchType::File,
                search_pattern: b"cL()==\"firstParty\"",
                replace_pattern: Some(b"true".as_slice()),
                patch_byte: None,
                patch_offset: None,
                description: "ToolSearch unlock (cL, unknown version): Replace with true",
            },
            UnifiedPatchPattern {
                feature: FeatureType::ToolSearch,
                patch_type: PatchType::File,
                search_pattern: b"O8()==\"firstParty\"",
                replace_pattern: Some(b"true".as_slice()),
                patch_byte: None,
                patch_offset: None,
                description: "ToolSearch unlock (O8, unknown version): Replace with true",
            },
            // NativeBinary 等长补丁
            UnifiedPatchPattern {
                feature: FeatureType::ToolSearch,
                patch_type: PatchType::File,
                search_pattern: b"cL()===\"firstParty\"",
                replace_pattern: Some(b"true/*           */"),
                patch_byte: None,
                patch_offset: None,
                description: "ToolSearch unlock (cL native, unknown version): Equal-length replacement",
            },
        ],
    }
}

/// UltraThink 补丁模式
///
/// 修复策略：同样使用始终启用的方法
fn get_ultrathink_patches(version: &ClaudeVersion) -> Vec<UnifiedPatchPattern> {
    match (version.major, version.minor, version.patch) {
        (2, 1, 72..) => vec![
            // UltraThink wS() 函数中的 firstParty 检查
            // 原始：return cL()==="firstParty"
            // 文件补丁：直接返回 true
            UnifiedPatchPattern {
                feature: FeatureType::UltraThink,
                patch_type: PatchType::File,
                search_pattern: b"return cL()==\"firstParty\"",
                replace_pattern: Some(b"return true".as_slice()),
                patch_byte: None,
                patch_offset: None,
                description: "UltraThink file patch: Always return true",
            },
            // NativeBinary 文件补丁：三等号 + 等长替换（26字节）
            UnifiedPatchPattern {
                feature: FeatureType::UltraThink,
                patch_type: PatchType::File,
                search_pattern: b"return cL()===\"firstParty\"",
                replace_pattern: Some(b"return true/*           */"),
                patch_byte: None,
                patch_offset: None,
                description: "UltraThink file patch (native binary): Equal-length replacement",
            },
            // 内存补丁：反转条件 + 修改字符串
            UnifiedPatchPattern {
                feature: FeatureType::UltraThink,
                patch_type: PatchType::Memory,
                search_pattern: b"return cL()===\"firstParty\"",
                replace_pattern: None,
                patch_byte: Some(b'!'),
                patch_offset: Some(14),
                description: "UltraThink memory patch: Invert === to !==",
            },
        ],
        _ => vec![],
    }
}

/// AgentTeams 补丁模式
fn get_agentteams_patches(_version: &ClaudeVersion) -> Vec<UnifiedPatchPattern> {
    // AgentTeams 通过 --agent-teams 参数启用，不需要补丁
    vec![]
}

/// WebSearch 补丁模式
fn get_websearch_patches(version: &ClaudeVersion) -> Vec<UnifiedPatchPattern> {
    match (version.major, version.minor, version.patch) {
        (2, 1, 72..) => vec![
            // 文件补丁: 修改描述文本，移除 "only available in the US"
            UnifiedPatchPattern {
                feature: FeatureType::WebSearch,
                patch_type: PatchType::File,
                search_pattern: b"Web search is only available in the US",
                replace_pattern: Some(b"Web search is available globally"),
                patch_byte: None,
                patch_offset: None,
                description: "WebSearch: Remove US-only restriction from description",
            },
            // NativeBinary 文件补丁：等长替换（38字节）
            UnifiedPatchPattern {
                feature: FeatureType::WebSearch,
                patch_type: PatchType::File,
                search_pattern: b"Web search is only available in the US",
                replace_pattern: Some(b"Web search is available in all regions"),
                patch_byte: None,
                patch_offset: None,
                description: "WebSearch file patch (native binary): Equal-length replacement",
            },
            // 内存补丁: 同样的修改（如果在内存中）
            UnifiedPatchPattern {
                feature: FeatureType::WebSearch,
                patch_type: PatchType::Memory,
                search_pattern: b"Web search is only available in the US",
                replace_pattern: None,
                patch_byte: Some(b'g'), // 将 'U' 改为 'g' (global)
                patch_offset: Some(28), // "Web search is only available in the US" 中 'U' 的位置
                description: "WebSearch: Memory patch for US-only restriction",
            },
            // 额外的补丁: 寻找可能的地区检测函数
            UnifiedPatchPattern {
                feature: FeatureType::WebSearch,
                patch_type: PatchType::Memory,
                search_pattern: b"getTimezoneOffset()>300",
                replace_pattern: Some(b"getTimezoneOffset()<=300"),
                patch_byte: None,
                patch_offset: None,
                description: "WebSearch: Bypass timezone-based region check",
            },
        ],
        _ => vec![],
    }
}

/// 持久代理内存补丁模式
fn get_persistent_memory_patches(version: &ClaudeVersion) -> Vec<UnifiedPatchPattern> {
    match (version.major, version.minor, version.patch) {
        (2, 1, 72..) => vec![
            UnifiedPatchPattern {
                feature: FeatureType::PersistentMemory,
                patch_type: PatchType::Memory,
                search_pattern: b"tengu_swinburne_dune",
                replace_pattern: None,
                patch_byte: None,
                patch_offset: None,
                description: "Persistent memory feature flag (monitoring only)",
            },
        ],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toolsearch_patches() {
        let version = ClaudeVersion { major: 2, minor: 1, patch: 72 };
        let patches = get_toolsearch_patches(&version);
        assert!(!patches.is_empty());
        assert_eq!(patches[0].feature, FeatureType::ToolSearch);
        
        // 验证文件补丁使用 true 而非反转条件
        let file_patch = patches.iter().find(|p| p.patch_type == PatchType::File).unwrap();
        assert_eq!(file_patch.replace_pattern, Some(b"true".as_slice()));
    }

    #[test]
    fn test_old_version_patches() {
        let version = ClaudeVersion { major: 2, minor: 1, patch: 70 };
        let patches = get_toolsearch_patches(&version);
        assert!(!patches.is_empty());
        // Old version uses O8 function
        assert!(patches.iter().any(|p| {
            String::from_utf8_lossy(p.search_pattern).contains("O8")
        }));
    }

    #[test]
    fn test_ultrathink_patches() {
        let version = ClaudeVersion { major: 2, minor: 1, patch: 72 };
        let patches = get_ultrathink_patches(&version);
        assert!(!patches.is_empty());
        
        // 验证文件补丁返回 true
        let file_patch = patches.iter().find(|p| p.patch_type == PatchType::File).unwrap();
        assert_eq!(file_patch.replace_pattern, Some(b"return true".as_slice()));
    }
}
