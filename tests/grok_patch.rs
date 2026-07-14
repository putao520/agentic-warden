//! Grok patch 集成测试：在真实本地 grok binary 上定位 GCS 上传 call 点
//!
//! 本地版本可能是 0.2.99 或 0.2.101（含对抗性重构）。测试用版本表适配。

use aiw::patcher::grok::install::{detect_grok, get_grok_binary_path};
use aiw::patcher::grok::targets::locate_repo_bundle_call_sites_versioned;

#[test]
fn test_locate_repo_bundle_call_sites_local() {
    let binary = std::fs::read("/home/putao/.grok/downloads/grok-linux-x86_64").unwrap();
    let version = detect_grok()
        .ok()
        .map(|i| format!("{}", i.version))
        .unwrap_or_default();
    let sites =
        locate_repo_bundle_call_sites_versioned(&binary, &version).expect("locate failed");
    // 必须找到恰好 2 个 call 点
    assert_eq!(
        sites.len(),
        2,
        "expected 2 GCS upload call sites, got {:?}",
        sites.iter().map(|s| format!("{:#x}", s)).collect::<Vec<_>>()
    );
    // 每个点是 e8（call rel32）
    for &off in &sites {
        assert_eq!(binary[off], 0xe8, "call opcode at {:#x} should be 0xe8", off);
    }
    // call 前 11 字节处是 lea rdi,[rsp+disp32]（48 8d bc 24）—— out 参数入 rdi
    let lea_prefix: &[u8] = &[0x48, 0x8d, 0xbc, 0x24];
    for &off in &sites {
        assert_eq!(
            &binary[off - 11..off - 7],
            lea_prefix,
            "lea rdi,[rsp] prefix before call at {:#x} mismatch",
            off
        );
    }
    // 2 个 call 点指向同一 GCS dispatcher
    let targets: Vec<usize> = sites
        .iter()
        .map(|&off| {
            let rel32 = i32::from_le_bytes([
                binary[off + 1],
                binary[off + 2],
                binary[off + 3],
                binary[off + 4],
            ]);
            (off as i64 + 5 + rel32 as i64) as usize
        })
        .collect();
    assert_eq!(targets[0], targets[1], "both call sites must target same dispatcher");
    // dispatcher 处是 6-push prologue（55 41 57 41 56 41 55 41 54 53）
    let prologue_head: &[u8] = &[0x55, 0x41, 0x57, 0x41, 0x56, 0x41, 0x55, 0x41, 0x54, 0x53];
    assert_eq!(
        &binary[targets[0]..targets[0] + prologue_head.len()],
        prologue_head,
        "GCS dispatcher prologue mismatch"
    );
}


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

