//! Claude CLI 版本工具与 max-token patch 常量
//!
//! 提供 ClaudeVersion 类型（用于显示版本号）以及 MaxContextTokens patch
//! 所需的通用 regex / 校验 / 编码工具。max-token patch 通过变量名无关的
//! regex 通用匹配 Claude CLI 常量块，跨版本稳定，无需维护版本签名数据库。

/// 通用 max-token patch 模式：变量名无关，跨版本稳定
///
/// 匹配 Claude CLI 二进制中的常量块，以前两个 `=200000` 锚定，
/// 后续到 `;` 前任意内容（元素个数可变）：
///
/// - 195/196: `var X=200000,Y=200000,Z=20000,W=32000,Q=128000;`（5 元素）
/// - 197: `var X=200000,Y=200000,Z=20000,W=32000,Q=128000,FIi;`（6 元素，末尾无值变量）
/// - 198: `var X=200000,Y=200000,Z=32000,Q=128000;`（4 元素，无 20000）
///
/// 两个 200000 分别是：
/// - 第一个：默认 context token 上限（如 `YOt`）
/// - 第二个：autoCompact 阈值（如 `Pte`）
///
/// 变量名因 minification 跨版本可能变化，故用 `[a-zA-Z_$][a-zA-Z0-9_$]*`
/// 通配。`[^;]*;` 兜底 197 的无值变量 `FIi` 和 198 的 4 元素。等长替换
/// （6 位十进制数）只改前两个 200000，多余元素保留不动，不破坏后续偏移。
pub const MAX_CONTEXT_TOKENS_SEARCH_REGEX: &str =
    r"var [a-zA-Z_$][a-zA-Z0-9_$]*=200000,[a-zA-Z_$][a-zA-Z0-9_$]*=200000[^;]*;";

/// 校验 max_tokens 值合法（6 位十进制数，100000~999999，保证等长替换）
///
/// 200000→500000 都是 6 字节，等长替换不破坏二进制后续偏移。
/// 值必须落在 [100000, 999999] 区间，否则替换后会改变长度。
pub fn validate_max_context_tokens(n: u32) -> Result<(), String> {
    if !(100000..=999999).contains(&n) {
        return Err(format!(
            "max_context_tokens must be 6 digits (100000~999999), got {}",
            n
        ));
    }
    Ok(())
}

/// 将 6 位数值编码为 6 字节 ASCII（不带校验，调用方需先 validate）
pub fn encode_max_context_tokens(n: u32) -> [u8; 6] {
    let mut buf = [0u8; 6];
    let s = format!("{}", n);
    let b = s.as_bytes();
    // 取最后 6 位 ASCII
    let start = b.len().saturating_sub(6);
    buf.copy_from_slice(&b[start..]);
    buf
}

/// Claude CLI 版本号（仅用于显示，不参与 patch 签名查找）
#[derive(Debug, Clone)]
pub struct ClaudeVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl std::fmt::Display for ClaudeVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl ClaudeVersion {
    pub fn from_string(s: &str) -> Option<Self> {
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
    fn test_version_parsing_invalid() {
        assert!(ClaudeVersion::from_string("2.1").is_none());
        assert!(ClaudeVersion::from_string("abc").is_none());
    }

    #[test]
    fn test_version_display() {
        let v = ClaudeVersion {
            major: 2,
            minor: 1,
            patch: 72,
        };
        assert_eq!(format!("{}", v), "2.1.72");
    }

    #[test]
    fn test_validate_max_context_tokens() {
        assert!(validate_max_context_tokens(100000).is_ok());
        assert!(validate_max_context_tokens(500000).is_ok());
        assert!(validate_max_context_tokens(999999).is_ok());
        assert!(validate_max_context_tokens(99999).is_err());
        assert!(validate_max_context_tokens(1000000).is_err());
    }

    #[test]
    fn test_encode_max_context_tokens() {
        assert_eq!(&encode_max_context_tokens(500000), b"500000");
        assert_eq!(&encode_max_context_tokens(300000), b"300000");
        assert_eq!(&encode_max_context_tokens(100000), b"100000");
    }

    #[test]
    fn test_max_context_tokens_regex_compiles() {
        // regex 必须可编译
        let re = regex::bytes::Regex::new(MAX_CONTEXT_TOKENS_SEARCH_REGEX);
        assert!(re.is_ok(), "regex must compile: {:?}", re);
    }

    #[test]
    fn test_max_context_tokens_regex_matches_sample() {
        let re = regex::bytes::Regex::new(MAX_CONTEXT_TOKENS_SEARCH_REGEX).unwrap();

        // 4 个版本的真实常量块样本（前两个 200000 锚定，后续到 ; 前任意内容）
        let samples: &[&[u8]] = &[
            // 195: 5 元素
            b"var YOt=200000,Pte=200000,Evi=20000,Wkd=32000,qkd=128000;",
            // 196: 5 元素（变量名不同）
            b"var uNt=200000,hne=200000,PIi=20000,$Pd=32000,OPd=128000;",
            // 197: 6 元素，末尾 FIi 无值
            b"var uNt=200000,yne=200000,jIi=20000,zPd=32000,KPd=128000,FIi;",
            // 198: 4 元素，无 20000
            b"var UBt=200000,One=200000,EFd=32000,AFd=128000;",
        ];

        for (i, sample) in samples.iter().enumerate() {
            let m = re.find(sample);
            assert!(m.is_some(), "regex must match version {} sample", 195 + i);
            let m = m.unwrap();
            assert_eq!(m.start(), 0, "version {} match must start at 0", 195 + i);
            assert_eq!(m.end(), sample.len(), "version {} match must cover full block", 195 + i);
        }

        // 197 末尾无值变量 FIi 必须被 [^;]* 兜底命中
        let sample_197 = b"var uNt=200000,yne=200000,jIi=20000,zPd=32000,KPd=128000,FIi;";
        let m = re.find(sample_197).expect("197 must match");
        assert!(m.as_bytes().ends_with(b"FIi;"));

        // 198 只有 4 元素（无 20000）也必须命中
        let sample_198 = b"var UBt=200000,One=200000,EFd=32000,AFd=128000;";
        let m = re.find(sample_198).expect("198 must match");
        assert!(!m.as_bytes().windows(6).any(|w| w == b"20000"));
    }
}
