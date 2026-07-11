#!/usr/bin/env python3
"""
CC 2.1.207 binary 全量 patch 审计脚本

对 8 个已知 patch 点验证命中 + 等长约束 + 抽取变量名（供 CLAUDE.md 矩阵更新）。
同时审计是否有新增间谍点 / 功能门控（tengu/firstParty/relay 歧视 / 新上报端点）。

用法: python3 verify_207.py <binary>
"""
import re
import sys
import os

BIN = sys.argv[1] if len(sys.argv) > 1 else "/srv/home-links/.local/share/claude/versions/2.1.207"

if not os.path.exists(BIN):
    print(f"FATAL: binary not found: {BIN}")
    sys.exit(2)

with open(BIN, "rb") as f:
    data = f.read()

print(f"=== CC binary 审计: {BIN} ===")
print(f"size: {len(data)} bytes\n")

# ---------- 8 个已知 patch 点 ----------

# 1. MaxContextTokens (regex, 变量名通配)
MAX_TOKEN_RE = rb"var [a-zA-Z_$][a-zA-Z0-9_$]*=200000,[a-zA-Z_$][a-zA-Z0-9_$]*=200000[^;]*;"
# 2. AntiTelemetry (literal)
TELEM = b"/api/event_logging/v2/batch"
# 3. AntiSpy escape (regex)
ESCAPE_RE = rb"if\([a-zA-Z_$][a-zA-Z0-9_$]*\._CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL\)return!0"
# 4. AntiSpy timezone (literal)
TZ = b"Intl.DateTimeFormat().resolvedOptions().timeZone"
# 5. AntiPromptBias (regex)
PROMPTBIAS_RE = rb'if\([a-zA-Z_$][a-zA-Z0-9_$]*\(\)\)n\.push\("\*\*Provider context:\*\* This session is not using'
# 6. AntiAtis (regex)
ATIS_RE = rb'function [a-zA-Z_$][a-zA-Z0-9_$]*\(\)\{let e=[a-zA-Z_$][a-zA-Z0-9_$]*\(\)[?]\.atis;return typeof e==="string"&&e\.length>0[?]e:void 0\}'
# 7. AntiFrameTrack (literal)
FRAME = b"/api/frame/track"
# 8. AntiCloudDetect (literal)
CLOUD = b"/^42:01/"

def count_literal(needle: bytes):
    if not needle:
        return 0, []
    idxs = []
    start = 0
    while True:
        i = data.find(needle, start)
        if i < 0:
            break
        idxs.append(i)
        start = i + 1
    return len(idxs), idxs

def find_regex(pat: bytes):
    re_obj = re.compile(pat, re.DOTALL)
    return list(re_obj.finditer(data))

# ---- MaxContextTokens: 抽变量名 ----
print("【1】MaxContextTokens (regex)")
mt = find_regex(MAX_TOKEN_RE)
print(f"  命中: {len(mt)} 处")
if mt:
    for m in mt[:3]:
        snippet = data[m.start():m.end()].decode('latin1')
        # 抽前两个变量名
        names = re.findall(rb'([a-zA-Z_$][a-zA-Z0-9_$]*)=200000', data[m.start():m.end()])
        print(f"  offset={m.start()} block={snippet[:80]}... names={[n.decode() for n in names[:2]]}")
print()

# ---- AntiTelemetry ----
print("【2】AntiTelemetry (/api/event_logging/v2/batch)")
n, _ = count_literal(TELEM)
print(f"  命中: {n} 处\n")

# ---- AntiSpy escape ----
print("【3a】AntiSpy escape hatch (_CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL)")
esc = find_regex(ESCAPE_RE)
print(f"  命中: {len(esc)} 处")
for m in esc[:3]:
    snippet = data[m.start():m.end()].decode('latin1')
    obj = re.findall(rb'if\(([a-zA-Z_$][a-zA-Z0-9_$]*)\._CLAUDE_CODE', data[m.start():m.end()])
    print(f"  offset={m.start()} obj={[o.decode() for o in obj]} snippet={snippet[:70]}...")
print()

# ---- AntiSpy timezone ----
print("【3b】AntiSpy timezone (Intl.DateTimeFormat)")
n, idxs = count_literal(TZ)
print(f"  命中: {n} 处  offsets={idxs[:5]}\n")

# ---- AntiPromptBias ----
print("【4】AntiPromptBias (Provider context)")
pb = find_regex(PROMPTBIAS_RE)
print(f"  命中: {len(pb)} 处")
for m in pb[:3]:
    snippet = data[m.start():m.end()].decode('latin1')
    fn = re.findall(rb'if\(([a-zA-Z_$][a-zA-Z0-9_$]*)\(\)\)', data[m.start():m.end()])
    print(f"  offset={m.start()} fn={[f.decode() for f in fn]} snippet={snippet[:60]}...")
print()

# ---- AntiAtis ----
print("【5】AntiAtis (x-cc-atis extract fn)")
atis = find_regex(ATIS_RE)
print(f"  命中: {len(atis)} 处")
for m in atis[:3]:
    snippet = data[m.start():m.end()].decode('latin1')
    fn = re.findall(rb'function ([a-zA-Z_$][a-zA-Z0-9_$]*)\(\)', data[m.start():m.end()])
    bf = re.findall(rb'\{let e=([a-zA-Z_$][a-zA-Z0-9_$]*)\(\)', data[m.start():m.end()])
    print(f"  offset={m.start()} fn={[f.decode() for f in fn]} bootstrap={[b.decode() for b in bf]}")
print()

# ---- AntiFrameTrack ----
print("【6】AntiFrameTrack (/api/frame/track)")
n, idxs = count_literal(FRAME)
print(f"  命中: {n} 处  offsets={idxs[:5]}\n")

# ---- AntiCloudDetect ----
print("【7】AntiCloudDetect (/^42:01/)")
n, idxs = count_literal(CLOUD)
print(f"  命中: {n} 处  offsets={idxs[:5]}\n")

# ---------- 新增间谍点 / 功能门控审计 ----------
print("=" * 60)
print("【审计】新增间谍点 / 功能门控扫描")
print("=" * 60)

# 已知上报端点 (patch 后)
KNOWN_ENDPOINTS = [
    b"/api/event_logging/v2/batch",
    b"/api/frame/track",
]
# 扫描所有 /api/ 端点
api_endpoints = set()
for m in re.finditer(rb'/api/[a-zA-Z0-9_/]+', data):
    ep = data[m.start():m.end()]
    api_endpoints.add(ep)
print(f"\n--- /api/ 端点全集 ({len(api_endpoints)} 个) ---")
for ep in sorted(api_endpoints):
    print(f"  {ep.decode('latin1')}")

# firstParty / relay / cloud / 间谍关键词
print("\n--- 间谍/门控关键词扫描 ---")
KEYWORDS = [
    b"_CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL",
    b"firstParty",
    b"relay",
    b"tengu",
    b"gu()",
    b"tMi(",
    b"eMi(",
    b"known",
    b"cnTZ",
    b"CLAUDE_CODE_",
    b"x-cc-atis",
    b"trackFrameEvent",
    b"networkInterfaces",
    b"42:01",
    b"/Google/",
    b"storage.googleapis.com",
    b"statsig",
    b"telemetry",
]
for kw in KEYWORDS:
    n, idxs = count_literal(kw)
    if n > 0:
        print(f"  {kw.decode('latin1'):50s} -> {n} 处")

# 新增：扫描所有 CLAUDE_CODE_ 环境变量/常量（发现新增门控）
print("\n--- _CLAUDE_CODE_ 全集 ---")
cc_consts = set()
for m in re.finditer(rb'[A-Z_]*_CLAUDE_CODE_[A-Z_]*', data):
    cc_consts.add(data[m.start():m.end()])
for c in sorted(cc_consts):
    print(f"  {c.decode('latin1')}")

print("\n=== 审计完成 ===")
