//! Grok 上传 patch 锚点定位
//!
//! 通过两步字节模式匹配定位 GCS blob 上传 dispatcher 的 2 个调用点：
//! 1. 扫 dispatcher prologue（6-push + sub $0x4f8）→ 得 dispatcher 入口集合
//! 2. 扫 `lea 0x3d0(%rsp),%rdi` + 3B gap + `call rel32`，解析 call target，
//!    target 命中 dispatcher 集合 → 命中 call 点
//!
//! 两条件叠加，跨版本稳定（prologue + lea 序列不变）且零误报。
//! 0.2.93/0.2.99 均命中恰好 2 个 call 点（都指向 GCS dispatcher）。
//!
//! 方法论见 docs/domain-knowledge/grok-build.md。

use crate::patcher::types::UnifiedPatchError;

/// GCS dispatcher prologue：`push %rbp; push %r15; push %r14; push %r13;
/// push %r12; push %rbx; sub $0x4f8,%rsp`
///
/// 字节: 55 41 57 41 56 41 55 41 54 53 48 81 ec f8 04 00 00 (17 字节)
///
/// 跨版本稳定（0.2.93/0.2.99 相同），但在 binary 内非唯一（~34 个大函数同
/// prologue），故需配合 call target 校验。
const DISPATCHER_PROLOGUE: &[u8] = &[
    0x55, 0x41, 0x57, 0x41, 0x56, 0x41, 0x55, 0x41, 0x54, 0x53, 0x48, 0x81, 0xec, 0xf8, 0x04, 0x00,
    0x00,
];

/// call 指令前缀：`lea 0x3d0(%rsp),%rdi`（out 参数地址入 rdi）
///
/// 字节: 48 8d bc 24 d0 03 00 00 (8 字节)
const LEA_OUT_PARAM: &[u8] = &[0x48, 0x8d, 0xbc, 0x24, 0xd0, 0x03, 0x00, 0x00];

/// lea 与 call 之间的 gap 字节数（`mov reg,%rsi` 第二参数，3 字节）
const LEA_CALL_GAP: usize = 3;

/// 5 字节替换字节：`xor eax,eax; mov [rdi],rax`
///
/// 替换 call 后，rax=0 且 out 参数 *rdi 写 0，调用方 `test/cmp; je` 跳过
/// 整个上传结果消费块（含 drop + 上传处理），语义等价"上传返回空结果"。
pub const CALL_REPLACE: &[u8] = &[0x31, 0xc0, 0x48, 0x89, 0x07];

/// 解析 `call rel32` 指令的目标地址。
///
/// `call_off` 是 call opcode（0xe8）的偏移。rel32 是有符号 32 位，
/// target = call_off + 5 + rel32。
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
/// 两步匹配：
/// 1. 扫 DISPATCHER_PROLOGUE 得 dispatcher 入口集合（~34 个候选）
/// 2. 扫 LEA_OUT_PARAM(8) + 3B gap + e8 rel32(5)，解析 call target，
///    target 在 dispatcher 集合内 → 命中
///
/// 0.2.93/0.2.99 均返回恰好 2 个（都指向同一 GCS dispatcher）。
pub fn locate_repo_bundle_call_sites(binary: &[u8]) -> Result<Vec<usize>, UnifiedPatchError> {
    // Step 1: 收集所有 dispatcher prologue 位置
    let mut dispatcher_locs = std::collections::HashSet::new();
    let mut i = 0;
    while i + DISPATCHER_PROLOGUE.len() <= binary.len() {
        if &binary[i..i + DISPATCHER_PROLOGUE.len()] == DISPATCHER_PROLOGUE {
            dispatcher_locs.insert(i);
        }
        i += 1;
    }

    // Step 2: 扫 lea + gap + call，解析 target 匹配 dispatcher
    let mut sites = Vec::new();
    let scan_len = LEA_OUT_PARAM.len() + LEA_CALL_GAP + 5; // 8+3+5=16
    let mut j = 0;
    while j + scan_len <= binary.len() {
        if &binary[j..j + LEA_OUT_PARAM.len()] == LEA_OUT_PARAM
            && binary[j + LEA_OUT_PARAM.len() + LEA_CALL_GAP] == 0xe8
        {
            let call_off = j + LEA_OUT_PARAM.len() + LEA_CALL_GAP;
            if let Some(tgt) = call_target(binary, call_off) {
                if dispatcher_locs.contains(&tgt) {
                    sites.push(call_off);
                }
            }
        }
        j += 1;
    }

    if sites.is_empty() {
        return Err(UnifiedPatchError::PatternNotFound(
            "Grok repo bundle call sites not found (prologue/lea pattern drift?)".to_string(),
        ));
    }
    Ok(sites)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_is_5_bytes() {
        // call e8 rel32 是 5 字节，替换必须等长
        assert_eq!(CALL_REPLACE.len(), 5);
    }

    #[test]
    fn test_call_target_parsing() {
        // call @ 0x10, rel32 = 0x100 → target = 0x10 + 5 + 0x100 = 0x115
        let mut bin = vec![0u8; 0x200];
        bin[0x10] = 0xe8;
        bin[0x11..0x15].copy_from_slice(&0x100i32.to_le_bytes());
        assert_eq!(call_target(&bin, 0x10), Some(0x115));
    }

    #[test]
    fn test_call_target_negative_rel32() {
        // call @ 0x100, rel32 = -0x10 → target = 0x100 + 5 - 0x10 = 0xF5
        let mut bin = vec![0u8; 0x200];
        bin[0x100] = 0xe8;
        bin[0x101..0x105].copy_from_slice(&(-0x10i32).to_le_bytes());
        assert_eq!(call_target(&bin, 0x100), Some(0xF5));
    }

    #[test]
    fn test_call_target_rejects_non_e8() {
        let bin = vec![0u8; 0x20];
        assert_eq!(call_target(&bin, 0x10), None); // 非 0xe8
    }

    #[test]
    fn test_locate_on_synthetic() {
        // 构造合成 binary：dispatcher prologue @ 0x100，2 个 lea+gap+call 指向它
        let mut bin = vec![0u8; 0x1000];
        // dispatcher prologue @ 0x100
        bin[0x100..0x100 + DISPATCHER_PROLOGUE.len()].copy_from_slice(DISPATCHER_PROLOGUE);
        // 2 个 call 点：lea(8) + gap(3) + call rel32(5)，call target = 0x100
        for call_off in [0x200, 0x300] {
            let lea_start = call_off - LEA_CALL_GAP - LEA_OUT_PARAM.len();
            bin[lea_start..lea_start + LEA_OUT_PARAM.len()].copy_from_slice(LEA_OUT_PARAM);
            // gap 3 字节任意（mov reg,%rsi）
            bin[call_off] = 0xe8;
            let rel32 = 0x100i64 - (call_off as i64 + 5);
            bin[call_off + 1..call_off + 5].copy_from_slice(&(rel32 as i32).to_le_bytes());
        }
        let sites = locate_repo_bundle_call_sites(&bin).unwrap();
        assert_eq!(sites.len(), 2, "expected 2 sites, got {}", sites.len());
        assert_eq!(sites, vec![0x200, 0x300]);
        for &off in &sites {
            assert_eq!(bin[off], 0xe8);
        }
    }

    #[test]
    fn test_locate_rejects_call_to_non_dispatcher() {
        // lea+gap+call 指向非 dispatcher 地址 → 不应命中
        let mut bin = vec![0u8; 0x1000];
        bin[0x100..0x100 + DISPATCHER_PROLOGUE.len()].copy_from_slice(DISPATCHER_PROLOGUE);
        // call 指向 0x500（无 prologue）
        let call_off = 0x200;
        let lea_start = call_off - LEA_CALL_GAP - LEA_OUT_PARAM.len();
        bin[lea_start..lea_start + LEA_OUT_PARAM.len()].copy_from_slice(LEA_OUT_PARAM);
        bin[call_off] = 0xe8;
        let rel32 = 0x500i64 - (call_off as i64 + 5);
        bin[call_off + 1..call_off + 5].copy_from_slice(&(rel32 as i32).to_le_bytes());
        let sites = locate_repo_bundle_call_sites(&bin);
        assert!(
            sites.is_err(),
            "should not find sites targeting non-dispatcher"
        );
    }
}
