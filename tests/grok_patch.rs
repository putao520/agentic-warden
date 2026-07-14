//! Grok patch 集成测试：在真实 0.2.99 binary 上定位 GCS 上传 call 点

use aiw::patcher::grok::targets::locate_repo_bundle_call_sites;

#[test]
fn test_locate_repo_bundle_call_sites_on_v0299() {
    let binary = std::fs::read("/home/putao/.grok/downloads/grok-linux-x86_64").unwrap();
    let sites = locate_repo_bundle_call_sites(&binary).expect("locate failed");
    // 必须找到恰好 2 个 call 点（0.2.99: 0x51c5692 / 0x51c9ecb）
    assert_eq!(
        sites.len(),
        2,
        "expected 2 GCS upload call sites, got {:?}",
        sites.iter().map(|s| format!("{:#x}", s)).collect::<Vec<_>>()
    );
    // 验证每个点都是 e8（call rel32）
    for &off in &sites {
        assert_eq!(binary[off], 0xe8, "call opcode at {:#x} should be 0xe8", off);
    }
    // 验证 call 前是 lea 0x3d0(%rsp),%rdi（48 8d bc 24 d0 03 00 00）+ 3B gap
    // 布局: lea(8B) at off-11, gap(3B) at off-3, call at off
    let lea: &[u8] = &[0x48, 0x8d, 0xbc, 0x24, 0xd0, 0x03, 0x00, 0x00];
    for &off in &sites {
        let lea_bytes = &binary[off - 11..off - 3];
        assert_eq!(lea_bytes, lea, "lea before call at {:#x} mismatch", off);
    }
    // 验证 call target 是 GCS dispatcher（prologue 在 target 处）
    let prologue: &[u8] = &[
        0x55, 0x41, 0x57, 0x41, 0x56, 0x41, 0x55, 0x41, 0x54, 0x53, 0x48, 0x81, 0xec, 0xf8, 0x04,
        0x00, 0x00,
    ];
    for &off in &sites {
        let rel32 = i32::from_le_bytes([
            binary[off + 1],
            binary[off + 2],
            binary[off + 3],
            binary[off + 4],
        ]);
        let tgt = (off as i64 + 5 + rel32 as i64) as usize;
        assert_eq!(
            &binary[tgt..tgt + prologue.len()],
            prologue,
            "call target {:#x} should be GCS dispatcher",
            tgt
        );
    }
}

