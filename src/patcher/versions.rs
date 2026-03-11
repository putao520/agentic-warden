//! Claude CLI version signature database
//!
//! Each verified version has its patch signatures stored here.
//! Add new versions ONLY after verifying the patch patterns work.

/// Patch signatures for a specific Claude CLI version
#[derive(Debug, Clone)]
pub struct VersionSignature {
    /// The function name used in firstParty check (e.g. "cL", "O8")
    pub fn_name: &'static str,
    /// File patch: search pattern
    pub file_search: &'static [u8],
    /// File patch: replace pattern (must be same length as file_search)
    pub file_replace: &'static [u8],
    /// Memory patch: search pattern for operator inversion
    pub mem_search: &'static [u8],
    /// Memory patch: byte to write for operator inversion
    pub mem_patch_byte: u8,
    /// Memory patch: offset within mem_search to patch
    pub mem_patch_offset: usize,
    /// Memory patch: search pattern for string mutation
    pub mem_str_search: &'static [u8],
    /// Memory patch: replacement byte for first char of string
    pub mem_str_patch_byte: u8,
}

/// Claude CLI version
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

    /// Look up the signature for this version
    pub fn signature(&self) -> Option<&'static VersionSignature> {
        get_signature(self.major, self.minor, self.patch)
    }

    /// Check if this version has verified patch signatures
    pub fn is_supported(&self) -> bool {
        self.signature().is_some()
    }

    /// List all supported version strings
    pub fn supported_versions_str() -> String {
        VERSION_DB
            .iter()
            .map(|(ma, mi, pa, _)| format!("{}.{}.{}", ma, mi, pa))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Version signature database
/// Format: (major, minor, patch, signature)
/// Add new entries ONLY after verifying patch patterns against the actual binary
const VERSION_DB: &[(u32, u32, u32, VersionSignature)] = &[
    (2, 1, 72, VersionSignature {
        fn_name: "cL",
        file_search: b"cL()===\"firstParty\"",
        file_replace: b"true/*           */",
        mem_search: b"cL()===\"firstParty\"",
        mem_patch_byte: b'!',
        mem_patch_offset: 7,
        mem_str_search: b"firstParty",
        mem_str_patch_byte: b'!',
    }),
];

/// Look up signature by version tuple
fn get_signature(major: u32, minor: u32, patch: u32) -> Option<&'static VersionSignature> {
    VERSION_DB
        .iter()
        .find(|(ma, mi, pa, _)| *ma == major && *mi == minor && *pa == patch)
        .map(|(_, _, _, sig)| sig)
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
    fn test_supported_version() {
        let v = ClaudeVersion::from_string("2.1.72").unwrap();
        assert!(v.is_supported());
        assert!(v.signature().is_some());
        assert_eq!(v.signature().unwrap().fn_name, "cL");
    }

    #[test]
    fn test_unsupported_version() {
        let v = ClaudeVersion::from_string("3.0.0").unwrap();
        assert!(!v.is_supported());
        assert!(v.signature().is_none());
    }

    #[test]
    fn test_signature_lengths_match() {
        for (_, _, _, sig) in VERSION_DB {
            assert_eq!(
                sig.file_search.len(),
                sig.file_replace.len(),
                "File patch must be equal length for {}",
                sig.fn_name
            );
        }
    }
}
