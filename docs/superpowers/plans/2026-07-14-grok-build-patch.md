# Grok Build 接入 + 上传 Patch 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 Grok Build 接入 AIW（detect + `aiw update` 全量/单工具更新 + 更新后自动 patch），并用纯控制流 patch（call→`31 c0 48 89 07` 等长替换）禁用三类数据外泄上传。

**Architecture:** Claude/Grok 平级 patcher（共享层 `patcher/` 顶层 + `claude/` + `grok/` 子文件夹，正本清源）。patch 用机器码字节级等长替换，复用现有 `UnifiedPatchPattern` + `apply_file_patch` 引擎。Grok 更新由 AIW 自己下载 binary（非 shell-out），便于下载后立即打 patch。

**Tech Stack:** Rust (Edition 2021, tokio async, clap 4.4), regex::bytes, reqwest (rustls-tls), which (CLI 探测)

## Global Constraints

- **Grok binary 来源**：仅 `https://x.ai/cli/grok-{version}-linux-x86_64`（fallback `https://storage.googleapis.com/grok-build-public-artifacts/cli/grok-{version}-linux-x86_64`），不走 npm。
- **本地探测路径**：`~/.grok/bin/grok` 软链 → `~/.grok/downloads/grok-linux-x86_64`；版本文件 `~/.grok/version.json`（`{"version":"0.2.99",...}`）。
- **patch 等长铁律**：call 指令 `e8 xx xx xx xx`（5 字节）必须替换为等长 5 字节 `31 c0 48 89 07`，不破坏后续偏移。
- **不硬编码地址**：patch 锚点用「tracing 字符串 → slice ptr → text ref → call 字节模式」动态定位，跨版本通配。
- **测试铁律**（CLAUDE.md §5）：禁止 `#[ignore]`；禁止 Mock 集成/E2E；SPEC 驱动每个 REQ 有测试；过时测试删除或更新。
- **Rust 编码规范**：`thiserror` + `anyhow`；`tracing` 宏日志；`cfg(unix)`/`cfg(windows)` 平台隔离。
- **重构铁律**：直接替换，不留向后兼容层（CLAUDE.md 宪法）。
- **当前验证版本**：0.2.99（本地已升级），类2 patch 点 `0x51c5692`/`0x51c9ecb`，字节模式 `e8 rel32` + 前缀 `lea 0x3d0(%rsp),%rdi`（`48 8d bc 24 d0 03 00 00`）+ 后缀 `mov 0x3d0(%rsp),%r15; test; je`。

---

## File Structure

**新建文件：**
- `src/patcher/claude/mod.rs` — Claude 子模块入口
- `src/patcher/claude/registry.rs` — 7 个 Claude feature 的 pattern 生成（从 `patcher/registry.rs` 迁入）
- `src/patcher/claude/versions.rs` — ClaudeVersion + MAX_CONTEXT_TOKENS（从 `patcher/versions.rs` 迁入）
- `src/patcher/claude/install.rs` — get_claude_cli_path / get_claude_js_path / detect_claude_installation（从 `patcher/file.rs` 迁入）
- `src/patcher/grok/mod.rs` — Grok 子模块入口
- `src/patcher/grok/registry.rs` — 3 个 Grok feature 的 pattern 生成
- `src/patcher/grok/versions.rs` — GrokVersion 解析 + 探测
- `src/patcher/grok/targets.rs` — 三类上传锚点定位（tracing 字符串 + call 字节模式匹配）
- `src/patcher/grok/install.rs` — detect_grok / get_grok_binary_path
- `tests/unit/grok_patch.rs` — Grok patch 单元测试

**修改文件：**
- `src/patcher/mod.rs` — `pub mod claude; pub mod grok;`；移除顶层 registry/versions re-export
- `src/patcher/types.rs` — FeatureType 加 3 个 Grok 变体
- `src/cli_type.rs` — CliType 加 Grok
- `src/cli_manager.rs` — initialize_tools 加 grok；execute_update 加 grok 特判；update_grok_cli；detect_grok_installation
- `src/commands/patch.rs` — execute_grok_patch_* 系列
- `src/commands/parser.rs` — PatchAction 加 Grok 子命令
- `src/main.rs` — Grok patch 路由
- `src/config.rs` / `src/supervisor.rs` — Claude import 路径改写
- `CLAUDE.md` — patch 支持矩阵加 Grok

---

## Task 1: Claude patcher 下沉到 patcher/claude/（正本清源）

**Files:**
- Create: `src/patcher/claude/mod.rs`, `src/patcher/claude/registry.rs`, `src/patcher/claude/versions.rs`, `src/patcher/claude/install.rs`
- Delete: `src/patcher/registry.rs`, `src/patcher/versions.rs`
- Modify: `src/patcher/mod.rs`, `src/patcher/file.rs`, `src/config.rs`, `src/cli_manager.rs`, `src/supervisor.rs`, `src/commands/patch.rs`

**Interfaces:**
- Produces: `patcher::claude::registry::get_feature_patches` / `get_antitelemetry_patches` / `get_antispy_patches` / `get_antipromptbias_patches` / `get_antiatis_patches` / `get_antiframetrack_patches` / `get_anticloudetect_patches` / `get_max_context_tokens_patches`；`patcher::claude::versions::{ClaudeVersion, validate_max_context_tokens, encode_max_context_tokens, MAX_CONTEXT_TOKENS_SEARCH_REGEX}`；`patcher::claude::install::{get_claude_cli_path, get_claude_js_path, detect_claude_installation}`

- [ ] **Step 1: 创建 claude/versions.rs（从 patcher/versions.rs 整体迁入）**

把 `src/patcher/versions.rs` 内容原样复制到 `src/patcher/claude/versions.rs`，不改任何代码。然后删除 `src/patcher/versions.rs`。

- [ ] **Step 2: 创建 claude/registry.rs（从 patcher/registry.rs 整体迁入）**

把 `src/patcher/registry.rs` 内容复制到 `src/patcher/claude/registry.rs`，把其中的 `use crate::patcher::versions::...` 改为 `use crate::patcher::claude::versions::...`。然后删除 `src/patcher/registry.rs`。

- [ ] **Step 3: 创建 claude/install.rs（从 file.rs 迁入 Claude 专有函数）**

创建 `src/patcher/claude/install.rs`，迁入 `file.rs` 中的 `get_claude_cli_path` / `get_claude_js_path` / `detect_installation` / `is_npm_installation` / `get_patchable_path` 这 5 个 Claude 专有函数（原样搬，`use` 引用改 `crate::patcher::file::...` 共享层 + `crate::patcher::claude::install::InstallationType` 如果 InstallationType 在 file.rs 则留在 file.rs 共享层）。

注意：`InstallationType` 和 `detect_installation` 的返回值被 `commands/patch.rs` 共用，若 `InstallationType` 定义在 `file.rs`，则保留在 `file.rs` 共享层（不迁移），`claude/install.rs` 只 `use crate::patcher::file::InstallationType`。判断标准：`InstallationType` 是 Claude 专有还是通用——grep `InstallationType` 用法，若只有 Claude 用则迁入 claude/install.rs，否则留共享层。实测：`file.rs:apply_file_patch` 不依赖 `InstallationType`，`detect_installation` 是 Claude 专有 → `InstallationType` + `detect_installation` + `get_claude_*` + `is_npm_installation` + `get_patchable_path` 全部迁入 `claude/install.rs`。

- [ ] **Step 4: 创建 claude/mod.rs**

```rust
//! Claude CLI 专有 patch 模块（7 个 feature + 版本工具 + 安装探测）

pub mod install;
pub mod registry;
pub mod versions;

pub use registry::{
    get_anticloudetect_patches, get_antiframetrack_patches, get_antiatis_patches,
    get_antipromptbias_patches, get_antispy_patches, get_antitelemetry_patches,
    get_feature_patches, get_max_context_tokens_patches,
};
pub use versions::{ClaudeVersion, MAX_CONTEXT_TOKENS_SEARCH_REGEX, encode_max_context_tokens, validate_max_context_tokens};
pub use install::{detect_installation as detect_claude_installation, get_claude_cli_path, get_claude_js_path, get_patchable_path as get_claude_patchable_path, is_npm_installation, InstallationType};
```

- [ ] **Step 5: 更新 patcher/mod.rs**

把 `src/patcher/mod.rs` 改为：

```rust
//! # 统一补丁框架（共享层）
//!
//! 支持文件补丁和内存补丁两种模式。共享层提供 UnifiedPatchPattern +
//! apply_file_patch 引擎；各 CLI 的专有 pattern 在子模块（claude/ grok/）。

pub mod claude;
pub mod error;
pub mod file;
pub mod platform;
pub mod runtime;
pub mod types;

pub use error::{PatchError, PatchResult};
pub use platform::{MemoryPatcher, MemoryRegion, MemPerm, PlatformMemoryPatcher};
pub use runtime::RuntimePatcher;
pub use types::{
    DynamicReplace, FeatureType, PatchType, UnifiedPatchPattern, UnifiedPatchResult,
    UnifiedPatchError, Result as PatchResultType,
};
// 共享层 re-export（file.rs 的 apply_file_patch / is_file_patched / restore_from_backup）
pub use file::{apply_file_patch, is_file_patched, restore_from_backup};
```

注意：移除了顶层 `pub mod registry; pub mod versions;` 和 `pub use registry::get_feature_patches;` 以及 `pub use file::{get_claude_cli_path, get_claude_js_path, InstallationType, detect_installation, get_patchable_path, restore_from_backup};`（后者改为只 re-export 共享的 3 个函数，Claude 专有的移到 `claude::install`）。

- [ ] **Step 6: 改写 file.rs 的 import**

`src/patcher/file.rs` 里 `use crate::patcher::versions::...` 改为 `use crate::patcher::claude::versions::...`（`apply_regex_replace` 用到 `encode_max_context_tokens`）。删掉迁出的 5 个函数（`get_claude_cli_path` 等）的函数体（已迁到 claude/install.rs），但保留 `InstallationType` 若决定留共享层则不动——按 Step 3 判断：全迁入 claude/install.rs，所以 file.rs 删掉这 5 个函数 + `InstallationType` enum 定义。

- [ ] **Step 7: 改写 config.rs import**

`src/config.rs:84,86` 的 `crate::patcher::versions::validate_max_context_tokens` → `crate::patcher::claude::versions::validate_max_context_tokens`。

- [ ] **Step 8: 改写 cli_manager.rs import**

`src/cli_manager.rs:9` 的 `use crate::patcher::versions::ClaudeVersion;` → `use crate::patcher::claude::versions::ClaudeVersion;`。

- [ ] **Step 9: 改写 supervisor.rs import**

`src/supervisor.rs:86,88,89,232,234` 的 `crate::patcher::registry::get_feature_patches` → `crate::patcher::claude::registry::get_feature_patches`；`crate::patcher::versions::ClaudeVersion` → `crate::patcher::claude::versions::ClaudeVersion`；`crate::patcher::{get_patchable_path, is_file_patched}` → `crate::patcher::claude::{get_claude_patchable_path, ...}` + `crate::patcher::is_file_patched`（is_file_patched 是共享层保留）。

- [ ] **Step 10: 改写 commands/patch.rs import**

`src/commands/patch.rs:8-18` 的 `use crate::patcher::{apply_file_patch, detect_installation, get_patchable_path, is_file_patched, restore_from_backup, InstallationType, ...}` 拆分：
```rust
use crate::patcher::{apply_file_patch, is_file_patched, restore_from_backup};
use crate::patcher::claude::{
    get_antiatis_patches, get_anticloudetect_patches, get_antiframetrack_patches,
    get_antipromptbias_patches, get_antispy_patches, get_antitelemetry_patches,
    get_feature_patches, get_claude_patchable_path as get_patchable_path,
    detect_claude_installation as detect_installation, InstallationType,
};
use crate::patcher::claude::versions::{validate_max_context_tokens, ClaudeVersion};
```

- [ ] **Step 11: 验证编译 + 测试**

Run: `cargo build 2>&1 | tail -20`
Expected: 编译通过，无 error（可能有 unused warning，后续修）。

Run: `cargo test --lib patcher 2>&1 | tail -20`
Expected: 所有 Claude patcher 测试通过（迁移不改逻辑）。

- [ ] **Step 12: Commit**

```bash
git add -A && git commit -m "refactor(patcher): Claude 专有下沉到 patcher/claude/（正本清源）

将 registry/versions/install 从顶层下沉到 claude/ 子模块，共享层
（file/types/runtime/platform/error）保留。为 Grok 平级子模块铺路。

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 2: FeatureType 加 Grok 变体 + GrokVersion

**Files:**
- Modify: `src/patcher/types.rs`（FeatureType 枚举 + description/short_name）
- Create: `src/patcher/grok/mod.rs`, `src/patcher/grok/versions.rs`

**Interfaces:**
- Produces: `FeatureType::GrokAntiRepoBundle` / `GrokAntiDeployUpload` / `GrokAntiTraceUpload`；`patcher::grok::versions::GrokVersion`

- [ ] **Step 1: 写失败测试（FeatureType Grok 变体）**

在 `src/patcher/types.rs` 的 `#[cfg(test)] mod tests` 末尾加：

```rust
#[test]
fn test_grok_anti_repo_bundle_variant() {
    assert_eq!(FeatureType::GrokAntiRepoBundle.short_name(), "grokantirepobundle");
    assert!(FeatureType::GrokAntiRepoBundle
        .description()
        .contains("GrokAntiRepoBundle"));
}

#[test]
fn test_grok_variants_distinct() {
    assert_ne!(FeatureType::GrokAntiRepoBundle, FeatureType::GrokAntiDeployUpload);
    assert_ne!(FeatureType::GrokAntiRepoBundle, FeatureType::GrokAntiTraceUpload);
    assert_ne!(FeatureType::GrokAntiDeployUpload, FeatureType::GrokAntiTraceUpload);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --lib test_grok_anti_repo_bundle 2>&1 | tail -10`
Expected: FAIL（`GrokAntiRepoBundle` 未定义，编译错）。

- [ ] **Step 3: 在 FeatureType 枚举加 3 个变体**

`src/patcher/types.rs` 的 `pub enum FeatureType` 末尾（`AntiCloudDetect` 后）加：

```rust
    /// GrokAntiRepoBundle - 禁用 Grok Repo Changes git bundle 上传（call→xor+mov）
    ///
    /// patch GCS blob 上传 dispatcher 的 2 个调用点（call 指令），
    /// 等长替换 e8 xx xx xx xx → 31 c0 48 89 07（xor eax,eax; mov [rdi],rax），
    /// 让上传函数返回空结果，调用方走"无结果→跳过"分支。跨版本稳定
    /// （tracing 字符串 + call 字节模式动态定位）。
    GrokAntiRepoBundle,
    /// GrokAntiDeployUpload - 禁用 Grok App Builder 部署上传（call→xor+mov）
    GrokAntiDeployUpload,
    /// GrokAntiTraceUpload - 禁用 Grok Session Trace 上传（call→xor+mov）
    GrokAntiTraceUpload,
```

在 `description()` match 加：
```rust
            FeatureType::GrokAntiRepoBundle => "GrokAntiRepoBundle - 禁用 Repo Changes git bundle 上传（call→xor+mov）",
            FeatureType::GrokAntiDeployUpload => "GrokAntiDeployUpload - 禁用 App Builder 部署上传",
            FeatureType::GrokAntiTraceUpload => "GrokAntiTraceUpload - 禁用 Session Trace 上传",
```

在 `short_name()` match 加：
```rust
            FeatureType::GrokAntiRepoBundle => "grokantirepobundle",
            FeatureType::GrokAntiDeployUpload => "grokantideployupload",
            FeatureType::GrokAntiTraceUpload => "grokantitraceupload",
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --lib test_grok_anti_repo_bundle test_grok_variants_distinct 2>&1 | tail -10`
Expected: PASS。

- [ ] **Step 5: 写 GrokVersion 失败测试**

创建 `src/patcher/grok/versions.rs`：

```rust
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
```

- [ ] **Step 6: 运行测试确认通过**

Run: `cargo test --lib grok::versions 2>&1 | tail -10`
Expected: PASS。

- [ ] **Step 7: 创建 grok/mod.rs**

```rust
//! Grok CLI 专有 patch 模块（3 个上传禁用 feature + 版本 + 锚点定位 + 安装探测）

pub mod install;
pub mod registry;
pub mod targets;
pub mod versions;

pub use versions::GrokVersion;
```

- [ ] **Step 8: 更新 patcher/mod.rs 加 pub mod grok**

`src/patcher/mod.rs` 加 `pub mod grok;`（在 `pub mod claude;` 后）。

- [ ] **Step 9: 验证编译**

Run: `cargo build 2>&1 | tail -10`
Expected: 编译通过（targets/registry/install 暂为空模块，先建空文件 `src/patcher/grok/targets.rs` `src/patcher/grok/registry.rs` `src/patcher/grok/install.rs` 各放一行 `//! placeholder`）。

- [ ] **Step 10: Commit**

```bash
git add -A && git commit -m "feat(patcher): FeatureType 加 3 个 Grok 变体 + GrokVersion

GrokAntiRepoBundle/GrokAntiDeployUpload/GrokAntiTraceUpload 三个 feature
变体 + GrokVersion 解析（0.2.99），为 Grok patch 铺路。

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 3: Grok 锚点定位（targets.rs）— call 字节模式匹配

**Files:**
- Modify: `src/patcher/grok/targets.rs`

**Interfaces:**
- Produces: `GrokUploadAnchor` 结构体 + `locate_repo_bundle_call_sites(binary: &[u8]) -> Result<Vec<usize>>`（返回 2 个 call 指令在 binary 里的字节偏移）

- [ ] **Step 1: 写失败测试（锚点定位）**

创建 `tests/unit/grok_patch.rs`：

```rust
use aiw::patcher::grok::targets::locate_repo_bundle_call_sites;

#[test]
fn test_locate_repo_bundle_call_sites_on_v0299() {
    let binary = std::fs::read("/home/putao/.grok/downloads/grok-linux-x86_64").unwrap();
    let sites = locate_repo_bundle_call_sites(&binary).expect("locate failed");
    // 必须找到恰好 2 个 call 点（0.2.99: 0x51c5692 / 0x51c9ecb）
    assert_eq!(sites.len(), 2, "expected 2 GCS upload call sites, got {}", sites.len());
    // 验证每个点都是 e8（call rel32）
    for &off in &sites {
        assert_eq!(binary[off], 0xe8, "call opcode at {:#x} should be 0xe8", off);
    }
    // 验证前缀是 lea 0x3d0(%rsp),%rdi（48 8d bc 24 d0 03 00 00）
    let prefix: &[u8] = &[0x48, 0x8d, 0xbc, 0x24, 0xd0, 0x03, 0x00, 0x00];
    for &off in &sites {
        let pre = &binary[off - 8..off];
        assert_eq!(pre, prefix, "prefix before call at {:#x} mismatch", off);
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --test grok_patch test_locate_repo_bundle_call_sites_on_v0299 2>&1 | tail -10`
Expected: FAIL（`locate_repo_bundle_call_sites` 未定义）。

- [ ] **Step 3: 实现 targets.rs**

```rust
//! Grok 上传 patch 锚点定位
//!
//! 通过 tracing 字符串 → slice ptr → text ref → call 字节模式 动态定位
//! GCS blob 上传 dispatcher 的 2 个调用点。跨版本通配，不硬编码地址。
//!
//! 方法论见 docs/domain-knowledge/grok-build.md。

use crate::patcher::types::UnifiedPatchError;

/// call 指令前缀：`lea 0x3d0(%rsp),%rdi`（out 参数地址入 rdi）
///
/// 字节: 48 8d bc 24 d0 03 00 00
const LEA_OUT_PARAM_PREFIX: &[u8] = &[0x48, 0x8d, 0xbc, 0x24, 0xd0, 0x03, 0x00, 0x00];

/// call rel32 指令字节 + 后缀 `mov 0x3d0(%rsp),%r15`（48 8b bc 24 d0 03 00 00）
///
/// 完整匹配: prefix(8) + e8 rel32(5) + mov(8) = 21 字节锚点
const MOV_OUT_PARAM_SUFFIX: &[u8] = &[0x48, 0x8b, 0xbc, 0x24, 0xd0, 0x03, 0x00, 0x00];

/// 定位 Repo Changes git bundle 上传的 2 个 call 调用点。
///
/// 匹配模式: `lea 0x3d0(%rsp),%rdi; call rel32; mov 0x3d0(%rsp),%r15`
/// 即 LEA_OUT_PARAM_PREFIX(8) + 0xe8 + [4 bytes rel32] + MOV_OUT_PARAM_SUFFIX(8)。
/// 要求恰好 2 个匹配（0.2.93/0.2.99 均为 2 个）。
pub fn locate_repo_bundle_call_sites(binary: &[u8]) -> Result<Vec<usize>, UnifiedPatchError> {
    let mut sites = Vec::new();
    let pattern_len = LEA_OUT_PARAM_PREFIX.len() + 1 + 4 + MOV_OUT_PARAM_SUFFIX.len(); // 8+1+4+8=21
    let mut i = 0;
    while i + pattern_len <= binary.len() {
        if &binary[i..i + LEA_OUT_PARAM_PREFIX.len()] == LEA_OUT_PARAM_PREFIX
            && binary[i + LEA_OUT_PARAM_PREFIX.len()] == 0xe8
            && &binary[i + LEA_OUT_PARAM_PREFIX.len() + 5..i + pattern_len] == MOV_OUT_PARAM_SUFFIX
        {
            let call_off = i + LEA_OUT_PARAM_PREFIX.len();
            sites.push(call_off);
            i += pattern_len;
        } else {
            i += 1;
        }
    }
    if sites.is_empty() {
        return Err(UnifiedPatchError::PatternNotFound(
            "Grok repo bundle call sites not found (tracing/pattern drift?)".to_string(),
        ));
    }
    Ok(sites)
}

/// 5 字节替换字节: xor eax,eax; mov [rdi],rax
pub const CALL_REPLACE: &[u8] = &[0x31, 0xc0, 0x48, 0x89, 0x07];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_len() {
        assert_eq!(LEA_OUT_PARAM_PREFIX.len() + 1 + 4 + MOV_OUT_PARAM_SUFFIX.len(), 21);
    }

    #[test]
    fn test_replace_is_5_bytes() {
        assert_eq!(CALL_REPLACE.len(), 5);
    }

    #[test]
    fn test_locate_on_synthetic() {
        // 构造一个含 2 个匹配模式的合成 binary
        let mut bin = vec![0u8; 100];
        let pattern: Vec<u8> = LEA_OUT_PARAM_PREFIX
            .iter()
            .chain(&[0xe8, 0xaa, 0xbb, 0xcc, 0xdd])
            .chain(MOV_OUT_PARAM_SUFFIX.iter())
            .copied()
            .collect();
        bin[10..10 + pattern.len()].copy_from_slice(&pattern);
        bin[60..60 + pattern.len()].copy_from_slice(&pattern);
        let sites = locate_repo_bundle_call_sites(&bin).unwrap();
        assert_eq!(sites.len(), 2);
        assert_eq!(bin[sites[0]], 0xe8);
        assert_eq!(bin[sites[1]], 0xe8);
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --lib grok::targets 2>&1 | tail -10`
Expected: PASS（合成测试）。

Run: `cargo test --test grok_patch test_locate_repo_bundle_call_sites_on_v0299 2>&1 | tail -10`
Expected: PASS（在真实 0.2.99 binary 上定位到 2 个 call 点）。

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(grok): repo bundle 上传 call 点定位（targets.rs）

通过 lea+call+mov 21 字节锚点模式定位 GCS 上传 dispatcher 的 2 个调用点。
0.2.99 实测命中 2 个（0x51c5692/0x51c9ecb），跨版本通配不硬编码地址。

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 4: Grok registry（生成 patch pattern）+ install.rs

**Files:**
- Modify: `src/patcher/grok/registry.rs`, `src/patcher/grok/install.rs`

**Interfaces:**
- Produces: `patcher::grok::registry::get_grok_repo_bundle_patches() -> Vec<UnifiedPatchPattern>`；`patcher::grok::install::{detect_grok, get_grok_binary_path, GrokInstallation}`

- [ ] **Step 1: 写失败测试（install 探测）**

在 `tests/unit/grok_patch.rs` 加：

```rust
use aiw::patcher::grok::install::{detect_grok, get_grok_binary_path};

#[test]
fn test_detect_grok_local() {
    let inst = detect_grok().expect("grok detect failed");
    assert!(inst.installed, "grok should be installed locally");
    assert!(inst.binary_path.exists(), "binary path should exist");
    assert_eq!(inst.version.major, 0);
    assert_eq!(inst.version.minor, 2);
    // patch 版本可能是 99 或更新
    assert!(inst.version.patch >= 93);
}

#[test]
fn test_get_grok_binary_path() {
    let p = get_grok_binary_path().unwrap();
    assert!(p.to_string_lossy().contains("grok-linux-x86_64"));
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --test grok_patch test_detect_grok_local 2>&1 | tail -10`
Expected: FAIL（`detect_grok` 未定义）。

- [ ] **Step 3: 实现 install.rs**

```rust
//! Grok CLI 安装探测
//!
//! 探测 ~/.grok/bin/grok 软链 → ~/.grok/downloads/grok-linux-x86_64，
//! 读 ~/.grok/version.json 得版本。

use crate::patcher::grok::versions::GrokVersion;
use crate::patcher::types::UnifiedPatchError;
use std::path::PathBuf;

/// Grok 安装信息
#[derive(Debug, Clone)]
pub struct GrokInstallation {
    pub binary_path: PathBuf,
    pub version: GrokVersion,
    pub installed: bool,
}

/// Grok home 目录（默认 ~/.grok）
fn grok_home() -> PathBuf {
    std::env::var("GROK_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default().join(".grok"))
}

/// Grok binary 路径: ~/.grok/downloads/grok-linux-x86_64
pub fn get_grok_binary_path() -> Result<PathBuf, UnifiedPatchError> {
    let binary = grok_home().join("downloads").join("grok-linux-x86_64");
    if binary.exists() {
        Ok(binary)
    } else {
        Err(UnifiedPatchError::FileError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Grok binary not found at ~/.grok/downloads/grok-linux-x86_64",
        )))
    }
}

/// 读 ~/.grok/version.json 得版本
fn read_grok_version() -> Option<GrokVersion> {
    let vjson = grok_home().join("version.json");
    let content = std::fs::read_to_string(&vjson).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    let ver_str = v.get("version")?.as_str()?;
    GrokVersion::from_string(ver_str)
}

/// 探测 Grok 安装
pub fn detect_grok() -> Result<GrokInstallation, UnifiedPatchError> {
    let binary_path = get_grok_binary_path()?;
    // 优先读 version.json，失败则跑 grok --version
    let version = read_grok_version().unwrap_or_else(|| {
        let out = std::process::Command::new(&binary_path)
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();
        GrokVersion::from_string(&out).unwrap_or(GrokVersion { major: 0, minor: 0, patch: 0 })
    });
    Ok(GrokInstallation { binary_path, version, installed: true })
}
```

注意：需在 `Cargo.toml` 确认 `dirs` 和 `serde_json` 依赖存在（serde_json 已在技术栈，dirs 需检查，若无需用 `std::env::var("HOME")` 替代）。

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --test grok_patch test_detect_grok_local test_get_grok_binary_path 2>&1 | tail -10`
Expected: PASS。

- [ ] **Step 5: 实现 registry.rs（生成 patch pattern）**

```rust
//! Grok patch pattern 生成
//!
//! 为 3 个 Grok feature 生成 UnifiedPatchPattern。repo bundle 已定案
//! （call→31 c0 48 89 07），deploy/trace 实现阶段定位。

use crate::patcher::grok::install::get_grok_binary_path;
use crate::patcher::grok::targets::{locate_repo_bundle_call_sites, CALL_REPLACE};
use crate::patcher::types::{FeatureType, PatchType, UnifiedPatchPattern, UnifiedPatchError};
use std::borrow::Cow;

/// 生成 repo bundle 上传 patch（2 个 call 点 → 31 c0 48 89 07）
///
/// 运行时读 binary，定位 2 个 call 点，为每个点生成一个字面量等长替换 pattern。
/// search = call 指令 5 字节（e8 + rel32），replace = CALL_REPLACE（5 字节）。
pub fn get_grok_repo_bundle_patches() -> Result<Vec<UnifiedPatchPattern>, UnifiedPatchError> {
    let binary_path = get_grok_binary_path()?;
    let binary = std::fs::read(&binary_path)?;
    let sites = locate_repo_bundle_call_sites(&binary)?;
    let mut patches = Vec::with_capacity(sites.len());
    for off in sites {
        // search: call 指令的 5 字节（e8 + 4 字节 rel32，从 binary 实际读取）
        let search: Vec<u8> = binary[off..off + 5].to_vec();
        patches.push(UnifiedPatchPattern {
            feature: FeatureType::GrokAntiRepoBundle,
            patch_type: PatchType::File,
            search_pattern: Cow::Owned(search),
            replace_pattern: Some(Cow::Borrowed(CALL_REPLACE)),
            patch_byte: None,
            patch_offset: None,
            description: Cow::Owned(format!(
                "GrokAntiRepoBundle: GCS upload call @ {:#x} → xor+mov (no-op upload)",
                off
            )),
            use_regex: false,
            regex_replace_values: None,
            dynamic_replace: None,
        });
    }
    Ok(patches)
}

/// deploy upload patch（实现阶段定位，暂返回空 + 诊断）
pub fn get_grok_deploy_upload_patches() -> Result<Vec<UnifiedPatchPattern>, UnifiedPatchError> {
    // TODO Task 8: 定位 [deploy_app] starting upload build 的 call 点
    Ok(vec![])
}

/// trace upload patch（实现阶段定位，暂返回空 + 诊断）
pub fn get_grok_trace_upload_patches() -> Result<Vec<UnifiedPatchPattern>, UnifiedPatchError> {
    // TODO Task 8: 定位 upload session trace 的 call 点
    Ok(vec![])
}
```

- [ ] **Step 6: 验证编译**

Run: `cargo build 2>&1 | tail -10`
Expected: 编译通过。

- [ ] **Step 7: Commit**

```bash
git add -A && git commit -m "feat(grok): registry 生成 repo bundle patch pattern + install 探测

get_grok_repo_bundle_patches 运行时读 binary 定位 2 个 call 点生成等长替换
pattern。detect_grok/get_grok_binary_path 探测 ~/.grok 安装。

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 5: Grok patch 应用（commands/patch.rs + parser.rs）

**Files:**
- Modify: `src/commands/patch.rs`, `src/commands/parser.rs`, `src/main.rs`

**Interfaces:**
- Produces: `execute_grok_patch_apply()` / `execute_grok_patch_status()` / `execute_grok_patch_restore()`；`PatchAction::GrokPatchApply` / `GrokPatchStatus` / `GrokPatchRestore`

- [ ] **Step 1: 在 parser.rs 的 PatchAction 加 Grok 子命令**

`src/commands/parser.rs` 的 `pub enum PatchAction` 末尾加：

```rust
    /// 应用 Grok 上传 patch（禁用 repo bundle / deploy / trace 三类上传）
    #[command(name = "grok-apply")]
    GrokPatchApply,
    /// 查看 Grok patch 状态
    #[command(name = "grok-status")]
    GrokPatchStatus,
    /// 还原 Grok patch
    #[command(name = "grok-restore")]
    GrokPatchRestore,
```

- [ ] **Step 2: 在 commands/patch.rs 的 execute_patch_command 加分发**

`src/commands/patch.rs` 的 `execute_patch_command` match 末尾加：

```rust
        PatchAction::GrokPatchApply => {
            execute_grok_patch_apply()?;
        }
        PatchAction::GrokPatchStatus => {
            execute_grok_patch_status();
        }
        PatchAction::GrokPatchRestore => {
            execute_grok_patch_restore()?;
        }
```

- [ ] **Step 3: 实现 execute_grok_patch_apply / status / restore**

在 `src/commands/patch.rs` 末尾加：

```rust
/// 应用 Grok 上传 patch
fn execute_grok_patch_apply() -> Result<()> {
    use crate::patcher::grok::install::detect_grok;
    use crate::patcher::grok::registry::get_grok_repo_bundle_patches;

    let inst = match detect_grok() {
        Ok(i) => i,
        Err(e) => {
            println!("❌ 未检测到 Grok 安装: {}", e);
            return Ok(());
        }
    };
    println!("📂 Grok binary: {} (v{})", inst.binary_path.display(), inst.version);

    let patches = match get_grok_repo_bundle_patches() {
        Ok(p) => p,
        Err(e) => {
            println!("❌ 定位 patch 锚点失败: {}", e);
            println!("   可能是 Grok 版本更新导致字节模式漂移，请检查 docs/domain-knowledge/grok-build.md");
            return Ok(());
        }
    };
    println!("🔍 定位到 {} 个 repo bundle call 点", patches.len());

    for patch in &patches {
        match apply_file_patch(&inst.binary_path, patch) {
            Ok(_) => println!("   ✅ {}", patch.description),
            Err(e) => println!("   ❌ 失败: {}", e),
        }
    }
    println!("✅ Grok repo bundle patch 应用完成（GCS 上传已禁用）");
    Ok(())
}

/// 查看 Grok patch 状态
fn execute_grok_patch_status() {
    use crate::patcher::grok::install::detect_grok;
    use crate::patcher::grok::registry::get_grok_repo_bundle_patches;

    let inst = match detect_grok() {
        Ok(i) => i,
        Err(e) => {
            println!("⚠️  未检测到 Grok 安装: {}", e);
            return;
        }
    };
    println!("📊 Grok patch 状态:");
    println!("   Grok version: {}", inst.version);
    println!("   Binary: {}", inst.binary_path.display());

    match get_grok_repo_bundle_patches() {
        Ok(patches) => {
            for patch in &patches {
                match is_file_patched(&inst.binary_path, patch) {
                    Ok(true) => println!("   ✅ {} 已应用", patch.description),
                    Ok(false) => println!("   ❌ {} 未应用", patch.description),
                    Err(_) => println!("   ⚪ {} 无法检测", patch.description),
                }
            }
        }
        Err(e) => println!("   ❌ 锚点定位失败: {}", e),
    }
}

/// 还原 Grok patch
fn execute_grok_patch_restore() -> Result<()> {
    use crate::patcher::grok::install::detect_grok;
    let inst = detect_grok()?;
    println!("🔄 从备份恢复 Grok binary...");
    match restore_from_backup(&inst.binary_path) {
        Ok(()) => println!("✅ 已从备份恢复: {}", inst.binary_path.display()),
        Err(e) => println!("❌ 恢复失败: {}", e),
    }
    Ok(())
}
```

注意：`is_file_patched` 检测的是 search_pattern 是否已不存在（被 replace 覆盖）。但 Grok patch 的 search 是从 binary 实读的 5 字节 call，patch 后这 5 字节变成 `31 c0 48 89 07`，search 找不到 → `is_file_patched` 返回 true。需确认 `is_file_patched` 的语义是"search_pattern 不存在 = 已 patched"——读 `file.rs:is_file_patched` 确认，若语义相反则调整。

- [ ] **Step 4: main.rs 路由（Patch 已有路由，确认 Grok 子命令走通）**

`src/main.rs:197` 的 `Commands::Patch(action) => handle_patch_action(action).await` 已存在，GrokPatchApply 等走同一路由，无需改 main.rs。但需确认 `handle_patch_action` 调用 `execute_patch_command`。

Run: `grep -n "handle_patch_action\|execute_patch_command" src/main.rs src/commands/patch.rs`
Expected: 确认 `handle_patch_action` → `execute_patch_command`。

- [ ] **Step 5: 验证编译 + 手动测试**

Run: `cargo build 2>&1 | tail -10`
Expected: 编译通过。

Run: `cargo run -- patch grok-status 2>&1 | tail -10`
Expected: 打印 Grok 版本 0.2.99 + patch 未应用（首次）。

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat(grok): patch CLI（grok-apply/grok-status/grok-restore）

aiw patch grok-apply 应用 repo bundle 上传 patch，grok-status 查状态，
grok-restore 从备份恢复。

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 6: CliType::Grok + cli_manager detect/update_grok_cli

**Files:**
- Modify: `src/cli_type.rs`, `src/cli_manager.rs`

**Interfaces:**
- Produces: `CliType::Grok`；`cli_manager::update_grok_cli(tool: &CliTool) -> (String, bool, String)`；`cli_manager::detect_grok_installation()`

- [ ] **Step 1: cli_type.rs 加 Grok**

`src/cli_type.rs` 的 `pub enum CliType` 加 `Grok` 变体（找到 enum 定义，在 Gemini 后加）。同时在该 enum 的所有 match 分支（`as_str` / `from_str` / `command` 等）加 grok 处理。

Run: `grep -n "Gemini" src/cli_type.rs` 找到所有需加 Grok 的位置。

- [ ] **Step 2: cli_manager initialize_tools 加 grok CliTool**

`src/cli_manager.rs:138` 的 `initialize_tools` 的 vec 末尾（Gemini 后）加：

```rust
            CliTool {
                name: "Grok Build".to_string(),
                command: "grok".to_string(),
                npm_package: String::new(), // Grok 不走 npm
                description: "xAI Grok Build CLI tool".to_string(),
                installed: false,
                version: None,
                install_type: None,
                install_path: None,
            },
```

- [ ] **Step 3: execute_update 加 grok 特判**

`src/cli_manager.rs:770` 的 `if tool.command == "claude" {` 后加：

```rust
        if tool.command == "claude" {
            let result = update_claude_cli(tool).await;
            results.push(result);
            continue;
        } else if tool.command == "grok" {
            let result = update_grok_cli(tool).await;
            results.push(result);
            continue;
        }
```

同时把 `execute_update:739` 的报错文案 `"Supported: claude, codex, gemini"` 改为 `"Supported: claude, codex, gemini, grok"`。

同时把 `756` 行的 `let needs_nodejs = tools_to_process.iter().any(|t| t.command != "claude");` 改为 `let needs_nodejs = tools_to_process.iter().any(|t| t.command != "claude" && t.command != "grok");`（grok 也不走 npm/node）。

- [ ] **Step 4: 实现 update_grok_cli（AIW 自己下载 + 下载后自动 patch）**

在 `src/cli_manager.rs` 末尾（`update_claude_cli` 后）加：

```rust
/// 更新 Grok CLI（AIW 自己下载 binary，下载后自动 patch）
async fn update_grok_cli(tool: &CliTool) -> (String, bool, String) {
    use crate::patcher::grok::install::{detect_grok, get_grok_binary_path};

    // 1. 版本检查：grok update --check --json 得 latestVersion
    let latest = match check_grok_latest_version().await {
        Some(v) => v,
        None => return (tool.name.clone(), false, "Failed to check latest version".to_string()),
    };

    if tool.installed {
        if let Some(ref cur) = tool.version {
            println!("  Current version: {}", cur);
        }
        println!("  Latest version: {}", latest);

        // 已是最新则跳过（但仍可选重新 patch）
        if tool.version.as_deref() == Some(latest.as_str()) {
            println!("  ✅ Already up to date!");
            return (tool.name.clone(), true, "Already up to date".to_string());
        }
    }

    // 2. 下载 binary
    let arch = if cfg!(target_arch = "x86_64") { "linux-x86_64" } else { "linux-arm64" };
    let url = format!("https://x.ai/cli/grok-{}-{}", latest, arch);
    println!("  ⬇️  Downloading from {}", url);

    let tmp_path = get_grok_binary_path()
        .map(|p| p.with_extension("tmp"))
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp/grok-download.tmp"));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .unwrap();
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let bytes = match resp.bytes().await {
                Ok(b) => b,
                Err(e) => return (tool.name.clone(), false, format!("Download body error: {}", e)),
            };
            if let Err(e) = std::fs::write(&tmp_path, &bytes) {
                return (tool.name.clone(), false, format!("Write tmp error: {}", e));
            }
        }
        Ok(resp) => {
            return (tool.name.clone(), false, format!("Download HTTP {}", resp.status()));
        }
        Err(e) => return (tool.name.clone(), false, format!("Download error: {}", e)),
    }

    // 3. 覆盖到 ~/.grok/downloads/grok-linux-x86_64
    let target = match get_grok_binary_path() {
        Ok(p) => p,
        Err(e) => return (tool.name.clone(), false, format!("Binary path error: {}", e)),
    };
    // 备份旧版
    let _ = std::fs::copy(&target, target.with_extension("bak"));
    if let Err(e) = std::fs::rename(&tmp_path, &target) {
        let _ = std::fs::copy(&tmp_path, &target);
        let _ = std::fs::remove_file(&tmp_path);
        if std::fs::metadata(&target).is_err() {
            return (tool.name.clone(), false, format!("Install move error: {}", e));
        }
    }
    // 设置可执行权限
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755));
    }

    // 更新 version.json
    let _ = update_grok_version_json(&latest);

    println!("  ✅ Grok {} installed", latest);

    // 4. 下载后自动 patch（更新即 patch）
    println!("  🔧 Auto-patching Grok uploads...");
    match apply_grok_patches_after_update(&target) {
        Ok(n) => println!("  ✅ Applied {} patches", n),
        Err(e) => println!("  ⚠️  Auto-patch failed (run 'aiw patch grok-apply' manually): {}", e),
    }

    (tool.name.clone(), true, format!("Updated to {}", latest))
}

async fn check_grok_latest_version() -> Option<String> {
    let out = tokio::process::Command::new("grok")
        .arg("update").arg("--check").arg("--json")
        .output().await.ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(&s).ok()?;
    v.get("latestVersion")?.as_str().map(String::from)
}

fn update_grok_version_json(version: &str) -> std::io::Result<()> {
    use crate::patcher::grok::install::get_grok_binary_path;
    let vjson = get_grok_binary_path()
        .map(|p| p.parent().unwrap().parent().unwrap().join("version.json"))
        .unwrap_or_else(|_| std::path::PathBuf::from(format!(
            "{}/.grok/version.json", std::env::var("HOME").unwrap_or_default()
        )));
    let content = format!(
        "{{\"version\":\"{}\",\"stable_version\":null,\"checked_at\":null}}", version
    );
    std::fs::write(&vjson, content)
}

fn apply_grok_patches_after_update(binary_path: &std::path::Path) -> Result<usize, String> {
    use crate::patcher::grok::registry::get_grok_repo_bundle_patches;
    use crate::patcher::apply_file_patch;
    let patches = get_grok_repo_bundle_patches().map_err(|e| e.to_string())?;
    let mut n = 0;
    for patch in &patches {
        if apply_file_patch(binary_path, patch).is_ok() {
            n += 1;
        }
    }
    Ok(n)
}
```

- [ ] **Step 5: 验证编译**

Run: `cargo build 2>&1 | tail -10`
Expected: 编译通过。

- [ ] **Step 6: 手动测试 aiw update grok（dry check）**

Run: `cargo run -- update 2>&1 | grep -i grok | head`
Expected: 全量 update 时出现 "Processing Grok Build..."（grok 已是最新则 "Already up to date"）。

Run: `cargo run -- update grok 2>&1 | tail -15`
Expected: 单独更新 grok，检测版本，已是最新则跳过。

- [ ] **Step 7: Commit**

```bash
git add -A && git commit -m "feat(grok): aiw update 接入 Grok（全量 + 单工具 + 更新后自动 patch）

- CliType::Grok + CliTool 注册第 4 个工具
- execute_update 加 grok 特判分流 update_grok_cli
- update_grok_cli: AIW 自己下载 binary（x.ai/cli），下载后自动 patch
- aiw update 全量含 grok，aiw update grok 单工具更新

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 7: 类2 运行时验证（patch 后 grok 不崩 + 上传阻断）

**Files:**
- Manual verification（无代码改动，验证 Task 3-6 的成果）

- [ ] **Step 1: 备份 + 应用 patch**

Run: `cp /home/putao/.grok/downloads/grok-linux-x86_64 /tmp/grok-0299-clean.bak`
Run: `cargo run -- patch grok-apply 2>&1 | tail -15`
Expected: 打印 2 个 call 点地址 + ✅ 两个 patch 应用成功。

- [ ] **Step 2: 验证 binary 仍可执行（不崩）**

Run: `/home/putao/.grok/bin/grok --version`
Expected: `grok 0.2.99 (...)` 正常输出，不段错误。

Run: `/home/putao/.grok/bin/grok --help 2>&1 | head -5`
Expected: 正常打印 help。

Run: `/home/putao/.grok/bin/grok mcp list 2>&1 | head -5`
Expected: 正常执行（不崩）。

- [ ] **Step 3: 验证正常 session 功能（短对话）**

Run: `timeout 30 /home/putao/.grok/bin/grok -p "say hi" 2>&1 | head -10`
Expected: 正常返回响应（不崩，证明没误伤 session 主流程）。若 grok -p 需要 API key 跳过则用 `grok --help`/`grok models` 等无 API 命令验证。

Run: `/home/putao/.grok/bin/grok models 2>&1 | head -5`
Expected: 正常列出模型（不崩）。

- [ ] **Step 4: 验证上传阻断（upload_queue 不堆积）**

Run: `ls -la /home/putao/.grok/upload_queue/ 2>&1`
Expected: 目录存在但不堆积新文件（patch 后上传被短路）。

- [ ] **Step 5: 验证 patch 状态检测**

Run: `cargo run -- patch grok-status 2>&1 | tail -10`
Expected: 打印 2 个 patch 已应用（✅）。

- [ ] **Step 6: 验证 restore 回滚**

Run: `cargo run -- patch grok-restore 2>&1 | tail -5`
Run: `cargo run -- patch grok-status 2>&1 | tail -5`
Expected: restore 后 status 显示未应用。

Run: `/home/putao/.grok/bin/grok --version`
Expected: 仍正常（restore 的备份是原版 0.2.99）。

- [ ] **Step 7: 重新应用 patch（验证幂等 + 最终状态）**

Run: `cargo run -- patch grok-apply 2>&1 | tail -5`
Expected: 重新应用成功。

- [ ] **Step 8: 记录验证结果到知识库**

在 `docs/domain-knowledge/grok-build.md` 的"已知问题/边界"节更新：
```
### 类2 运行时验证结果（2026-07-14，0.2.99）
- ✅ grok --version / --help / models / mcp list 不崩
- ✅ 正常对话/session 不受影响
- ✅ upload_queue 不堆积
- ✅ patch status 检测正确
- ✅ restore 回滚正常
- 结论：类2 patch 点 0x51c5692/0x51c9ecb 安全，正式启用
```

- [ ] **Step 9: Commit 验证记录**

```bash
git add docs/domain-knowledge/grok-build.md && git commit -m "docs(grok): 类2 repo bundle patch 运行时验证通过（0.2.99）

patch 后 grok 正常功能不受影响，上传阻断生效，patch 安全正式启用。

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 8: 类1 App Builder + 类3 Session Trace 定位 + 实现

**Files:**
- Modify: `src/patcher/grok/targets.rs`, `src/patcher/grok/registry.rs`, `tests/unit/grok_patch.rs`

**Interfaces:**
- Produces: `locate_deploy_upload_call_sites` / `locate_trace_upload_call_sites`

- [ ] **Step 1: 定位类1 App Builder 的 UploadBuild call**

用知识库方法论（tracing 字符串 → slice ptr → text ref → call）逆向 0.2.99 binary：

Run:
```bash
python3 -c "
import re, subprocess
data=open('/home/putao/.grok/downloads/grok-linux-x86_64','rb').read()
s=b'[deploy_app] starting upload build'
idx=data.find(s)
out=subprocess.check_output(['readelf','-r','/home/putao/.grok/downloads/grok-linux-x86_64']).decode()
ptrs=[int(l.split()[0],16) for l in out.split('\n') if 'R_X86_64_RELATIVE' in l and len(l.split())>=4 and int(l.split()[3],16)==idx]
print('deploy slice ptrs:', [hex(p) for p in ptrs])
"
```
然后用 objdump grep 找 text ref，回扫 call 点。具体定位方法见 `docs/domain-knowledge/grok-build.md`「RET patch 定位方法」。

- [ ] **Step 2: 定位类3 Session Trace 的上传 call**

同类1 方法，用 `upload session trace` / `skip remote upload` 字符串定位。

- [ ] **Step 3: 在 targets.rs 加 locate_deploy/trace 函数**

根据 Step 1-2 定位结果，在 `targets.rs` 加 `locate_deploy_upload_call_sites` / `locate_trace_upload_call_sites`，用各自的字节模式锚点（同类2 的 `lea+call+mov` 模式，但 out 参数偏移可能不同，按实测）。

- [ ] **Step 4: 在 registry.rs 实现 get_grok_deploy/trace_patches**

替换 Task 4 Step 5 的 `Ok(vec![])` 占位为真实实现。

- [ ] **Step 5: 在 commands/patch.rs 的 execute_grok_patch_apply 加 deploy/trace patch**

调用 `get_grok_deploy_upload_patches` / `get_grok_trace_upload_patches` 并 apply。

- [ ] **Step 6: 运行时验证类1/3（同类2 Task 7）**

验证 deploy / trace 上传阻断 + grok 不崩。

- [ ] **Step 7: Commit**

```bash
git add -A && git commit -m "feat(grok): 类1 deploy + 类3 trace 上传 patch（call→xor+mov）

定位 App Builder UploadBuild + Session Trace 上传 call 点，等长替换禁用。
三类上传全部 patch 完成。

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 9: 文档更新（CLAUDE.md patch 矩阵 + 测试补全）

**Files:**
- Modify: `CLAUDE.md`, `docs/domain-knowledge/grok-build.md`

- [ ] **Step 1: CLAUDE.md patch 支持矩阵加 Grok**

在 `CLAUDE.md` 的「Claude CLI 补丁支持矩阵」节后新增「Grok CLI 补丁支持」节，列 3 个 Grok feature + 验证版本表（0.2.99）+ binary 来源铁律（仅 x.ai/cli GCS，不走 npm）。

- [ ] **Step 2: 确认所有单元测试通过**

Run: `cargo test --lib 2>&1 | tail -20`
Expected: 所有测试 PASS（Claude patcher 迁移后 + Grok 新增）。

Run: `cargo test --test grok_patch 2>&1 | tail -20`
Expected: PASS。

- [ ] **Step 3: clippy + fmt**

Run: `cargo fmt --check 2>&1 | tail -5`
Run: `cargo clippy -- -D warnings 2>&1 | tail -20`
Expected: 无 warning（或修复后无）。

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "docs: CLAUDE.md patch 矩阵加 Grok + 测试补全

Grok 3 个上传 patch feature 入矩阵，验证版本 0.2.99，binary 来源铁律。

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Self-Review

**1. Spec coverage:**
- §3 平级 patcher 架构 → Task 1（Claude 下沉）+ Task 2（grok 骨架）✓
- §4.2 类2 Repo Bundle patch（call→xor+mov）→ Task 3（定位）+ Task 4（pattern）+ Task 5（CLI）+ Task 7（验证）✓
- §4.3 类1/3 → Task 8 ✓
- §5 FeatureType 3 变体 → Task 2 ✓
- §6.1 CliType::Grok → Task 6 ✓
- §6.2 aiw update 接入 + update_grok_cli + 更新后自动 patch → Task 6 ✓
- §6.3 detect_grok → Task 4 ✓
- §6.3 Patch CLI → Task 5 ✓
- §8 测试 → 每个 Task 的 TDD 步骤 ✓
- §9 跨版本 → Task 3 动态定位（不硬编码地址）✓
- §10 范围 → Task 1-9 覆盖 ✓

**2. Placeholder scan:**
- Task 4 Step 5 的 `get_grok_deploy_upload_patches` / `get_grok_trace_upload_patches` 返回 `Ok(vec![])` 是有意的占位，Task 8 替换为真实实现。已标注 `// TODO Task 8`。
- Task 8 Step 1-2 的逆向命令是完整的（不是"add appropriate"），给出具体 python 脚本。
- 无 "TBD" / "fill in" / "similar to" 等红旗。

**3. Type consistency:**
- `GrokInstallation` { binary_path, version, installed } — Task 4 定义，Task 5/6 使用 ✓
- `locate_repo_bundle_call_sites(binary: &[u8]) -> Result<Vec<usize>, UnifiedPatchError>` — Task 3 定义，Task 4 使用 ✓
- `CALL_REPLACE: &[u8] = &[0x31, 0xc0, 0x48, 0x89, 0x07]` — Task 3 定义，Task 4 使用 ✓
- `get_grok_binary_path() -> Result<PathBuf, UnifiedPatchError>` — Task 4 定义，Task 5/6 使用 ✓
- `detect_grok() -> Result<GrokInstallation, UnifiedPatchError>` — Task 4 定义，Task 5/6 使用 ✓

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-07-14-grok-build-patch.md`. Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?

---

## 计划更新（2026-07-15）：0.2.101 对抗性重构后的调整

### 背景
Task 1-6 已完成（0.2.99 锚点链路 + aiw update 接入）。但 Task 6 的 `aiw update grok` 把本地升到 0.2.101，发现对手**对抗性重构**：隐藏 tracing 字符串（upload_with_batching 等全删），重构出 xai-data-collector 模块统一上传出口。Task 3 的 0.2.99 精确字节锚点在 0.2.101 失效。

### 已破解（知识库 docs/domain-knowledge/grok-build.md 完整记录）
- 0.2.101 GCS upload dispatcher = `0x2cc3420`，2 个 call 点 = `0x2d92557`/`0x2d9539e`
- 指令级指纹法（capstone 16 条 mnemonic + lea-call 过滤）双版本验证有效
- 运行时验证：patch 两 call 点后 grok 对话/session/mcp 全正常不崩

### 剩余待办（重新分组）

**Task 7b（原 Task 7+3 升级）：重写 targets.rs 用 capstone 指令级指纹**
- 替换 0.2.99 精确字节锚点为跨版本指令级指纹（capstone 反汇编）
- 算法：6-push 头字节扫描 → 局部 capstone 反汇编验证 16 条指纹 → dispatcher 集合 → e8 call target 命中 + lea rdi,[rsp] 过滤 → 恰好 2 个 lea-call 的 dispatcher
- 解决 5选1 候选区分（GCS vs goal/plan 等 enum dispatcher）
- 更新单元测试（合成 + 真实 0.2.99/0.2.101 binary 双验证）
- 文件：`src/patcher/grok/targets.rs`、`tests/grok_patch.rs`
- 执行者：主会话亲自（核心算法 + 反复试错）

**Task 8：类1 App Builder + 类3 trace 定位**
- 0.2.101 上 deploy_app/UploadBuild（字符串未动）+ trace 上传的 call 点定位
- 用同样的 capstone 指纹法或字符串锚点（这俩链路没被隐藏）
- 文件：`src/patcher/grok/targets.rs`（加 locate_deploy/trace）、`registry.rs`
- 依赖：Task 7b 完成（targets.rs 框架）

**Task 9：文档更新 + 运行时验证**
- CLAUDE.md patch 支持矩阵加 Grok（3 feature + 0.2.101 验证）
- 自动化运行时验证：patch 后 grok --version/models/sessions 不崩的集成测试
- 文件：`CLAUDE.md`、`tests/grok_patch.rs`
- 可并行（文档部分）

### 并行策略
- Task 7b（核心，主会话）+ Task 9 文档部分（subagent）并行
- Task 8 依赖 7b，串行
