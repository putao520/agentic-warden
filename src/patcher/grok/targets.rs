//! Grok 上传 patch 锚点定位（capstone 指令级指纹 + 版本表）
//!
//! 对抗 0.2.101+ 的 tracing 字符串隐藏：用指令结构指纹定位 GCS upload
//! dispatcher，再找调用它的 2 个 call 点。
//!
//! ## 定位策略（分层）
//!
//! 1. **指纹定位候选**：扫 6-push dispatcher 头（字节级）→ 局部 capstone
//!    反汇编验证 16 条指令指纹（push×6, sub, mov×3, movzx, lea, movsxd,
//!    add, mov, jmp）→ 得 enum jump-table dispatcher 集合。
//! 2. **call 点定位**：扫 `e8 rel32` call，target 命中 dispatcher 集合 +
//!    call 前 11 字节是 `lea rdi,[rsp+disp32]`（out 参数入 rdi）。
//! 3. **GCS 区分**（版本表）：dispatcher 集合里多个候选同构（Rust 泛型
//!    monomorphization），静态无法区分。用版本表记录每版本 GCS dispatcher
//!    的区分特征（栈帧大小，0.2.99 独有 0x4f8；0.2.101 用地址 fallback）。
//!
//! ## 5 候选同构现实
//!
//! 0.2.101 实测：5 个 enum dispatcher 二进制 100% 同构（同 prologue、同
//! jump table arm 相对偏移、同调用模式），是同一泛型代码的 5 份拷贝
//! （repo_changes + goal/plan/skeptic/memory 等状态机）。静态无法区分 GCS，
//! 必须用版本表。详见 docs/domain-knowledge/grok-build.md。
//!
//! patch：call `e8 rel32`(5B) → `31 c0 48 89 07`(5B, xor eax,eax; mov [rdi],rax)，
//! 让上传函数返回空结果，调用方走"无结果→跳过"分支。

use crate::patcher::types::UnifiedPatchError;
use capstone::arch::x86::ArchMode;
use capstone::{arch::BuildsCapstone, Capstone};

/// 5 字节替换字节：`xor eax,eax; mov [rdi],rax`
///
/// 替换 call 后，rax=0 且 out 参数 *rdi 写 0，调用方 `cmp; jcc` 走无结果路径。
/// 运行时验证（0.2.101）：patch 后 grok 对话/session/mcp 全正常不崩。
pub const CALL_REPLACE: &[u8] = &[0x31, 0xc0, 0x48, 0x89, 0x07];

/// 6-push dispatcher 头字节（10 字节固定 + sub rsp 7 字节，栈帧通配）。
///
/// `55 41 57 41 56 41 55 41 54 53 48 81 ec ?? ?? 00 00`
/// 注意：栈帧 2 字节通配用专门扫描（不能用 regex `.`，因 `\x0a`=\n 会被 `.` 拒绝；
/// 栈帧 0xa58 的字节含 0x0a）。
const DISPATCHER_HEAD: &[u8] = &[
    0x55, 0x41, 0x57, 0x41, 0x56, 0x41, 0x55, 0x41, 0x54, 0x53, 0x48, 0x81, 0xec,
];

/// dispatcher 指令指纹（capstone Intel 语法的 mnemonic 序列，16 条）。
///
/// `push×6, sub, mov, mov, mov, movzx, lea, movsxd, add, mov, jmp`
/// 注意 capstone 用 Intel 语法：`movzx`（非 AT&T 的 movzbl）、`movsxd`（非 movslq）。
const DISPATCHER_FP: &[&str] = &[
    "push", "push", "push", "push", "push", "push", "sub", "mov", "mov", "mov", "movzx", "lea",
    "movsxd", "add", "mov", "jmp",
];

/// call 前 11 字节处的 out 参数 lea 前缀：`lea rdi,[rsp+disp32]` = `48 8d bc 24`
const LEA_RDI_RSP: &[u8] = &[0x48, 0x8d, 0xbc, 0x24];

/// GCS dispatcher 版本表。
///
/// 每版本记录 GCS dispatcher 的区分特征。因 0.2.101+ 候选同构，栈帧无法区分时
/// 用地址 fallback。
///
/// - `stack_frame`：GCS dispatcher 的栈帧大小。0.2.99 GCS 独有 0x4f8（其他候选
///   0xa58），可区分。0.2.101 全 0xa58，栈帧失效，用 `addr_fallback`。
/// - `addr_fallback`：GCS dispatcher 的文件偏移（栈帧失效时兜底）。
#[allow(dead_code)]
struct GcsVersionEntry {
    version_prefix: &'static str,
    stack_frame: u32,
    addr_fallback: Option<usize>,
}

const GCS_TABLE: &[GcsVersionEntry] = &[
    GcsVersionEntry {
        version_prefix: "0.2.9",
        stack_frame: 0x4f8,
        addr_fallback: None, // 0.2.99 栈帧 0x4f8 唯一，可区分
    },
    GcsVersionEntry {
        version_prefix: "0.2.10",
        stack_frame: 0xa58,
        addr_fallback: Some(0x2cc3420), // 0.2.101 全 0xa58，用地址兜底
    },
];

/// 查版本的 GCS 区分特征。
fn lookup_gcs_entry(version: &str) -> Option<&'static GcsVersionEntry> {
    GCS_TABLE.iter().find(|e| version.starts_with(e.version_prefix))
}

/// 初始化 capstone（x86-64）。
fn make_cs() -> Result<Capstone, UnifiedPatchError> {
    Capstone::new()
        .x86()
        .mode(ArchMode::Mode64)
        .build()
        .map_err(|e| UnifiedPatchError::Other(format!("capstone init: {}", e)))
}

/// 读 dispatcher 头处的栈帧大小（sub rsp 的 imm32）。
fn read_stack_frame(binary: &[u8], disp_off: usize) -> Option<u32> {
    // sub rsp, imm32 在 head+13 处：48 81 ec XX XX 00 00
    let p = disp_off + DISPATCHER_HEAD.len();
    if p + 4 > binary.len() {
        return None;
    }
    Some(u32::from_le_bytes([
        binary[p],
        binary[p + 1],
        binary[p + 2],
        binary[p + 3],
    ]))
}

/// 验证某偏移处是 dispatcher 指纹（局部 capstone 反汇编 16 条）。
fn matches_dispatcher_fp(cs: &Capstone, binary: &[u8], off: usize) -> bool {
    let end = (off + 0x60).min(binary.len());
    if off >= end {
        return false;
    }
    let insns = match cs.disasm_all(&binary[off..end], off as u64) {
        Ok(i) => i,
        Err(_) => return false,
    };
    let mnems: Vec<&str> = insns.iter().take(16).map(|i| i.mnemonic().unwrap_or("")).collect();
    mnems.as_slice() == DISPATCHER_FP
}

/// 解析 `call rel32` 的目标地址（call_off 是 0xe8 的偏移）。
fn call_target(binary: &[u8], call_off: usize) -> Option<usize> {
    if call_off + 5 > binary.len() || binary[call_off] != 0xe8 {
        return None;
    }
    let rel32 = i32::from_le_bytes([
        binary[call_off + 1],
        binary[call_off + 2],
        binary[call_off + 3],
        binary[call_off + 4],
    ]);
    (call_off as i64 + 5 + rel32 as i64)
        .try_into()
        .ok()
        .filter(|&t: &usize| t < binary.len())
}

/// 定位 Repo Changes git bundle 上传的 2 个 call 调用点。
///
/// 算法：指纹定位 dispatcher 候选 → 版本表区分 GCS → 找 GCS 的 2 个 call 点。
pub fn locate_repo_bundle_call_sites(binary: &[u8]) -> Result<Vec<usize>, UnifiedPatchError> {
    locate_repo_bundle_call_sites_for(binary, None)
}

/// 带版本提示的定位（用于运行时按 grok 版本选 GCS 区分特征）。
pub fn locate_repo_bundle_call_sites_versioned(
    binary: &[u8],
    version: &str,
) -> Result<Vec<usize>, UnifiedPatchError> {
    locate_repo_bundle_call_sites_for(binary, Some(version))
}

fn locate_repo_bundle_call_sites_for(
    binary: &[u8],
    version: Option<&str>,
) -> Result<Vec<usize>, UnifiedPatchError> {
    let cs = make_cs()?;

    // Step 1: 字节级扫 dispatcher 头，记录 (off, stack_frame)
    let mut candidates: Vec<(usize, u32)> = Vec::new();
    let mut i = 0;
    while i + DISPATCHER_HEAD.len() + 4 <= binary.len() {
        if &binary[i..i + DISPATCHER_HEAD.len()] == DISPATCHER_HEAD {
            // 栈帧高 16 位必须为 0（sub rsp, imm32 的 imm32 高 16 位 0）
            let p = i + DISPATCHER_HEAD.len();
            if binary[p + 2] == 0 && binary[p + 3] == 0 {
                // 局部 capstone 验证指纹
                if matches_dispatcher_fp(&cs, binary, i) {
                    let frame = read_stack_frame(binary, i).unwrap_or(0);
                    candidates.push((i, frame));
                }
            }
        }
        i += 1;
    }

    // Step 2: 版本表选 GCS dispatcher
    let entry = version.and_then(lookup_gcs_entry);
    let gcs_disp: usize = if let Some(e) = entry {
        // 优先栈帧区分
        let by_frame: Vec<_> = candidates
            .iter()
            .filter(|(_, f)| *f == e.stack_frame)
            .map(|(o, _)| *o)
            .collect();
        if by_frame.len() == 1 {
            by_frame[0]
        } else if let Some(addr) = e.addr_fallback {
            // 栈帧失效，地址兜底
            if candidates.iter().any(|(o, _)| *o == addr) {
                addr
            } else {
                return Err(UnifiedPatchError::PatternNotFound(format!(
                    "GCS dispatcher addr fallback {:#x} not in candidates (version {})",
                    addr,
                    e.version_prefix
                )));
            }
        } else {
            return Err(UnifiedPatchError::PatternNotFound(format!(
                "GCS stack_frame {:#x} matched {} candidates, no addr fallback (version {})",
                e.stack_frame,
                by_frame.len(),
                e.version_prefix
            )));
        }
    } else {
        // 无版本提示或未知版本：退化到栈帧 0x4f8（0.2.93-0.2.99 有效）
        let by_frame: Vec<_> = candidates
            .iter()
            .filter(|(_, f)| *f == 0x4f8)
            .map(|(o, _)| *o)
            .collect();
        if by_frame.len() == 1 {
            by_frame[0]
        } else {
            return Err(UnifiedPatchError::PatternNotFound(format!(
                "cannot identify GCS dispatcher without version hint ({} candidates, {} with frame 0x4f8). \
                 pass version or update GCS_TABLE",
                candidates.len(),
                by_frame.len()
            )));
        }
    };

    // Step 3: 扫 call e8 rel32，target == gcs_disp + call 前 11 字节是 lea rdi,[rsp]
    let mut sites = Vec::new();
    let mut j = 0;
    while j + 5 <= binary.len() {
        if binary[j] == 0xe8 {
            if let Some(tgt) = call_target(binary, j) {
                if tgt == gcs_disp && j >= 11 && &binary[j - 11..j - 7] == LEA_RDI_RSP {
                    sites.push(j);
                }
            }
        }
        j += 1;
    }

    if sites.is_empty() {
        return Err(UnifiedPatchError::PatternNotFound(format!(
            "no lea-call sites targeting GCS dispatcher {:#x}",
            gcs_disp
        )));
    }
    Ok(sites)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_is_5_bytes() {
        assert_eq!(CALL_REPLACE.len(), 5);
    }

    #[test]
    fn test_call_target_parsing() {
        let mut bin = vec![0u8; 0x200];
        bin[0x10] = 0xe8;
        bin[0x11..0x15].copy_from_slice(&0x100i32.to_le_bytes());
        assert_eq!(call_target(&bin, 0x10), Some(0x115));
    }

    #[test]
    fn test_call_target_negative_rel32() {
        let mut bin = vec![0u8; 0x200];
        bin[0x100] = 0xe8;
        bin[0x101..0x105].copy_from_slice(&(-0x10i32).to_le_bytes());
        assert_eq!(call_target(&bin, 0x100), Some(0xF5));
    }

    #[test]
    fn test_call_target_rejects_non_e8() {
        let bin = vec![0u8; 0x20];
        assert_eq!(call_target(&bin, 0x10), None);
    }

    #[test]
    fn test_lookup_gcs_entry_029() {
        let e = lookup_gcs_entry("0.2.99").unwrap();
        assert_eq!(e.stack_frame, 0x4f8);
        assert!(e.addr_fallback.is_none());
    }

    #[test]
    fn test_lookup_gcs_entry_02101() {
        let e = lookup_gcs_entry("0.2.101").unwrap();
        assert_eq!(e.stack_frame, 0xa58);
        assert_eq!(e.addr_fallback, Some(0x2cc3420));
    }

    #[test]
    fn test_lookup_gcs_entry_unknown_version() {
        assert!(lookup_gcs_entry("9.9.9").is_none());
    }
}
