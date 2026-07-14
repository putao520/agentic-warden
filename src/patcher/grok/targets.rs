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
        assert_eq!(
            LEA_OUT_PARAM_PREFIX.len() + 1 + 4 + MOV_OUT_PARAM_SUFFIX.len(),
            21
        );
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
