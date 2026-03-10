//! Claude CLI 版本相关补丁配置
//!
//! 由于代码混淆，每个版本的函数名可能不同。
//! 此模块维护版本到补丁模式的映射。

/// Claude CLI 版本信息
#[derive(Debug, Clone)]
pub struct ClaudeVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ClaudeVersion {
    pub fn from_string(s: &str) -> Option<Self> {
        // 格式: "2.1.72" 或类似
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() >= 3 {
            let major = parts[0].parse().ok()?;
            let minor = parts[1].parse().ok()?;
            let patch = parts[2].parse().ok()?;
            Some(Self { major, minor, patch })
        } else {
            None
        }
    }
}

/// 补丁模式配置
#[derive(Debug, Clone)]
pub struct PatchPattern {
    /// 函数名（可能是混淆后的，如 "O8", "cL" 等）
    pub function_name: &'static str,
    /// 完整模式: `function_name()==="firstParty"`
    pub full_pattern: &'static [u8],
    /// 短模式: `function_name()==="firstParty"&&(`
    pub short_pattern: &'static [u8],
    /// 在模式中第三个 `=` 的偏移位置
    pub equals_offset: usize,
}

/// 版本到补丁模式的映射
///
/// | Claude 版本 | 函数名 | 状态 |
/// |-------------|--------|------|
/// | <= 2.1.71   | O8     | 已弃用 |
/// | 2.1.72+     | cL     | 当前 |
/// | 未来版本    | 未知   | 需检测 |
pub fn get_patch_pattern(version: &ClaudeVersion) -> Option<PatchPattern> {
    match (version.major, version.minor, version.patch) {
        // 2.1.72 及以后: 使用 cL 函数
        (2, 1, 72..) => Some(PatchPattern {
            function_name: "cL",
            full_pattern: b"cL()===\"firstParty\"",
            short_pattern: b"cL()===\"firstParty\"",
            equals_offset: 6, // c(0) L(1) ((2) )(3) =(4) =(5) =(6)
        }),
        // 2.1.71 及之前: 使用 O8 函数
        (2, 0..=1, 0..=71) => Some(PatchPattern {
            function_name: "O8",
            full_pattern: b"O8()===\"firstParty\"",
            short_pattern: b"O8()===\"firstParty\"",
            equals_offset: 6, // O(0) 8(1) ((2) )(3) =(4) =(5) =(6)
        }),
        // 未知版本: 尝试动态检测
        _ => None,
    }
}

/// 动态检测补丁模式
///
/// 当版本未知时，尝试搜索常见的模式
pub fn detect_patch_pattern() -> Vec<PatchPattern> {
    vec![
        // 尝试 cL (当前版本 2.1.72+)
        PatchPattern {
            function_name: "cL",
            full_pattern: b"cL()===\"firstParty\"",
            short_pattern: b"cL()===\"firstParty\"",
            equals_offset: 6,
        },
        // 尝试 O8 (旧版本)
        PatchPattern {
            function_name: "O8",
            full_pattern: b"O8()===\"firstParty\"",
            short_pattern: b"O8()===\"firstParty\"",
            equals_offset: 6,
        },
        // 未来可以添加更多可能的函数名
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        let v = ClaudeVersion::from_string("2.1.72").unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.minor, 1);
        assert_eq!(v.patch, 72);
    }

    #[test]
    fn test_version_pattern_mapping() {
        let v = ClaudeVersion::from_string("2.1.72").unwrap();
        let pattern = get_patch_pattern(&v).unwrap();
        assert_eq!(pattern.function_name, "cL");

        let v_old = ClaudeVersion::from_string("2.1.70").unwrap();
        let pattern_old = get_patch_pattern(&v_old).unwrap();
        assert_eq!(pattern_old.function_name, "O8");
    }

    #[test]
    fn test_unknown_version() {
        let v = ClaudeVersion::from_string("3.0.0").unwrap();
        assert!(get_patch_pattern(&v).is_none());
    }
}
