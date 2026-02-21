//! Git worktree management utilities
//!
//! Extracted from `commands/ai_cli.rs` for reuse across CLI and MCP contexts.

use anyhow::{anyhow, Result};
use std::path::PathBuf;

/// Check if the given path is inside a git repository.
pub(crate) fn check_git_repository(work_dir: &PathBuf) -> Result<()> {
    match git2::Repository::discover(work_dir) {
        Ok(_) => Ok(()),
        Err(e) if e.class() == git2::ErrorClass::Repository => Err(anyhow!(
            "Error: Not a git repository. Please initialize git first:\n  cd {} && git init",
            work_dir.display()
        )),
        Err(e) => Err(anyhow!(
            "Error: Unable to access git repository: {}",
            e.message()
        )),
    }
}

/// Generate an 8-char lowercase hex string for worktree naming.
pub(crate) fn generate_worktree_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    format!("{:08x}", timestamp % 0x100000000)
}

/// Create a git worktree from the given repository path.
///
/// Returns `(worktree_path, branch_name, commit_hash)`.
pub(crate) fn create_worktree(repo_path: &PathBuf) -> Result<(PathBuf, String, String)> {
    let repo = git2::Repository::open(repo_path)
        .map_err(|e| anyhow!("Failed to open git repository: {}", e.message()))?;

    let head = repo
        .head()
        .map_err(|e| anyhow!("Failed to get HEAD: {}", e.message()))?;
    let commit = head
        .peel_to_commit()
        .map_err(|e| anyhow!("Failed to peel to commit: {}", e.message()))?;
    let commit_hash = commit.id().to_string();

    let branch_name = head.shorthand().unwrap_or("HEAD").to_string();

    let worktree_id = generate_worktree_id();
    let worktree_path = PathBuf::from("/tmp").join(format!("aiw-worktree-{}", worktree_id));

    if worktree_path.exists() {
        return Err(anyhow!(
            "Worktree directory already exists: {}. Please remove it manually.",
            worktree_path.display()
        ));
    }

    let status = std::process::Command::new("git")
        .args(["worktree", "add", "-b"])
        .arg(&format!("aiw-worktree-{}", worktree_id))
        .arg(&worktree_path)
        .arg(&commit_hash)
        .current_dir(repo_path)
        .output()
        .map_err(|e| anyhow!("Failed to execute git worktree command: {}", e))?;

    if !status.status.success() {
        let stderr = String::from_utf8_lossy(&status.stderr);
        return Err(anyhow!("Failed to create worktree: {}", stderr));
    }

    Ok((worktree_path, branch_name, commit_hash))
}
