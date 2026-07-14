//! Grok CLI 版本工具

/// Grok CLI 版本号（用于显示 + patch 锚点诊断，不参与 patch 签名查找）
#[derive(Debug, Clone)]
pub struct GrokVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl std::fmt::Display for GrokVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl GrokVersion {
    pub fn from_string(s: &str) -> Option<Self> {
        // grok --version 输出: "grok 0.2.99 (b1b49ccb71)"
        let v = s.split_whitespace().find(|t| t.contains('.'))?;
        let parts: Vec<&str> = v.split('.').collect();
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
    fn test_grok_version_parsing() {
        let v = GrokVersion::from_string("grok 0.2.99 (b1b49ccb71)").unwrap();
        assert_eq!(v.major, 0);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 99);
    }

    #[test]
    fn test_grok_version_display() {
        let v = GrokVersion { major: 0, minor: 2, patch: 99 };
        assert_eq!(format!("{}", v), "0.2.99");
    }

    #[test]
    fn test_grok_version_invalid() {
        assert!(GrokVersion::from_string("no version here").is_none());
    }
}
