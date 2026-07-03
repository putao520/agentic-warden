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

## 第 11 章：CC v2.1.199 审计（新间谍点/功能门控排查）

**审计时间**：2026-07-03
**审计方法**：对比 2.1.198 与 2.1.199 二进制（250MB），diff 字符串/statsig 门控/API 端点/指纹字段

### 5 个 patch 点命中验证（2.1.199）

| patch 点 | 命中 | 199 字面量样本 |
|---------|------|---------------|
| max-token 常量块 | ✅ | `var tUt=200000,gre=200000,HGd=32000,TGd=128000;`（4 元素，无 20000，正则 `[^;]*;` 兼容） |
| AntiTelemetry 端点 | ✅ | `/api/event_logging/v2/batch`（1 处） |
| AntiSpy 时区 | ✅ | `Intl.DateTimeFormat().resolvedOptions().timeZone`（2 处） |
| AntiSpy 逃生口 | ✅ | `if(De._CLAUDE_CODE_ASSUME_FIRST_PARTY_BASE_URL)return!0`（配置对象名 199=`De`，198=`Pe`，语义正则通配） |
| AntiPromptBias | ✅ | `if(qX())n.push("**Provider context:**...`（条件函数 199=`qX`，198=`dX`，语义正则通配） |

**跨版本机制全部生效**——5 个 patch 点在 199 全命中，语义正则成功通配 199 的新 minified 变量名（`De`/`qX`）。

### 新增可疑字符串排查

对比 198 vs 199 的可读字符串 diff，命中 2 个含 detect/relay 关键词的新字符串：

**1. `isRelayHuman`（4 处命中）**

完整逻辑（@235389458）：
```js
function IHa(e,{isRelayHuman:t}){
  if(!t)return;                              // isRelayHuman=false 直接返回
  if(typeof e!=="object"||e===null||!("turn_id"in e))return;
  let n=e.turn_id;
  if(typeof n!=="string"||n===""||n.length>KIp||!YIp.test(n))return;  // KIp=128, YIp=/^[\x21-\x7e]+$/
  return n                                   // 返回合法 turn_id
}
```

**判定：非本地间谍点**。
- `isRelayHuman` 是**服务端下发的标记**（relay 服务告诉客户端"这是人类用户"），客户端只是消费
- 用于 CCR（Claude Code Relay）的 `X-CCR-Turn-Id` header turn 追踪（会话续接机制）
- CC 没有在本地判断"用户是不是人类/中转站"，而是接收服务端标记控制 turn_id 校验是否启用
- 不是本地识别探针，无需 patch

**2. `egress_probe`（@234567801）**

上下文：`WFP Egress fence could not be verified — probe to ${o.target} was '${o.egress_probe}'... Re-run 'srt-win install'`

**判定：非间谍功能**。
- WFP = Windows Filtering Platform（Windows 防火墙框架）
- 这是 CC 调用的 Windows 网络防护工具（`srt-win`）的出站探测逻辑
- Windows 专属，Linux/macOS 不生效
- 非客户端识别/上报，无需 patch

### statsig 门控新增（tengu_xxx）

199 新增 12 个 `tengu_*` 实验门控，全部是功能性实验（非间谍/识别）：
- `tengu_agent_view_leader_command_notice`（UI 通知）
- `tengu_gzip_request_bodies`（请求压缩）
- `tengu_loop_command` / `tengu_stacked_slash_commands`（命令功能）
- `tengu_memory_sync_persistence_warning`（memory 同步）
- `tengu_refusal_fallback_bridge_forwarded/timeout`（拒答回退）
- `tengu_teleport_repo_host_unverified`（仓库 teleport）
- 其他（omelette_fouet/loggia_denkbild/team_mem_conflict 等内部代号）

这些是 CC 的功能 A/B 实验，由 `cedar_lagoon` 等 statsig 服务端控制，客户端只读，非间谍。

### 指纹/上报字段对比

| 字段 | 198 命中 | 199 命中 | 变化 |
|------|---------|---------|------|
| machineID | 4 | 4 | 无 |
| deviceId | 57 | 57 | 无 |
| fingerprint | 56 | 56 | 无 |
| sessionId | 779 | 779 | 无 |
| userId | 6 | 6 | 无 |
| custom_base_url | 4 | 4 | 无 |
| ineligible_reason | 2 | 2 | 无 |
| Asia/Shanghai（斜杠值） | 0 | 0 | 198 砍掉后未加回 |

**指纹体系无变化**——199 没有新增指纹字段，也没有恢复被曝光的时区探针。

### 审计结论

**CC v2.1.199 相对 198 没有新的本地间谍点/识别探针/功能门控歧视**。

- 5 个现有 patch 点全命中（语义正则通配 199 的 `De`/`qX` 成功）
- `isRelayHuman` 是服务端标记消费（非本地识别）
- `egress_probe` 是 Windows WFP 防火墙探测（非间谍，Linux 不生效）
- 新增 statsig 门控都是功能性实验
- 指纹/上报字段数量与 198 完全一致
- 被曝光的 `Asia/Shanghai` 时区探针未恢复

**AIW patch 框架无需扩展**，现有 5 个 patch 直接覆盖 199。
