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
        // 未知版本 - 只保留 NativeBinary 等长补丁
        _ => vec![
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
/// 注意：UltraThink 通过 ToolSearch 补丁已启用（cL()==="firstParty" 被替换为 true）
/// 不需要单独的补丁，因此返回空向量
fn get_ultrathink_patches(_version: &ClaudeVersion) -> Vec<UnifiedPatchPattern> {
    vec![]
}

/// AgentTeams 补丁模式
fn get_agentteams_patches(_version: &ClaudeVersion) -> Vec<UnifiedPatchPattern> {
    // AgentTeams 通过 --agent-teams 参数启用，不需要补丁
    vec![]
}

/// WebSearch 补丁模式
///
/// 注意: WebSearch 的地区限制是通过服务器端或其他逻辑判断的
/// 修改描述文本不能真正启用功能，所以只保留内存补丁用于监控
fn get_websearch_patches(version: &ClaudeVersion) -> Vec<UnifiedPatchPattern> {
    match (version.major, version.minor, version.patch) {
        (2, 1, 72..) => vec![
            // 内存补丁: 仅用于监控，实际地区限制由其他逻辑控制
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
        
        // 验证文件补丁使用等长替换
        let file_patch = patches.iter().find(|p| p.patch_type == PatchType::File).unwrap();
        assert_eq!(file_patch.replace_pattern, Some(b"true/*           */".as_slice()));
    }

    #[test]
    fn test_ultrathink_patches() {
        let version = ClaudeVersion { major: 2, minor: 1, patch: 72 };
        let patches = get_ultrathink_patches(&version);
        assert!(patches.is_empty());  // UltraThink 现在通过 ToolSearch 启用，无独立补丁
    }
}
