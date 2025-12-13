//! 版本信息管理
//!
//! 提供版本相关的功能和信息

#![allow(dead_code)] // 版本管理功能，当前未使用但保留作为未来功能

use std::process::Command;

/// 获取构建信息
#[derive(Debug)]
pub struct BuildInfo {
    pub version: String,
    pub commit_hash: String,
    pub build_date: String,
    pub rust_version: String,
}

impl BuildInfo {
    /// 获取当前构建信息
    pub fn get() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            commit_hash: get_git_hash(),
            build_date: get_build_date(),
            rust_version: get_rust_version(),
        }
    }

    /// 打印版本信息
    pub fn print(&self) {
        println!("agentic-warden {}", self.version);
        println!("Commit: {}", self.commit_hash);
        println!("Built on: {}", self.build_date);
        println!("Rust version: {}", self.rust_version);
    }
}

/// 获取 Git 提交哈希
fn get_git_hash() -> String {
    Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// 获取构建日期
fn get_build_date() -> String {
    option_env!("VERGEN_BUILD_DATE")
        .unwrap_or("unknown")
        .to_string()
}

/// 获取 Rust 版本
fn get_rust_version() -> String {
    option_env!("VERGEN_RUSTC_SEMVER")
        .unwrap_or("unknown")
        .to_string()
}
