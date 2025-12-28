//! AI CLI role management and parsing utilities.
//!
//! Roles are stored as Markdown files under `~/.aiw/role/` with the following
//! structure:
//!
//! ```text
//! <description>
//! ------------
//! <content>
//! ```
//! - Description: short summary shown in listings.
//! - Content: full role prompt used by downstream tools.

pub mod builtin;

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;

const ROLE_FILE_EXTENSION: &str = "md";
const DESCRIPTION_CONTENT_DELIMITER: &str = "------------";
const MAX_ROLE_FILE_BYTES: u64 = 1_048_576; // 1MB safety limit

/// Parsed role file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    pub name: String,
    pub description: String,
    pub content: String,
    pub file_path: PathBuf,
}

/// Lightweight role info returned by listing APIs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema)]
pub struct RoleInfo {
    pub name: String,
    pub description: String,
    pub file_path: String,
}

impl Role {
    pub fn as_info(&self) -> RoleInfo {
        RoleInfo {
            name: self.name.clone(),
            description: self.description.clone(),
            file_path: self.file_path.display().to_string(),
        }
    }
}

/// Errors returned by the role manager.
#[derive(Error, Debug)]
pub enum RoleError {
    #[error("Role not found: {0}")]
    NotFound(String),
    #[error("Invalid role name: {message}")]
    InvalidName { message: String },
    #[error("Role file outside allowed directory: {path}")]
    PathTraversal { path: String },
    #[error("Role file too large (>1MB): {size} bytes at {path}")]
    FileTooLarge { path: String, size: u64 },
    #[error("Role file is not valid UTF-8: {path}")]
    InvalidEncoding { path: String },
    #[error("Role file format error at {path}: {details}")]
    InvalidFormat { path: String, details: String },
    #[error("Home directory not available")]
    HomeDirectoryUnavailable,
    #[error("I/O error for {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: io::Error,
    },
}

pub type RoleResult<T> = Result<T, RoleError>;

/// Role manager responsible for loading role definitions from disk.
#[derive(Debug, Clone)]
pub struct RoleManager {
    base_dir: PathBuf,
}

impl RoleManager {
    /// Create manager using the default `~/.aiw/role/` directory.
    pub fn new() -> RoleResult<Self> {
        let home_dir = dirs::home_dir().ok_or(RoleError::HomeDirectoryUnavailable)?;
        Ok(Self {
            base_dir: home_dir.join(".aiw").join("role"),
        })
    }

    /// Create manager pointing to a custom directory (used in tests).
    pub fn with_base_dir<P: Into<PathBuf>>(base_dir: P) -> RoleResult<Self> {
        Ok(Self {
            base_dir: base_dir.into(),
        })
    }

    /// List and parse all roles in the base directory.
    pub fn list_all_roles(&self) -> RoleResult<Vec<Role>> {
        if !self.base_dir.exists() {
            return Ok(Vec::new());
        }
        let base_dir = self.canonicalize_base_dir()?;

        let mut roles = Vec::new();
        for entry in WalkDir::new(&base_dir).follow_links(false).into_iter() {
            let entry = entry.map_err(|err| RoleError::Io {
                path: err
                    .path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| base_dir.display().to_string()),
                source: err
                    .into_io_error()
                    .unwrap_or_else(|| io::Error::new(io::ErrorKind::Other, "walkdir error")),
            })?;

            if !entry.file_type().is_file() {
                continue;
            }

            if !Self::is_markdown_file(entry.path()) {
                continue;
            }

            let role = self.parse_role_file(entry.path(), &base_dir)?;
            roles.push(role);
        }

        roles.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(roles)
    }

    /// Retrieve a single role by name (without extension).
    pub fn get_role(&self, name: &str) -> RoleResult<Role> {
        let normalized_name = self.normalize_and_validate_name(name)?;
        let candidate_path = self
            .base_dir
            .join(format!("{normalized_name}.{ROLE_FILE_EXTENSION}"));

        if !candidate_path.exists() {
            return Err(RoleError::NotFound(normalized_name.to_string()));
        }

        let base_dir = self.canonicalize_base_dir()?;
        self.parse_role_file(&candidate_path, &base_dir)
    }

    /// Retrieve multiple roles by names.
    ///
    /// Returns a tuple of (valid_roles, invalid_role_names).
    /// Invalid roles are skipped with their names collected for reporting.
    pub fn get_roles(&self, names: &[&str]) -> (Vec<Role>, Vec<String>) {
        let mut valid_roles = Vec::new();
        let mut invalid_names = Vec::new();

        for name in names {
            match self.get_role(name) {
                Ok(role) => valid_roles.push(role),
                Err(_) => invalid_names.push(name.to_string()),
            }
        }

        (valid_roles, invalid_names)
    }

    fn canonicalize_base_dir(&self) -> RoleResult<PathBuf> {
        fs::canonicalize(&self.base_dir).map_err(|source| RoleError::Io {
            path: self.base_dir.display().to_string(),
            source,
        })
    }

    fn parse_role_file(&self, path: &Path, base_dir: &Path) -> RoleResult<Role> {
        let canonical_path = fs::canonicalize(path).map_err(|source| RoleError::Io {
            path: path.display().to_string(),
            source,
        })?;

        if !canonical_path.starts_with(base_dir) {
            return Err(RoleError::PathTraversal {
                path: canonical_path.display().to_string(),
            });
        }

        let metadata = fs::metadata(&canonical_path).map_err(|source| RoleError::Io {
            path: canonical_path.display().to_string(),
            source,
        })?;

        if metadata.len() > MAX_ROLE_FILE_BYTES {
            return Err(RoleError::FileTooLarge {
                path: canonical_path.display().to_string(),
                size: metadata.len(),
            });
        }

        let bytes = fs::read(&canonical_path).map_err(|source| RoleError::Io {
            path: canonical_path.display().to_string(),
            source,
        })?;

        let raw_content = String::from_utf8(bytes).map_err(|_| RoleError::InvalidEncoding {
            path: canonical_path.display().to_string(),
        })?;

        let (description, content) =
            Self::split_description_and_content(&raw_content, &canonical_path)?;

        let name = canonical_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| RoleError::InvalidFormat {
                path: canonical_path.display().to_string(),
                details: "Filename must be valid UTF-8".to_string(),
            })?
            .to_string();

        Ok(Role {
            name,
            description,
            content,
            file_path: canonical_path,
        })
    }

    fn split_description_and_content(raw: &str, path: &Path) -> RoleResult<(String, String)> {
        if let Some(idx) = raw.find(DESCRIPTION_CONTENT_DELIMITER) {
            let (description_part, content_part) = raw.split_at(idx);
            let content = content_part
                .trim_start_matches(DESCRIPTION_CONTENT_DELIMITER)
                .trim_start_matches(|c| c == '\n' || c == '\r')
                .trim()
                .to_string();

            let description = description_part.trim().to_string();
            return Ok((description, content));
        }

        Err(RoleError::InvalidFormat {
            path: path.display().to_string(),
            details: "Missing description/content delimiter '------------'".to_string(),
        })
    }

    fn normalize_and_validate_name<'a>(&self, name: &'a str) -> RoleResult<Cow<'a, str>> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(RoleError::InvalidName {
                message: "Role name cannot be empty".to_string(),
            });
        }
        if trimmed.len() > 255 {
            return Err(RoleError::InvalidName {
                message: "Role name exceeds 255 characters".to_string(),
            });
        }

        let normalized = trimmed
            .strip_suffix(format!(".{ROLE_FILE_EXTENSION}").as_str())
            .unwrap_or(trimmed);

        // Strict character set validation: only allow [A-Za-z0-9_-]
        for ch in normalized.chars() {
            if !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-' {
                return Err(RoleError::InvalidName {
                    message: format!(
                        "Role name contains invalid character '{}'. Only alphanumeric characters (A-Z, a-z, 0-9), underscores (_), and hyphens (-) are allowed.",
                        ch
                    ),
                });
            }
        }

        let path = Path::new(normalized);
        for component in path.components() {
            if !matches!(component, Component::Normal(_)) {
                return Err(RoleError::InvalidName {
                    message: "Role name must not contain path separators or traversal".to_string(),
                });
            }
        }

        Ok(Cow::from(normalized))
    }

    fn is_markdown_file(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case(ROLE_FILE_EXTENSION))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_role_file(dir: &Path, name: &str, description: &str, content: &str) {
        let file_path = dir.join(format!("{}.md", name));
        let file_content = format!("{}\n------------\n{}", description, content);
        std::fs::write(file_path, file_content).unwrap();
    }

    #[test]
    fn test_get_roles_all_valid() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();

        create_test_role_file(base_dir, "role1", "Role 1 Description", "Role 1 Content");
        create_test_role_file(base_dir, "role2", "Role 2 Description", "Role 2 Content");
        create_test_role_file(base_dir, "role3", "Role 3 Description", "Role 3 Content");

        let manager = RoleManager::with_base_dir(base_dir).unwrap();
        let (valid_roles, invalid_names) = manager.get_roles(&["role1", "role2", "role3"]);

        assert_eq!(valid_roles.len(), 3);
        assert!(invalid_names.is_empty());
        assert_eq!(valid_roles[0].name, "role1");
        assert_eq!(valid_roles[1].name, "role2");
        assert_eq!(valid_roles[2].name, "role3");
    }

    #[test]
    fn test_get_roles_some_invalid() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();

        create_test_role_file(base_dir, "valid1", "Valid 1", "Content 1");
        create_test_role_file(base_dir, "valid2", "Valid 2", "Content 2");

        let manager = RoleManager::with_base_dir(base_dir).unwrap();
        let (valid_roles, invalid_names) = manager.get_roles(&["valid1", "invalid1", "valid2", "invalid2"]);

        assert_eq!(valid_roles.len(), 2);
        assert_eq!(invalid_names.len(), 2);
        assert_eq!(valid_roles[0].name, "valid1");
        assert_eq!(valid_roles[1].name, "valid2");
        assert!(invalid_names.contains(&"invalid1".to_string()));
        assert!(invalid_names.contains(&"invalid2".to_string()));
    }

    #[test]
    fn test_get_roles_all_invalid() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();

        let manager = RoleManager::with_base_dir(base_dir).unwrap();
        let (valid_roles, invalid_names) = manager.get_roles(&["nonexistent1", "nonexistent2"]);

        assert!(valid_roles.is_empty());
        assert_eq!(invalid_names.len(), 2);
    }

    #[test]
    fn test_get_roles_empty_input() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();

        let manager = RoleManager::with_base_dir(base_dir).unwrap();
        let (valid_roles, invalid_names) = manager.get_roles(&[]);

        assert!(valid_roles.is_empty());
        assert!(invalid_names.is_empty());
    }

    #[test]
    fn test_single_role_backward_compatible() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();

        create_test_role_file(base_dir, "single", "Single Role", "Single Content");

        let manager = RoleManager::with_base_dir(base_dir).unwrap();

        // Single role via get_role
        let role = manager.get_role("single").unwrap();
        assert_eq!(role.name, "single");

        // Single role via get_roles (should work the same)
        let (valid_roles, invalid_names) = manager.get_roles(&["single"]);
        assert_eq!(valid_roles.len(), 1);
        assert!(invalid_names.is_empty());
        assert_eq!(valid_roles[0].name, "single");
    }
}
