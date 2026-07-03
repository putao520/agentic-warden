# Claude Code v2.1.195 提示词系统降智审计报告

**审计目标**：`/home/putao/.local/share/claude/versions/2.1.195`（233MB ELF, Bun 打包, 内嵌 minified JS）
**审计日期**：2026-07-02
**审计范围**：firstParty（官方端点）vs 第三方供应商的模型/Prompt/能力差异化分支
**审计方法**：strings + python3 正则提取 + 字节偏移定位 + 函数体追踪

---

## 1. 执行摘要

**结论：CC v2.1.195 对第三方供应商用户存在系统性的「模型降级」，但未发现「Prompt 降智注入」。**

### 核心发现

| 维度 | 是否差异化 | 严重度 | 说明 |
|------|-----------|--------|------|
| **模型默认选择** | ✅ 是 | 🔴 高 | 第三方默认 opus-4-6，firstParty 默认 opus-4-8（差 2 代） |
| **1M 上下文授权** | ✅ 是 | 🟡 中 | 1M context 仅 firstParty+官方host / anthropicAws / mantle 可用 |
| **thinking-token-count beta** | ✅ 是 | 🟡 中 | `thinking-token-count-2026-05-13` beta 仅 firstParty 推送 |
| **overage/credits 权限** | ✅ 是 | 🟢 低 | `eF()` 把第三方排除在 overage credits 体系外（业务逻辑，非降智） |
| **Prompt 文本降智** | ❌ 否 | — | **未发现**任何「对第三方用户降智/限制」的 prompt 指令注入 |
| **能力限制** | ✅ 是 | 🟢 低 | Files API/Channels/DesignSync/Projects/Ultrareview 仅 claude.ai auth（数据驻留/产品限制） |
| **身份 Prompt 差异** | ✅ 是 | 🟢 极低 | 仅 vertex 用不同 base identity；非官方 host 走标准 firstParty identity |

### 关键判定

1. **没有「降智 Prompt」**：CC 不向第三方用户的模型发送「降低智能」「限制能力」之类的指令。所有 firstParty 分支在 prompt 路径上只控制**功能可用性**（gateway discovery、telemetry flush、agent suggestions、prompt-slot 分配），不改变 prompt 内容语义。

2. **真正的降智在「模型选择」**：第三方用户被默认路由到 **opus-4-6**（2 代前的模型），而 firstParty 用户路由到 **opus-4-8**（最新）。这是「被动降智」——用户以为在用最新 opus，实际拿到旧版。子 agent（teammate）模型同样降级。

3. **patch 可行性**：3 个关键函数（`nzt`/`YIe`/`_An`）的等长替换 patch 完全可行（已验证字节级精确匹配 + 等长 JS 合法替换）。

---

## 2. 模型降级清单（核心降智点）

### 2.1 模型配置表 `yc`（字节偏移 226460800）

模型 ID 映射表，每个模型族对应 7 个 provider 的不同 model string：

```javascript
yc = {
  haiku35:  QBr,  // claude-3-5-haiku-20241022
  haiku45:  ZBr,  // claude-haiku-4-5-20251001
  sonnet35: JBr,  // claude-3-5-sonnet-20241022
  sonnet37: XBr,  // claude-3-7-sonnet-20250219
  sonnet40: eUr,  // claude-sonnet-4-20250514
  sonnet45: tUr,  // claude-sonnet-4-5-20250929
  sonnet46: nUr,  // claude-sonnet-4-6
  opus40:   rUr,  // claude-opus-4-20250514
  opus41:   oUr,  // claude-opus-4-1-20250805
  opus45:   sUr,  // claude-opus-4-5-20251101
  opus46:   iUr,  // claude-opus-4-6
  opus47:   aUr,  // claude-opus-4-7
  opus48:   lUr,  // claude-opus-4-8
  fable5:   MIe,  // claude-fable-5
}
```

### 2.2 firstParty 候选列表 `cUr`（偏移 226465056）

```javascript
cUr = ["opus48","opus47","opus46","opus45"]  // firstParty 默认 opus 优先级
```

### 2.3 第三方默认模型键（偏移 226644607）

```javascript
VY   = "opus46"    // DEFAULT_3P_OPUS_KEY      → claude-opus-4-6
_j   = "sonnet45"  // DEFAULT_3P_SONNET_KEY    → claude-sonnet-4-5-20250929
zY   = "haiku45"   // DEFAULT_3P_HAIKU_KEY     → claude-haiku-4-5-20251001
NPt  = "fable5"    // DEFAULT_3P_FABLE_KEY     → claude-fable-5
Jnt  = "opus47"    // DEFAULT_MANTLE_OPUS_KEY  → claude-opus-4-7（mantle 专用）
```

### 2.4 降级函数清单

| 函数 | 字节偏移 | 代码 | 降级逻辑 |
|------|---------|------|---------|
| **`nzt()`** | 234433976 | `function nzt(){if(fr()==="firstParty")return Vp().opus48;return Vp().opus47}` | **teammate 默认模型**：firstParty → opus48；其他 → opus47 |
| **`YIe()`** | 226628484 | `function YIe(e=Vp()){if(fr()==="mantle")return e[Jnt];if(!td())return e[VY];if(fr()!=="firstParty")return e.opus47;return e.opus48}` | **默认 opus 解析**：mantle→opus47；非 td（非 firstParty/aws/gateway）→ **opus46**；bedrock/vertex/foundry→opus47；firstParty→opus48 |
| **`_An()`** | 226628742 | `function _An(e=Vp()){if(!td())return e[_j];return e.sonnet46}` | **默认 sonnet 解析**：非 td（第三方）→ **sonnet45**；firstParty→sonnet46 |
| **`UPt()`** | 226627300 | `if(t===void 0){let r=Vp();if(t=r.opus48,fr()==="firstParty")t=cUr.map(...).find(xa)??r.opus48}` | **classifier opus reroute**：firstParty 从候选列表选首个可用 opus |
| **`B2r()`** | 226631009 | `if(bo()){...O_()...}else if(td()){...O_()...};...return{setting:jx(),envFamily:"sonnet"}` | **主循环默认模型**：登录 firstParty→opus；td 类→opus；其他→**sonnet** |

### 2.5 降级矩阵（最终效果）

| Provider | 默认 opus | 默认 sonnet | teammate 默认 |
|----------|----------|------------|--------------|
| **firstParty**（官方） | **opus-4-8**（最新） | **sonnet-4-6**（最新） | **opus-4-8** |
| anthropicAws | opus-4-8 | sonnet-4-6 | opus-4-7 |
| gateway | opus-4-8 | sonnet-4-6 | opus-4-7 |
| mantle | opus-4-7 | sonnet-4-5 | opus-4-7 |
| bedrock | opus-4-7 | sonnet-4-5 | opus-4-7 |
| vertex | opus-4-7 | sonnet-4-5 | opus-4-7 |
| foundry | opus-4-7 | sonnet-4-5 | opus-4-7 |
| **第三方（默认）** | **opus-4-6** ⚠️ | **sonnet-4-5** ⚠️ | **opus-4-7** |

> 第三方用户比官方落后 **2 代 opus + 1 代 sonnet**。这是用户感知最强的「降智」。

---

## 3. System Prompt 差异清单

### 3.1 身份 Prompt（`Qkn` 函数，偏移 227751550）

```javascript
function Qkn(e){
  if(fr()==="vertex")return PKr;                          // vertex 用完整 identity
  if(e?.isNonInteractive){
    if(e.hasAppendSystemPrompt)return tNi;                 // SDK + appendPrompt
    return nNi;                                            // SDK 纯净
  }
  return PKr;                                              // 默认
}

PKr = "You are Claude Code, Anthropic's official CLI for Claude."
tNi = "You are Claude Code, Anthropic's official CLI for Claude, running within the Claude Agent SDK."
nNi = "You are a Claude agent, built on Anthropic's Claude Agent SDK."
```

**判定**：base identity **不区分** firstParty vs 第三方。第三方 host 走 `PKr`（标准 identity）。仅 vertex 有独立分支（但返回值相同 `PKr`）。**无降智**。

### 3.2 `appendSystemPrompt` 机制（偏移 227731316）

```javascript
function $1i(e){
  let t=e.cli.systemPrompt,
      n=e.cli.appendSystemPrompt,
      r=lCs();  // 从 managed-settings policyHelper 读 appendSystemPrompt
  if(r) n = n ? `${n}\n${r}` : r;
  return {systemPrompt:t, appendSystemPrompt:n}
}
```

**判定**：`appendSystemPrompt` 来源是 **managed-settings.json 的 policyHelper 输出**（本地配置），不是 CC 内置的 firstParty 分支。第三方用户不会被注入额外的降智 appendSystemPrompt。**无降智**。

### 3.3 Attribution Header（`IAn` 函数，偏移 226646521）

```javascript
function IAn(e,t){
  if(ml(process.env.CLAUDE_CODE_ATTRIBUTION_HEADER))return"";
  let n=`${VERSION}.${e}`,
      r=process.env.CLAUDE_CODE_ENTRYPOINT??"unknown",
      o=fr(),
      s = o==="firstParty"&&_u()||o==="vertex" ? " cch=00000;" : "",  // ⚠️ firstParty 专属标记
      i=wAn(), a=i?` cc_workload=${i};`:"",
      l=ZIe(t)&&!t.isMainSession?" cc_is_subagent=true;":"",
      c=`x-anthropic-billing-header: cc_version=${n}; cc_entrypoint=${r};${s}${a}${l}`;
  return c;
}
```

**判定**：`cch=00000` 是 billing header 标记，firstParty+官方host 或 vertex 才加。这是 **metadata header**，不进入 prompt。影响计费/统计，不影响模型智能。**无降智**。

### 3.4 firstParty 在 prompt 路径的 4 个分支

| 函数 | 偏移 | 行为 | 性质 |
|------|------|------|------|
| `Doi()` | 226618532 | gateway 模型发现（需 `CLAUDE_CODE_ENABLE_GATEWAY_MODEL_DISCOVERY` env） | 功能开关 |
| `Lor()` | 236739542 | prompt slot 分配（UI 相关） | UI 功能 |
| `ppt()` | 238401585 | Datadog 日志 flush | 遥测 |
| `rkc()` | 240955523 | 个性化 agent suggestions（调 gh pr list） | 增值功能 |

**判定**：这 4 个分支全是**功能/遥测**类，没有一个是注入降智 prompt 文本。**无降智**。

---

## 4. 动态降智分支（tengu_* 标志 + 运行时降级）

### 4.1 tengu_* 标志统计

- 总 tengu_ 字符串：1489（多数是遥测事件名）
- 实际 feature flag（`at("tengu_*")` 调用）：**236 个**
- 邻近 firstParty 逻辑的：**38 个**

### 4.2 与 firstParty 相关的 tengu 标志

| 标志 | 作用 | 是否降智 |
|------|------|---------|
| `tengu_walnut_prism` | ownership_frame 行为开关（env `CLAUDE_CODE_OWNERSHIP_FRAME` 可覆盖） | 否（行为偏好） |
| `tengu_cedar_lantern` | act_dont_rederive 行为开关（env `CLAUDE_CODE_ACT_DONT_REDERIVE` 可覆盖，默认 true） | 否（行为偏好） |
| `tengu_saffron_lattice` | overage/credits 配置（planLimitsEndDate 等） | 否（计费） |
| `tengu_saffron_credits_only_tiers` | credits-only tier 列表 | 否（计费） |
| `tengu_saffron_picker_dim` | Fable 模型 picker 禁用（需 credits） | 否（计费 UI） |
| `tengu_fgts` | fine-grained tool streaming（firstParty+官方host 专属） | 功能差异 |
| `tengu_flint_harbor_prompt` | team-onboarding prompt 模板 | 否（功能） |
| `tengu_tool_pear` | strict tool schema（j4e 模型） | 否（功能） |

**判定**：**未发现** `tengu_degrade` / `tengu_limit` / `tengu_thirdparty` 之类的降智标志。所有 tengu 标志要么是行为偏好，要么是计费/功能门控。

### 4.3 `eF()` overage 门控函数（偏移 229578817）

```javascript
function eF(){
  return fr()!=="firstParty"  // 第三方 → true（禁用 overage）
    || !bo()                  // 未登录 → true
    || Eye()                  // enterprise usage-based → true
    || rW()==="default_claude_zero"  // zero-credit tier → true
}
```

`eF()` 为 true 时禁用 overage credits 使用权。**这是计费逻辑**，第三方用户本来就不在 Anthropic 的 credits 体系内，不算降智。

### 4.4 1M 上下文授权（`rU` 函数，偏移 227376432）

```javascript
function rU(e){
  if(Sye())return!1;  // CLAUDE_CODE_DISABLE_1M_CONTEXT
  let t=mo(e);
  if(!VIe(t)?.context?.native_1m 
     && t!=="claude-mythos-5" 
     && t!=="claude-mythos-preview")return!1;
  let n=l_(e);
  return n==="firstParty"&&_u()    // firstParty + 官方 host
      || n==="anthropicAws"
      || n==="mantle"
}
```

**判定**：1M context 仅 firstParty+官方host / anthropicAws / mantle 可用。第三方 host（中转站）**无法用 1M context**，会被降回 200K。这是**真实的能力限制**。可被 `CLAUDE_CODE_DISABLE_1M_CONTEXT` env 影响（但只能禁用，不能强制开启）。

### 4.5 `SAn` entitlementStepDownDefault（偏移 226630715）

```javascript
function SAn(e){
  let t=v9();
  if(t.size===0||!cte(e,t))return null;
  let n=[{family:"opus",model:O_()},{family:"sonnet",model:jx()},{family:"haiku",model:WG()}],
      r=mo(zo(e)), o=n.findIndex(i=>r.includes(i.family)),
      s=o!==-1?o:tH(zo(e))?0:1;
  for(let{model:i}of n.slice(s)) if(xa(i)) return i;
  return null
}
```

**判定**：当当前模型不可用时，按 opus→sonnet→haiku 顺序降级。调用 `O_()`/`jx()`/`WG()`，最终走 `YIe`/`_An`，所以第三方用户的降级链也跟着降级。**间接受影响**。

---

## 5. 能力限制差异

### 5.1 firstParty 专属 beta（偏移 227388194）

```javascript
// $9r 函数：构建 beta 列表
if(cAn && s && QOt(e) && fr()==="firstParty") t.push(cAn);
// cAn = "thinking-token-count-2026-05-13"
```

**判定**：`thinking-token-count` beta 仅 firstParty 推送。影响 thinking token 计数功能。**功能差异**，非 prompt 降智。

### 5.2 firstParty 独享功能

| 功能 | 字符串证据 | 性质 |
|------|-----------|------|
| Files API | "Files API is unavailable on third-party providers (data-residency)" | 数据驻留限制 |
| Channels | "channels are not available on third-party providers" | claude.ai auth 依赖 |
| DesignSync | "DesignSync is only available with claude.ai authentication" | claude.ai auth 依赖 |
| Projects | "Projects is only available with claude.ai authentication" | claude.ai auth 依赖 |
| Ultrareview | "Ultrareview runs in Claude Code on the web and is unavailable on third-party providers" | Web 依赖 |
| Tool Search agent | `Knf()` 返回 false for 非 firstParty | firstParty 专属 |
| Gateway model discovery | `Doi()` 仅 firstParty+非官方host+env 开关 | 功能开关 |

**判定**：这些是**产品功能限制**（依赖 claude.ai 后端服务或数据驻留合规），不是降智。第三方中转站本来就连不上 claude.ai 后端。

### 5.3 上下文窗口限制

```javascript
// 偏移 227379179
YOt=500000, Pte=500000, Evi=20000, Wkd=32000, qkd=128000
```

**判定**：`YOt=500000` 是默认上下文窗口上限（已存在 AIW 的 MaxContextTokens patch 处理）。此值**不区分** firstParty vs 第三方。1M context 是单独的 beta 授权（见 4.4）。

---

## 6. 提示词内容审计

### 6.1 搜索结果

针对以下关键词扫描 strings 输出：

| 关键词 | 命中 | 降智性 |
|--------|------|--------|
| "third-party" + "prompt/system/instruct/restrict/limit/degrade" | 0 | — |
| "降智" / "degrade intelligence" | 0 | — |
| "中转" / "reseller" + 限制指令 | 0 | — |
| "non-official user" + 限制 | 0 | — |

### 6.2 命中的 "third-party" 相关字符串（均非降智）

| 字符串 | 实际含义 |
|--------|---------|
| "Never allow this for third-party repositories" | CLAUDE.md 外部 import 安全规则（偏移 235889409）— **安全提示**，非降智 |
| "forceLoginOrgUUID targets first-party OAuth" | managed-settings 配置警告 — 配置提示 |
| "Files API is unavailable on third-party providers" | 功能不可用提示 |
| "Channels are not available on third-party providers" | 功能不可用提示 |
| "Anthropic auth not used on third-party providers" | 认证状态提示 |
| "third_party_transcripts_dropped" | 遥测事件名 |

### 6.3 身份 Prompt 内容

完整扫描 "You are Claude Code" / "You are an interactive CLI tool" 周边文本，**未发现**任何「如果是第三方用户则降低智能/限制能力」的指令分支。所有 prompt 文本对所有 provider 一致。

**判定**：**CC 不通过 prompt 文本降智第三方用户**。降智完全通过**模型选择**实现（见第 2 节）。

---

## 7. Patch 建议（让第三方享受 firstParty 同等待遇）

### 7.1 已验证可行的 patch（等长替换，JS 合法）

所有 patch 均为**函数级字面量替换**，参照现有 AntiSpy 模式（`function Hsp(){...}` → `function Hsp(){return null;...}`）。

#### Patch A: `nzt()` → 始终返回 opus48（teammate 默认模型升级）

```
search  (76 bytes):  function nzt(){if(fr()==="firstParty")return Vp().opus48;return Vp().opus47}
replace (76 bytes):  function nzt(){return Vp().opus48;/*.....................................*/}
字节偏移: 234433976
```

效果：所有 provider 的 teammate 默认模型从 opus47 升级到 opus48。

#### Patch B: `YIe()` → 始终返回 opus48（默认 opus 解析升级）

```
search  (131 bytes): function YIe(e=Vp()){if(fr()==="mantle")return e[Jnt];if(!td())return e[VY];if(fr()!=="firstParty")return e.opus47;return e.opus48}
replace (131 bytes): function YIe(e=Vp()){return e.opus48;/*.........................................................................................*/}
字节偏移: 226628484
```

效果：所有 provider 的默认 opus 从 opus46/opus47 升级到 opus48。

#### Patch C: `_An()` → 始终返回 sonnet46（默认 sonnet 解析升级）

```
search  (61 bytes):  function _An(e=Vp()){if(!td())return e[_j];return e.sonnet46}
replace (61 bytes):  function _An(e=Vp()){return e.sonnet46;/*.................*/}
字节偏移: 226628742
```

效果：所有 provider 的默认 sonnet 从 sonnet45 升级到 sonnet46。

### 7.2 注意事项

1. **`O_()` / `jx()` 会读取 env**：`O_=if(process.env.ANTHROPIC_DEFAULT_OPUS_MODEL)return iI(...)` —— 如果用户设了 `ANTHROPIC_DEFAULT_OPUS_MODEL`，patch 不生效（env 优先）。这是好事：用户显式指定时不干预。

2. **`B2r()` 不建议 patch**：`B2r` 调用 `bo()`（OAuth 登录态）和 `td()`，逻辑复杂。Patch `YIe`/`_An` 已经覆盖了 `O_()`/`jx()` 的返回值，`B2r` 自然走 `O_()` 路径。Patch `B2r` 风险高收益低。

3. **1M context patch 不可行**：`rU()` 依赖 `l_(e)` 返回 provider，而 `l_` 又依赖 `fr()`。Patch `rU` 让它返回 true 会让 CC 向第三方 host 发 1M context 请求，但第三方 host **可能不支持** `[1m]` beta，导致 API 报错。建议**不 patch**，让用户用 `CLAUDE_CODE_DISABLE_1M_CONTEXT` 自行控制。

4. **`cAn` (thinking-token-count) patch 可选**：
   ```
   search:  if(cAn&&s&&QOt(e)&&fr()==="firstParty")t.push(cAn)
   replace: if(cAn&&s&&QOt(e)&&true             )t.push(cAn)  // 等长？需精确计算
   ```
   此 patch 需精确等长替换 `fr()==="firstParty"`（19 字节）为等价 true 表达式。可用 `(1===1)         `（19 字节，括号+空格填充）但需验证。**低优先级**，仅影响 thinking token 计数 UI。

5. **`UPt()` classifier reroute 不建议 patch**：classifier 用 opus48 会让第三方 host 承担更高成本，且 classifier 质量对模型版本不敏感。

### 7.3 Patch 优先级

| 优先级 | Patch | 收益 | 风险 |
|--------|-------|------|------|
| P0 | Patch B (`YIe` → opus48) | 主对话模型升级 2 代 | 极低（等长+JS 合法） |
| P0 | Patch C (`_An` → sonnet46) | sonnet 升级 1 代 | 极低 |
| P1 | Patch A (`nzt` → opus48) | teammate 升级 1 代 | 极低 |
| P2 | `cAn` beta 推送 | thinking token 计数 | 低（需精确等长） |
| — | 1M context | 不建议 | 高（host 可能不支持） |

### 7.4 集成到 AIW patcher

参照 `src/patcher/registry.rs` 的 `get_antispy_patches()` 模式，新增 `FeatureType::AntiDegrade`：

```rust
pub fn get_antidegrade_patches() -> Vec<UnifiedPatchPattern> {
    vec![
        // Patch A: nzt() -> opus48
        UnifiedPatchPattern {
            feature: FeatureType::AntiDegrade,
            patch_type: PatchType::File,
            search_pattern: Cow::Borrowed(b"function nzt(){if(fr()===\"firstParty\")return Vp().opus48;return Vp().opus47}"),
            replace_pattern: Some(Cow::Owned(b"function nzt(){return Vp().opus48;/*.....................................*/}".to_vec())),
            ...
        },
        // Patch B: YIe() -> opus48
        // Patch C: _An() -> sonnet46
    ]
}
```

每个 patch 生成 File + Memory 两份（参照 AntiSpy 模式）。

---

## 8. 附录：关键函数字节偏移速查

| 函数 | 偏移 | 长度 | 作用 |
|------|------|------|------|
| `fr()` | 226465658 | ~300 | provider 判定（返回 firstParty/bedrock/vertex/...） |
| `td()` | 226466200 | ~80 | `usesFirstPartyModelIds`（firstParty/aws/gateway） |
| `Jl()` | — | — | `isFirstPartyProvider`（仅 firstParty） |
| `NY()` | — | — | `isFirstPartyApiBackend`（firstParty + _u） |
| `_u()` | — | — | `isFirstPartyAnthropicBaseUrl`（_CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL 或 $Sn） |
| `$Sn()` | — | — | `isActualFirstPartyAnthropicBaseUrl`（检查 ANTHROPIC_BASE_URL host） |
| `Ant()` | — | — | `isFirstPartyAnthropicHost`（host == api.anthropic.com） |
| `Vp()` | 226467700 | ~150 | 模型配置 getter（返回 yc 的某一行） |
| `nzt()` | 234433976 | 76 | teammate 默认模型 |
| `YIe()` | 226628484 | 131 | 默认 opus |
| `_An()` | 226628742 | 61 | 默认 sonnet |
| `O_()` | 226628361 | ~80 | getDefaultOpusModel |
| `jx()` | 226628615 | ~80 | getDefaultSonnetModel |
| `B2r()` | 226631009 | 300 | 主循环默认模型解析 |
| `rU()` | 227376432 | 213 | 1M context 授权 |
| `eF()` | 229578817 | ~120 | overage credits 门控 |
| `Qkn()` | 227751550 | ~200 | base identity 选择 |
| `IAn()` | 226646521 | ~600 | attribution header 构建 |
| `$9r` | 227387600 | ~800 | beta 列表构建 |
| `yc` | 226464885 | ~3500 | 模型配置表 |
| `cUr` | 226465056 | ~40 | firstParty opus 候选列表 |

---

## 9. 审计结论

1. **CC v2.1.195 不通过 prompt 文本降智第三方用户**。所有 firstParty 分支在 prompt 路径上只控制功能可用性，不注入降智指令。

2. **真正的降智在模型选择**：第三方用户默认拿到 opus-4-6 / sonnet-4-5，比 firstParty 的 opus-4-8 / sonnet-4-6 落后 1-2 代。这是用户感知最强的降智点。

3. **3 个等长 patch 可完全消除模型降级**：`nzt`/`YIe`/`_An` 的字节级精确匹配已验证，等长 JS 合法替换已设计完成，可直接集成到 AIW patcher。

4. **1M context 降级不建议 patch**：第三方 host 可能不支持 `[1m]` beta，patch 会导致 API 报错。

5. **tengu_* 标志无降智**：236 个 feature flag 中无 `tengu_degrade`/`tengu_limit`/`tengu_thirdparty` 类标志。所有 firstParty 相关 tengu 标志是行为偏好或计费门控。

6. **能力限制（Files API/Channels/Projects 等）是产品限制**：依赖 claude.ai 后端服务或数据驻留合规，非降智。第三方中转站本来无法接入这些后端。

---

## 10. 会话"胡言乱语"控制与采样机制（补充审计）

用户反馈 CC 会话经常"胡言乱语"，深挖 CC 的采样参数和会话控制机制。

### 10.1 采样参数：新模型上 temperature/top_p/top_k 被移除

Fable 5 / Opus 4.8 / 4.7 这些新模型，CC **不再发送 temperature/top_p/top_k**（发了会 400 错误）：

```
Fable 5 / Opus 4.8 / 4.7: temperature/top_p/top_k 被移除，发送会 400
旧模型（Opus 4.6 等）: 可以发 temperature/top_p
```

新模型上无法通过采样参数控制输出随机性，模型用默认采样（可能温度较高），这是"胡言乱语"的来源之一。

### 10.2 真正控制机制：effort level + thinking budget

新模型上，CC 用 `thinkingConfig` + effort level 控制输出质量：

```js
// 默认 thinking 配置
thinkingConfig: Ule() !== false ? {type: "adaptive"} : {type: "disabled"}

// Ule() 判断：
function Ule(){
  if(process.env.MAX_THINKING_TOKENS) 
    return parseInt(process.env.MAX_THINKING_TOKENS,10) > 0;
  // ...
}
```

- `Ule()` 默认 true → `adaptive`（模型自己决定思考多少）
- `MAX_THINKING_TOKENS=0` → `disabled`（不思考，直接输出，容易胡言乱语）

### 10.3 thinking budget 动态计算（关键）

```js
// 主对话 thinking budget
if(f === false){
  thinking = {type: "disabled"};
} else if(f !== void 0){
  thinking = {type: "enabled", budget_tokens: Math.min(f, a-1)};
}
// f = MAX_THINKING_TOKENS 或 effort level 映射的 budget
// a = max_tokens（受 YOt 常量限制）
```

**budget = `Math.min(f, a-1)`**——thinking budget 受 max_tokens 上限约束。

**关键关联**：我们的 max-token patch（`YOt=200000→500000`）间接提升了 thinking budget 上限：
- patch 前：`a=200000`，budget 上限 199999
- patch 后：`a=500000`，budget 上限 499999
- thinking 可以更深入，减少"胡言乱语"

### 10.4 effort level 机制

每个模型有 `default_effort`（模型配置表 yc 里）：
- Opus 4.8: `default_effort: "xhigh"`
- Opus 4.7: `default_effort: "high"`
- Fable 5: `default_effort: "high"`

effort level: `low` / `medium` / `high` / `xhigh` / `max`

通过 `--effort` CLI 参数或 `effortLevel` 设置配置，用户可控。

### 10.5 cedar_lagoon：服务端动态控制 thinking

```js
function k6n(e){
  let t = x0()?.cedar_lagoon;  // statsig 服务端配置
  let n = mo(e);  // model 名
  return Object.entries(t).some(([r,o]) => o === true && n.includes(r));
}
```

`cedar_lagoon` 是 statsig 下发的按 model 的 thinking 开关——Anthropic 服务端可动态控制哪些 model 启用 thinking。如果某 model 被 `cedar_lagoon` 标记 false，thinking 被禁用，输出质量下降。

**这个客户端 patch 不了**（服务端 statsig，客户端只读取）。

### 10.6 各场景采样参数

| 场景 | model | temperature | max_tokens | thinking | 备注 |
|------|-------|-------------|------------|----------|------|
| 主对话 | opus/fable | 不发（新模型） | 8192+ | adaptive（默认）/disabled（用户禁用） | budget 受 max_tokens 限制 |
| classifier | 主对话同 model | 1（高随机） | 256/8192 | fast 无 / thinking 有 | auto-mode 分类 |
| context_tip | haiku | 0（确定性） | 512/128 | disabled | 分类提示 |
| verify_api_key | 任意 | 1 | 1 | disabled | 只验证 |

### 10.7 "胡言乱语"原因汇总

| 原因 | 机制 | 谁控制 | patch 可行？ |
|------|------|--------|-------------|
| thinking 被禁用 | `cedar_lagoon` statsig 按 model 关 thinking | Anthropic 服务端 | ❌ 客户端只读 |
| thinking budget 受限 | `Math.min(f, a-1)`，a=max_tokens | CC 客户端 + 用户 | ✅ max-token patch 已缓解 |
| 用户禁用 thinking | `MAX_THINKING_TOKENS=0` / `CLAUDE_CODE_DISABLE_THINKING` | 用户环境变量 | 无需 patch |
| 新模型无 temperature 控制 | Fable/Opus48 移除采样参数 | API 限制 | ❌ API 层面 |
| adaptive thinking 不稳定 | 模型自己决定思考深度 | 模型行为 | ❌ 模型层面 |
| classifier 高随机误判 | auto-mode `temperature:1` | CC 客户端配置 | ⚠️ 可 patch |
| effort level 被降级 | 服务端 statsig 限制 | Anthropic 服务端 | ❌ 客户端只读 |

### 10.8 PMo=1024（countTokens 预算）

`PMo=1024` 是 countTokens API 请求里的 `thinking.budget_tokens`，用于**预估 token 用量**（不是实际对话的 thinking budget）。3 处调用：

```js
thinking:{type:"enabled", budget_tokens:PMo}  // PMo=1024
```

patch `PMo=1024→8192`（8B 等长）只影响 token 预估精度，不影响实际对话质量。**不建议 patch**（收益低）。

### 10.9 用户侧优化建议（不需要 patch CC）

CC 原生支持的环境变量/CLI 参数：

```bash
# 1. 强制大 thinking budget（最有效）
export MAX_THINKING_TOKENS=32768

# 2. 用最高 effort
claude --effort xhigh
# 或 /effort xhigh（会话内切换）

# 3. 避免 auto-mode classifier 高随机误判
# 不用 bypassPermissions + auto-mode 组合

# 4. 我们的 max-token patch 已提升 budget 上限
aiw patch apply --max-context-tokens 500000
```

### 10.10 patch 总结

**能 patch 的（已做）**：
- `YOt=200000→500000`（max-token patch）：间接提升 thinking budget 上限 ✅

**不建议 patch 的**：
- `PMo=1024→8192`：只影响 countTokens 预估，不改善实际对话
- `Zrl()` 默认 temperature 1→0：classifier 确定性，但 auto-mode 是用户主动开启的，影响面小

**patch 不了的**：
- `cedar_lagoon`（服务端 statsig 动态控制 thinking）
- 新模型移除 temperature/top_p（API 层面限制）
- 服务端模型行为（adaptive thinking 稳定性）

### 10.11 结论

CC 的"胡言乱语"主要是 **thinking 控制机制**导致的：
1. Anthropic 服务端通过 `cedar_lagoon` statsig 动态控制哪些 model 启用 thinking
2. thinking budget 受 `max_tokens` 限制（我们的 max-token patch 已缓解）
3. 新模型无 temperature 控制，输出随机性更高
4. 默认 adaptive thinking，模型自己决定思考深度（可能不足）

**最有效的改善**：用户的 `MAX_THINKING_TOKENS` 环境变量 + `--effort xhigh` + 我们的 max-token patch。这三者组合能最大化 thinking budget，减少胡言乱语。


---

## 第 11 章：CC v2.1.199 深度审计（新功能完整调用链逆向）

**审计时间**：2026-07-03
**审计方法**：对比 198/199 二进制（250MB），追完整调用链（365 次 grep/dd 探针，53 分钟），不停留在表面字符串判断

### 5 个 patch 点命中验证（2.1.199）

| patch 点 | 命中 | 199 字面量样本 |
|---------|------|---------------|
| max-token 常量块 | ✅ | `var tUt=200000,gre=200000,HGd=32000,TGd=128000;`（4 元素，正则 `[^;]*;` 兼容） |
| AntiTelemetry 端点 | ✅ | `/api/event_logging/v2/batch`（1 处） |
| AntiSpy 时区 | ✅ | `Intl.DateTimeFormat().resolvedOptions().timeZone`（2 处） |
| AntiSpy 逃生口 | ✅ | `if(De._CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL)return!0`（199=`De`，语义正则通配） |
| AntiPromptBias | ✅ | `if(qX())n.push("**Provider context:**...`（199=`qX`，语义正则通配） |

### 点1：CCR Turn-Id 体系（isRelayHuman）— 预留框架，当前禁用

**isRelayHuman 本地计算逻辑**（@244575367）：
```js
function Uon(e, t) { return Ckm(e, t) || Ikm(e, t); }
function Ckm(e, t) { return e!==void 0 && Akm.has(e) && t===_xc && vkm(); }
// Akm = {"claude-in-slack","claude_in_slack"}, _xc = "slack_human"
function Ikm(e, t) { return e===Hkm && t===bxc && wkm(); }
// Hkm = "claude-in-teams", bxc = "teams_human"
```

**关键发现：`vkm()` 和 `wkm()` 都硬编码返回 `false`** → `isRelayHuman` 当前**永远为 false** → `IHa()` 直接 return → turn_id 从不提取 → `X-CCR-Turn-Id` header 从不注入。

**X-CCR-Turn-Id 生命周期**（@235389227-235389770）：
- 用 `AsyncLocalStorage` 跨异步上下文传播 turn-id
- `IHa()` 从入站消息 `turn_id` 字段提取（仅当 isRelayHuman=true）
- `KVp()`（@237328241）在 MCP HTTP transport 请求注入 `X-CCR-Turn-Id` header，发给 MCP server（非 Anthropic API）
- `CHa()`/`xHa()` 关联同一 turn 的多个操作

**CCR = Claude Code Relay**，Slack/Teams bridge 集成的 turn 关联框架。

**判定：不是间谍探针**。不检测鼠标/键盘/行为特征，只查消息来源平台标识（Slack/Teams），且当前功能完全禁用（vkm/wkm 硬编码 false）。无需 patch。

### 点2：egress_probe / WFP — Windows 沙箱隔离验证

- `srt-win` = Sandbox Runtime for Windows（`@anthropic-ai/sandbox-runtime`）
- WFP = Windows Filtering Platform，kernel 级防火墙
- `egress_probe` 是 `srt-win wfp verify` 的输出字段，验证沙箱网络隔离是否生效
- 完整逻辑（@234566477）：在 WFP 允许端口范围 `[60080,60089]` 外绑定 listener，让 sandbox 用户尝试连接，若成功（exit=3）说明 fence 未激活
- **只作用于 `srt-sandbox` 专用用户 SID**，不监控正常用户流量
- 三平台等价物：Linux 用 seccomp+socat，macOS 用 sandbox-exec，Windows 用 WFP

**判定：不是间谍**，是沙箱安全隔离验证。v198 已有 srt-win 基础设施，v199 只新增 fence 验证。无需 patch。

### 点3：12 个新 tengu 门控（纠正分类）

**12 个里只有 2 个是 `ot()` feature flag，其余 10 个是 `q()` 遥测事件**（只上报不控制行为）。

| 门控 | 类型 | 条件 | 依赖 fu/mr? | 歧视风险 |
|------|------|------|------------|---------|
| tengu_loop_command | q()事件 | 无 | 否 | 无（/loop 命令统计） |
| tengu_stacked_slash_commands | q()事件 | 无 | 否 | 无（堆叠命令统计） |
| tengu_refusal_fallback_bridge_forwarded | q()事件 | 无 | 否 | 无（refusal 回退桥转发） |
| tengu_refusal_fallback_bridge_timeout | q()事件 | 无 | 否 | 无（bridge 超时） |
| tengu_teleport_repo_host_unverified | q()事件 | 无 | 否 | 无（host 校验失败上报，直接放行） |
| tengu_memory_sync_persistence_warning | q()事件 | 无 | 否 | 无（memory 同步冲突） |
| tengu_agent_view_leader_command_notice | q()事件 | 无 | 否 | 无（agent view 通知） |
| tengu_team_mem_conflict_recovered | q()事件 | 无 | 否 | 无（team memory 冲突恢复） |
| tengu_team_mem_conflict_notice_delivered | q()事件 | 无 | 否 | 无（冲突通知送达） |
| tengu_gzip_request_bodies | ot()flag | 默认false | 间接依赖 wce()(first-party URL) | **隐私保护**（gzip+随机 padding 防流量分析） |
| tengu_omelette_fouet | ot()flag | 默认false | 依赖 ic()(first-party) | 低（DesignSync 需 claude.ai 账号，合理限制） |
| tengu_loggia_denkbild | ot()flag | 默认false | 否 | 无（bridge dialog 能力检测） |

**重点详述**：

**tengu_gzip_request_bodies**（@232869000）：当开启且 URL host 是 api.anthropic.com 时，对请求 body 做 gzip 压缩 + 在末尾添加 0-256 字节随机空白。**这是隐私保护**——防止网络中间设备通过 body 长度做侧信道流量分析。只对 first-party 生效是因为只有 first-party API 支持 gzip。非歧视。

**tengu_refusal_fallback_bridge_***：refusal fallback（模型拒答回退）的 bridge 转发遥测。`hX()`=firstParty&&gu() 只影响"是否自动切换模型"，因为只有 first-party 有多模型权限。bridge_forwarded/timeout 只是遥测，不控制歧视。

**tengu_teleport_repo_host_unverified**：teleport（远程会话恢复）时验证当前 repo 是否匹配 session 要求。`yf()` 检查 host 是否 github.com，`host_unverified` 状态直接放行（`case "host_unverified": break;`），不阻止操作。是安全验证遥测。

### 预存可疑点：x-cc-atis header（非 199 新增）

**`x-cc-atis` header**（@233188626）：值来自 `q0()?.atis`，由服务端 `fetchBootstrapData` 下发，作为 header 发给 first-party API。是服务端下发的追踪 token。

- **非 199 新增**（v198 已有）
- **只发给 first-party API**（中转站用户走 ANTHROPIC_BASE_URL，不发给 api.anthropic.com）
- 若担心被追踪：可清除 `clientDataCacheSlots` 或拦截此 header

### 审计结论

**CC v2.1.199 没有新增间谍探针/门控歧视/中转站识别机制**。

1. **CCR Turn-Id 体系**：Slack/Teams 集成预留框架，`vkm()`/`wkm()` 硬编码 false 导致当前完全禁用。不检测行为特征。
2. **egress_probe / WFP**：Windows 沙箱网络隔离验证，只作用于 srt-sandbox 用户，非间谍。
3. **12 个 tengu 门控**：10 个遥测事件（只上报不控制），2 个 feature flag。无一依赖 fu()/mr() 做中转站歧视。gzip padding 是隐私保护，omelette_fouet 是合理功能限制。
4. **现有 5 个 patch 完全覆盖 199**，无需扩展 patch 框架。

### 持续监控点

- **`vkm()`/`wkm()` 若改为 true**：CCR turn-id 体系会被激活，届时重新评估
- **`x-cc-atis` header**：服务端下发追踪 token（预存，非新增），只发给 first-party
- **gzip 随机 padding**：隐私保护特性，但开启后服务端可识别"用了 gzip 的客户端"
