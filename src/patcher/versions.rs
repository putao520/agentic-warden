//! Claude CLI version signature database
//!
//! Each verified version has its patch signatures stored here.
//! Add new versions ONLY after verifying the patch patterns work.
//!
//! Signatures are platform-specific, as different operating systems may have
//! different function names due to minification variations.

/// Supported platforms for Claude CLI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    LinuxX64,
    LinuxArm64,
    Win32X64,
    Win32Arm64,
    DarwinX64,
    DarwinArm64,
}

impl Platform {
    /// Detect the current platform at compile time
    pub fn detect() -> Self {
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        return Platform::LinuxX64;
        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        return Platform::LinuxArm64;
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        return Platform::Win32X64;
        #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
        return Platform::Win32Arm64;
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        return Platform::DarwinX64;
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        return Platform::DarwinArm64;

        // Fallback for unsupported platforms (should not happen in practice)
        #[cfg(not(any(
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "aarch64"),
            all(target_os = "windows", target_arch = "x86_64"),
            all(target_os = "windows", target_arch = "aarch64"),
            all(target_os = "macos", target_arch = "x86_64"),
            all(target_os = "macos", target_arch = "aarch64"),
        )))]
        compile_error!("Unsupported platform for Claude CLI patching");
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", platform_short_name(*self))
    }
}

/// Get short name for a platform
fn platform_short_name(platform: Platform) -> &'static str {
    match platform {
        Platform::LinuxX64 => "linux-x64",
        Platform::LinuxArm64 => "linux-arm64",
        Platform::Win32X64 => "win32-x64",
        Platform::Win32Arm64 => "win32-arm64",
        Platform::DarwinX64 => "darwin-x64",
        Platform::DarwinArm64 => "darwin-arm64",
    }
}

/// Patch signatures for a specific platform
#[derive(Debug, Clone)]
pub struct PlatformSignature {
    /// Platform this signature applies to
    pub platform: Platform,
    /// The function name used in firstParty check
    pub fn_name: &'static str,
    /// File patch: search pattern
    pub file_search: &'static [u8],
    /// File patch: replace pattern (must be same length)
    pub file_replace: &'static [u8],
    /// Memory patch: search pattern
    pub mem_search: &'static [u8],
    /// Memory patch: byte to write
    pub mem_patch_byte: u8,
    /// Memory patch: offset
    pub mem_patch_offset: usize,
    /// Memory patch: string search pattern
    pub mem_str_search: &'static [u8],
    /// Memory patch: string replacement byte
    pub mem_str_patch_byte: u8,
}

/// Version signatures with platform support
#[derive(Debug, Clone)]
pub struct VersionEntry {
    /// Platform-specific signatures
    pub platforms: &'static [PlatformSignature],
}

/// Legacy alias for backward compatibility
pub type VersionSignature = PlatformSignature;

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

    /// Look up the signature for this version and current platform
    pub fn signature(&self) -> Option<&'static PlatformSignature> {
        get_signature(self.major, self.minor, self.patch, Platform::detect())
    }

    /// Look up the signature for a specific platform
    pub fn signature_for_platform(&self, platform: Platform) -> Option<&'static PlatformSignature> {
        get_signature(self.major, self.minor, self.patch, platform)
    }

    /// Check if this version has verified patch signatures for the current platform
    pub fn is_supported(&self) -> bool {
        self.signature().is_some()
    }

    /// Check if this version has verified patch signatures for a specific platform
    pub fn is_supported_on_platform(&self, platform: Platform) -> bool {
        self.signature_for_platform(platform).is_some()
    }

    /// List all supported version strings with platforms
    pub fn supported_versions_str() -> String {
        let mut result = Vec::new();
        for (ma, mi, pa, entry) in VERSION_DB {
            for sig in entry.platforms {
                result.push(format!("{}.{}.{} ({})", ma, mi, pa, platform_short_name(sig.platform)));
            }
        }
        result.join(", ")
    }

    /// Get all platforms supported for this version
    pub fn supported_platforms(&self) -> Vec<Platform> {
        VERSION_DB
            .iter()
            .find(|(ma, mi, pa, _)| *ma == self.major && *mi == self.minor && *pa == self.patch)
            .map(|(_, _, _, entry)| entry.platforms.iter().map(|sig| sig.platform).collect())
            .unwrap_or_default()
    }
}

/// Version signature database
/// Format: (major, minor, patch, VersionEntry)
/// Add new entries ONLY after verifying patch patterns against the actual binary
const VERSION_DB: &[(u32, u32, u32, VersionEntry)] = &[
    (2, 1, 72, VersionEntry {
        platforms: &[
            // Linux x64
            PlatformSignature {
                platform: Platform::LinuxX64,
                fn_name: "cL",
                file_search: b"cL()===\"firstParty\"",
                file_replace: b"true/*           */",
                mem_search: b"cL()===\"firstParty\"",
                mem_patch_byte: b'!',
                mem_patch_offset: 7,
                mem_str_search: b"firstParty",
                mem_str_patch_byte: b'!',
            },
            // Linux arm64
            PlatformSignature {
                platform: Platform::LinuxArm64,
                fn_name: "F8",
                file_search: b"F8()===\"firstParty\"",
                file_replace: b"true/*           */",
                mem_search: b"F8()===\"firstParty\"",
                mem_patch_byte: b'!',
                mem_patch_offset: 7,
                mem_str_search: b"firstParty",
                mem_str_patch_byte: b'!',
            },
            // Windows x64
            PlatformSignature {
                platform: Platform::Win32X64,
                fn_name: "Qf",
                file_search: b"Qf()===\"firstParty\"",
                file_replace: b"true/*           */",
                mem_search: b"Qf()===\"firstParty\"",
                mem_patch_byte: b'!',
                mem_patch_offset: 7,
                mem_str_search: b"firstParty",
                mem_str_patch_byte: b'!',
            },
            // macOS (Apple Silicon)
            PlatformSignature {
                platform: Platform::DarwinArm64,
                fn_name: "F8",
                file_search: b"F8()===\"firstParty\"",
                file_replace: b"true/*           */",
                mem_search: b"F8()===\"firstParty\"",
                mem_patch_byte: b'!',
                mem_patch_offset: 7,
                mem_str_search: b"firstParty",
                mem_str_patch_byte: b'!',
            },
        ],
    }),
    (2, 1, 73, VersionEntry {
        platforms: &[
            // Linux x64
            PlatformSignature {
                platform: Platform::LinuxX64,
                fn_name: "lL",
                file_search: b"lL()===\"firstParty\"",
                file_replace: b"true/*           */",
                mem_search: b"lL()===\"firstParty\"",
                mem_patch_byte: b'!',
                mem_patch_offset: 7,
                mem_str_search: b"firstParty",
                mem_str_patch_byte: b'!',
            },
            // Linux arm64
            PlatformSignature {
                platform: Platform::LinuxArm64,
                fn_name: "l8",
                file_search: b"l8()===\"firstParty\"",
                file_replace: b"true/*           */",
                mem_search: b"l8()===\"firstParty\"",
                mem_patch_byte: b'!',
                mem_patch_offset: 7,
                mem_str_search: b"firstParty",
                mem_str_patch_byte: b'!',
            },
            // Windows x64
            PlatformSignature {
                platform: Platform::Win32X64,
                fn_name: "lf",
                file_search: b"lf()===\"firstParty\"",
                file_replace: b"true/*           */",
                mem_search: b"lf()===\"firstParty\"",
                mem_patch_byte: b'!',
                mem_patch_offset: 7,
                mem_str_search: b"firstParty",
                mem_str_patch_byte: b'!',
            },
            // macOS (Apple Silicon)
            PlatformSignature {
                platform: Platform::DarwinArm64,
                fn_name: "l8",
                file_search: b"l8()===\"firstParty\"",
                file_replace: b"true/*           */",
                mem_search: b"l8()===\"firstParty\"",
                mem_patch_byte: b'!',
                mem_patch_offset: 7,
                mem_str_search: b"firstParty",
                mem_str_patch_byte: b'!',
            },
        ],
    }),
    (2, 1, 74, VersionEntry {
        platforms: &[
            // Linux x64
            PlatformSignature {
                platform: Platform::LinuxX64,
                fn_name: "aL",
                file_search: b"aL()===\"firstParty\"",
                file_replace: b"true/*           */",
                mem_search: b"aL()===\"firstParty\"",
                mem_patch_byte: b'!',
                mem_patch_offset: 7,
                mem_str_search: b"firstParty",
                mem_str_patch_byte: b'!',
            },
            // Linux arm64
            PlatformSignature {
                platform: Platform::LinuxArm64,
                fn_name: "o8",
                file_search: b"o8()===\"firstParty\"",
                file_replace: b"true/*           */",
                mem_search: b"o8()===\"firstParty\"",
                mem_patch_byte: b'!',
                mem_patch_offset: 7,
                mem_str_search: b"firstParty",
                mem_str_patch_byte: b'!',
            },
            // Windows x64
            PlatformSignature {
                platform: Platform::Win32X64,
                fn_name: "tf",
                file_search: b"tf()===\"firstParty\"",
                file_replace: b"true/*           */",
                mem_search: b"tf()===\"firstParty\"",
                mem_patch_byte: b'!',
                mem_patch_offset: 7,
                mem_str_search: b"firstParty",
                mem_str_patch_byte: b'!',
            },
            // macOS (Apple Silicon)
            PlatformSignature {
                platform: Platform::DarwinArm64,
                fn_name: "o8",
                file_search: b"o8()===\"firstParty\"",
                file_replace: b"true/*           */",
                mem_search: b"o8()===\"firstParty\"",
                mem_patch_byte: b'!',
                mem_patch_offset: 7,
                mem_str_search: b"firstParty",
                mem_str_patch_byte: b'!',
            },
        ],
    }),
    (2, 1, 75, VersionEntry {
        platforms: &[
            // Linux x64
            PlatformSignature {
                platform: Platform::LinuxX64,
                fn_name: "BL",
                file_search: b"BL()===\"firstParty\"",
                file_replace: b"true/*           */",
                mem_search: b"BL()===\"firstParty\"",
                mem_patch_byte: b'!',
                mem_patch_offset: 7,
                mem_str_search: b"firstParty",
                mem_str_patch_byte: b'!',
            },
            // Linux arm64
            PlatformSignature {
                platform: Platform::LinuxArm64,
                fn_name: "B8",
                file_search: b"B8()===\"firstParty\"",
                file_replace: b"true/*           */",
                mem_search: b"B8()===\"firstParty\"",
                mem_patch_byte: b'!',
                mem_patch_offset: 7,
                mem_str_search: b"firstParty",
                mem_str_patch_byte: b'!',
            },
            // Windows x64
            PlatformSignature {
                platform: Platform::Win32X64,
                fn_name: "Bf",
                file_search: b"Bf()===\"firstParty\"",
                file_replace: b"true/*           */",
                mem_search: b"Bf()===\"firstParty\"",
                mem_patch_byte: b'!',
                mem_patch_offset: 7,
                mem_str_search: b"firstParty",
                mem_str_patch_byte: b'!',
            },
            // macOS (Apple Silicon)
            PlatformSignature {
                platform: Platform::DarwinArm64,
                fn_name: "B8",
                file_search: b"B8()===\"firstParty\"",
                file_replace: b"true/*           */",
                mem_search: b"B8()===\"firstParty\"",
                mem_patch_byte: b'!',
                mem_patch_offset: 7,
                mem_str_search: b"firstParty",
                mem_str_patch_byte: b'!',
            },
        ],
    }),
    (2, 1, 76, VersionEntry {
        platforms: &[
            // Linux x64
            PlatformSignature {
                platform: Platform::LinuxX64,
                fn_name: "QL",
                file_search: b"QL()===\"firstParty\"",
                file_replace: b"true/*           */",
                mem_search: b"QL()===\"firstParty\"",
                mem_patch_byte: b'!',
                mem_patch_offset: 7,
                mem_str_search: b"firstParty",
                mem_str_patch_byte: b'!',
            },
            // Linux arm64
            PlatformSignature {
                platform: Platform::LinuxArm64,
                fn_name: "Q8",
                file_search: b"Q8()===\"firstParty\"",
                file_replace: b"true/*           */",
                mem_search: b"Q8()===\"firstParty\"",
                mem_patch_byte: b'!',
                mem_patch_offset: 7,
                mem_str_search: b"firstParty",
                mem_str_patch_byte: b'!',
            },
            // Windows x64
            PlatformSignature {
                platform: Platform::Win32X64,
                fn_name: "cf",
                file_search: b"cf()===\"firstParty\"",
                file_replace: b"true/*           */",
                mem_search: b"cf()===\"firstParty\"",
                mem_patch_byte: b'!',
                mem_patch_offset: 7,
                mem_str_search: b"firstParty",
                mem_str_patch_byte: b'!',
            },
            // macOS (Apple Silicon)
            PlatformSignature {
                platform: Platform::DarwinArm64,
                fn_name: "Q8",
                file_search: b"Q8()===\"firstParty\"",
                file_replace: b"true/*           */",
                mem_search: b"Q8()===\"firstParty\"",
                mem_patch_byte: b'!',
                mem_patch_offset: 7,
                mem_str_search: b"firstParty",
                mem_str_patch_byte: b'!',
            },
        ],
    }),
];

/// Look up signature by version tuple and platform
fn get_signature(major: u32, minor: u32, patch: u32, platform: Platform) -> Option<&'static PlatformSignature> {
    VERSION_DB
        .iter()
        .find(|(ma, mi, pa, _)| *ma == major && *mi == minor && *pa == patch)
        .and_then(|(_, _, _, entry)| {
            entry.platforms.iter().find(|sig| sig.platform == platform)
        })
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
        let platform = Platform::detect();
        assert!(v.is_supported_on_platform(platform));
        let sig = v.signature_for_platform(platform);
        assert!(sig.is_some());
        // Verify function name is set (actual value depends on platform)
        assert!(!sig.unwrap().fn_name.is_empty());
    }

    #[test]
    fn test_unsupported_version() {
        let v = ClaudeVersion::from_string("3.0.0").unwrap();
        assert!(!v.is_supported());
        assert!(v.signature().is_none());
    }

    #[test]
    fn test_signature_lengths_match() {
        for (_, _, _, entry) in VERSION_DB {
            for sig in entry.platforms {
                assert_eq!(
                    sig.file_search.len(),
                    sig.file_replace.len(),
                    "File patch must be equal length for {} on {:?}",
                    sig.fn_name,
                    sig.platform
                );
            }
        }
    }

    #[test]
    fn test_platform_detect() {
        let platform = Platform::detect();
        // Verify we can detect a platform (doesn't matter which one for this test)
        match platform {
            Platform::LinuxX64 | Platform::LinuxArm64 |
            Platform::Win32X64 | Platform::Win32Arm64 |
            Platform::DarwinX64 | Platform::DarwinArm64 => {
                // Valid platform detected
            }
        }
    }

    #[test]
    fn test_supported_platforms_for_version() {
        let v = ClaudeVersion::from_string("2.1.72").unwrap();
        let platforms = v.supported_platforms();
        // Should have at least one platform
        assert!(!platforms.is_empty());
    }

    #[test]
    fn test_platform_display() {
        assert_eq!(format!("{}", Platform::LinuxX64), "linux-x64");
        assert_eq!(format!("{}", Platform::Win32X64), "win32-x64");
        assert_eq!(format!("{}", Platform::DarwinArm64), "darwin-arm64");
    }
}
