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
        "expected 2 GCS upload call sites, got {}",
        sites.len()
    );
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
