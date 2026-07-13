# 设计：AIW 接入 Grok Build CLI + Patch 上传功能

> 日期：2026-07-13
> 版本：v1.0（已确认）
> 范围：Grok Build 作为 AIW 管理的 CLI 之一（detect+update）+ patch 掉全部三类数据外泄上传

## 1. 背景与目标

### 1.1 用户意图还原

- **痛点**：Grok Build CLI 在未许可下把用户资料/代码打包上传到 xAI 服务端（git bundle 打包→GCS 上传、App Builder 部署上传、Session Trace 上传），存在数据外泄风险。
- **真实目标**：禁止 Grok 未经许可偷窃资料和代码（断三类上传），且不影响其他正常功能（对话/worktree/mcp/plan/goal/skeptic/memory）、不卡死服务器（频繁打包不上传不能堆积卡死）。
- **技术约束**（用户明确）：**只用 RET / RAX / JMP 控制流 patch**，不用低端字符串 URL 替换。

### 1.2 目标

1. Grok Build 作为 AIW 管理的 CLI（与 Claude/Codex/Gemini 平级），提供 detect + update 能力。
2. patch 掉三类上传：
   - 类1 App Builder 部署上传（`project.tgz` → `app-builder-deployer.grok.com`）
   - 类2 Repo Changes Bundle 上传（`git bundle create` → GCS `gs://`）
   - 类3 Session Trace 上传（`grok trace` → 远端）
3. patch 方式：纯控制流 patch（call 指令等长替换为 `xor eax,eax; mov [rdi],rax`），可静态证明安全。
4. 跨版本稳定：patch 锚点用「tracing 字符串 → slice ptr → text ref → call 字节模式」动态定位，不硬编码地址。

## 2. 领域知识库支撑

逆向事实沉淀于 `docs/domain-knowledge/grok-build.md`，包含：
- Grok Build binary 结构（Rust static-pie stripped ELF，非 Bun JS bundle）
- 下载源（`https://x.ai/cli/grok-{version}-linux-x86_64`，fallback GCS bucket）
- 三类上传的 tracing 字符串 + slice ptr + text 引用点（0.2.93 + 0.2.99 双版本验证）
- L2 patch 点定案（call→`31 c0 48 89 07` 等长替换，静态安全证明 4 步）
- 函数边界检测教训（Rust prologue 入口在 `push %rbp`，非 `push %rbx`）

## 3. 架构：Claude/Grok 平级 patcher

复用 AIW 现有 `src/patcher/` 框架，Claude/Grok 平级下沉到子文件夹，共享层留顶层。顺手正本清源——现有 `registry.rs`/`versions.rs` 名义通用实为 Claude 专有，下沉到 `patcher/claude/`。

```
src/patcher/
├── mod.rs              # 共享层入口：pub mod claude; pub mod grok; 导出共享类型
├── types.rs            # 共享：FeatureType（Claude 7 + Grok 4 变体）、UnifiedPatchPattern
├── file.rs             # 共享：apply_file_patch / is_file_patched / backup / InstallationType
├── runtime.rs          # 共享：内存 patch 引擎
├── error.rs            # 共享：PatchError
├── platform/           # 共享：跨平台内存操作
│
├── claude/             ★ Claude 专有下沉（原 registry.rs/versions.rs 内容迁入）
│   ├── mod.rs
│   ├── registry.rs     # get_antitelemetry_patches 等 7 个 Claude feature
│   ├── versions.rs     # ClaudeVersion + MAX_CONTEXT_TOKENS_SEARCH_REGEX + encode/validate
│   └── install.rs      # get_claude_cli_path / get_claude_js_path / detect_claude_installation
│
└── grok/               ★ 新建 Grok 专有（与 claude/ 平级）
    ├── mod.rs
    ├── registry.rs     # 4 个 Grok feature 的 UnifiedPatchPattern 生成
    ├── versions.rs     # GrokVersion 解析（0.2.99）+ 版本探测
    ├── targets.rs      # 三类上传的锚点定位（tracing 字符串 + slice ptr + text ref + call 字节模式匹配）
    └── install.rs      # detect_grok / get_grok_binary_path（~/.grok/bin/grok + version.json）
```

### 共享层复用（不改）

- `UnifiedPatchPattern`：字面量等长替换机制，search/replace 从「字符串」扩展为「机器码字节序列」。
- `apply_file_patch`：备份→匹配→等长替换→写回，全复用。
- `ensure_binary_backup`：首次 patch 前备份 `grok-linux-x86_64.aiw-backup`。
- `is_file_patched`：检测 patch 是否生效（检测 call 字节是否已被 `31 c0 48 89 07` 覆盖）。
- `restore_from_backup`：回滚。

### Claude 侧迁移（正本清源）

现有 `patcher/registry.rs`（7 个 Claude feature）+ `patcher/versions.rs`（ClaudeVersion + MAX_CONTEXT_TOKENS）迁入 `patcher/claude/`，改 import 路径：
- `src/config.rs`：`patcher::versions::validate_max_context_tokens` → `patcher::claude::versions::...`
- `src/cli_manager.rs`：`patcher::versions::ClaudeVersion` → `patcher::claude::versions::ClaudeVersion`
- `src/supervisor.rs`：`patcher::registry::get_feature_patches` → `patcher::claude::registry::...`
- `src/commands/patch.rs`：7 个 `get_anti*_patches` → `patcher::claude::registry::...`

纯文件移动 + import 路径改写，不改逻辑。符合 CLAUDE.md「重构策略：直接替换，不留向后兼容层」。

## 4. Patch 策略：纯控制流 patch（call→xor+mov 等长替换）

### 4.1 核心方法

定位上传链路的 call 指令，等长 5 字节替换 `e8 xx xx xx xx`（`call rel32`）→ `31 c0 48 89 07`（`xor eax,eax; mov [rdi],rax`），让上传函数返回空结果，调用方走「无结果→跳过」分支。

**为何不 RET 函数**：GCS dispatcher 的调用链根是 session 级混合调度器（同时含 GCS 上传 + goal/plan + skeptic report + memory 系统），RET 无法静态证明不影响 goal/plan/skeptic/memory。patch 2 个 call 精准断上传，不碰混合调度器其他分支。

**为何不纯 nop**：纯 `90*5` 不写 out 参数，调用方读到栈垃圾可能 panic。`31 c0 48 89 07` 主动写 0 → 调用方 `test; je` 跳过 → 安全。

**为何不用字符串 URL 替换**：用户明确要求纯控制流 patch；且 URL patch 不阻断打包（你②的卡死问题），控制流 patch 从调用点断，彻底。

### 4.2 类2 Repo Bundle（已定案，可静态证明安全）

**定位链**（跨版本通用方法）：
1. `readelf -r` 拿重定位表，筛 `Addend = tracing 字符串 vaddr` → 得 slice ptr（`.data.rel.ro`）。
2. `objdump -d -j .text` 反汇编，grep `# <slice_ptr_vaddr>` → 得 `lea` 引用点（upload tracing 打印处）。
3. 回扫找函数入口（`push %rbp` 且前一指令为 ret/ud2/nop），得 GCS dispatcher 入口。
4. grep `call <dispatcher_entry>` → 得 2 个直接 call 调用者。
5. 验证 call 字节 = `e8 rel32`（5字节）+ 前缀 `lea 0x3d0(%rsp),%rdi` + 后缀 `mov 0x3d0(%rsp),%r15; test; je`。

**双版本锚点**（已验证）：

| 版本 | dispatcher 入口 | 2 个 call 调用者 | call 字节 |
|------|----------------|-----------------|-----------|
| 0.2.93 | `0x4ef4b70` | `0x4f04ba2`, `0x4f0a7ee` | `e8 c9 ff fe ff`, `e8 7d a3 fe ff` |
| 0.2.99 | `0x51b9540` | `0x51c5692`, `0x51c9ecb` | `e8 a9 3e ff ff`, `e8 70 f6 fe ff` |

地址跨版本漂移，但**字节模式 + 调用约定 + out 参数偏移 `0x3d0(%rsp)` 跨版本稳定**。

**替换**：`e8 xx xx xx xx`（5 字节）→ `31 c0 48 89 07`（5 字节，`xor eax,eax; mov [rdi],rax`）。

**静态安全证明**（4 步，双版本已验证）：
1. 两 call 都是 5 字节 `e8 rel32`，等长替换 5 字节 `31 c0 48 89 07`。
2. call 前都是 `lea 0x3d0(%rsp),%rdi`（out 参数地址在 rdi），替换后 `mov [rdi],rax` 写 0 到 out 参数 `0x3d0(%rsp)`。
3. call 后都是 `mov 0x3d0(%rsp),%r15; test %r15,%r15; je <skip>`（或 `cmpq $0x0,0x3d0(%rsp); je`），读到 0 → 跳过整个结果消费块（含 drop call + 上传结果处理）。
4. 不碰 dispatcher、不碰根函数 `0x4ed8418`/0.2.99 对应根的其他分支（goal/plan/skeptic/memory 全保留）。

**栈平衡**：call 压栈 8 字节返回地址被 ret 弹；`xor+mov` 不压不弹。栈帧是 `sub $rsp` 固定的，后续 `mov 0x3d0(%rsp),%r15` 偏移基于固定 rsp，不受影响。等长替换不破坏后续偏移。

### 4.3 类1 App Builder + 类3 Session Trace（必须交付，定位方法同类2）

三类上传**全部禁用**是用户明确要求，类1/3 不是可选或 YAGNI 砍掉，只是 patch 点定位放在实现阶段做（方法同类2，已验证可行）。

| 类 | tracing 字符串 | 定位方法 | 状态 |
|----|---------------|---------|------|
| 类1 App Builder | `[deploy_app] starting upload build` | 同类2：tracing→slice ptr→text ref→所在函数的 UploadBuild gRPC call→等长替换 | 实现阶段定位（必须交付） |
| 类3 Session Trace | `upload session trace` / `skip remote upload` | 同类2 | 实现阶段定位（必须交付） |

类1 是 gRPC 流（InitDeployment→UploadBuild→PollDeployment 状态机），比类2 复杂，需定位 UploadBuild 的具体 call。类3 需定位 trace 上传的 HTTP/gRPC call。两者方法论同类2，实现阶段用同样的「tracing→slice→text→call 字节模式」定位。若某类在实现阶段定位不到安全 patch 点，按 §11 退路处理（记录放弃理由，不强行 patch 导致风险）。

## 5. FeatureType 新增（3 个 Grok feature）

```rust
// types.rs FeatureType 新增（与 Claude 7 个平级）
GrokAntiRepoBundle,    // 类2: Repo Changes git bundle 上传（已定案，2 call → 31 c0 48 89 07）
GrokAntiDeployUpload,  // 类1: App Builder 部署上传（实现阶段定位 call）
GrokAntiTraceUpload,   // 类3: Session Trace 上传（实现阶段定位 call）
```

每个 feature 的 `UnifiedPatchPattern`：
- `search_pattern`：call 指令的 5 字节字节序列（`e8 <rel32>`，rel32 动态计算 = dispatcher_entry - call_addr - 5，或用 regex 通配 rel32 字节）。
- `replace_pattern`：`31 c0 48 89 07`（5 字节等长）。
- `patch_type`：File。
- `use_regex`：可选 true，用 `e8[\x00-\xff][\x00-\xff][\x00-\xff][\x00-\xff]` regex 匹配 call 指令（rel32 通配），配合前缀 `lea 0x3d0(%rsp),%rdi`（`48 8d bc 24 d0 03 00 00`）锚定，确保只匹配 upload 链路的 call 不误伤其他 `e8` call。

patch 是否生效的检测（`is_file_patched`）是每个 feature 自带能力（检测 call 字节是否已被 `31 c0 48 89 07` 覆盖），无需单独的 audit feature。

## 6. CLI 集成

### 6.1 CLI 类型

```rust
// cli_type.rs
pub enum CliType { Claude, Codex, Gemini, Grok }  // 新增 Grok
```

### 6.2 安装/更新管理（cli_manager.rs）

Grok 接入现有 `aiw update` 流程，与 Claude 同类（不走 npm，有独立下载机制）：

**CliTool 注册**（`initialize_tools` 加第 4 个工具）：
```rust
CliTool {
    name: "Grok Build".to_string(),
    command: "grok".to_string(),
    npm_package: String::new(),  // Grok 不走 npm，留空
    description: "xAI Grok Build CLI tool".to_string(),
    installed: false, version: None, install_type: None, install_path: None,
},
```

**分流**（`execute_update` 加特判，与 Claude 特判风格一致）：
```rust
// 现有: if tool.command == "claude" { update_claude_cli(tool).await }
// 新增: else if tool.command == "grok" { update_grok_cli(tool).await }
// 其余 npm 分支不变
```
`execute_update` 的 "Supported: claude, codex, gemini" 报错文案更新为含 grok。

**update_grok_cli**（AIW 自己下载，非 shell-out）：
```rust
async fn update_grok_cli(tool: &CliTool) -> (String, bool, String) {
    // 1. 版本检查：调 `grok update --check --json` 得 latestVersion
    //    （或直接 HEAD 探测 x.ai/cli 的版本清单）
    // 2. 已装 + 版本相同 → 跳过；版本不同 → 下载
    // 3. 下载：https://x.ai/cli/grok-{latest}-linux-x86_64
    //    fallback: https://storage.googleapis.com/grok-build-public-artifacts/cli/grok-{latest}-linux-x86_64
    // 4. 放 ~/.grok/downloads/grok-linux-x86_64（覆盖），更新软链 + version.json
    // 5. 未装 → curl https://x.ai/cli/install.sh | sh（与 Claude curl install 对称）
}
```

**关键优势（AIW 自己下载 vs shell-out `grok update`）**：AIW 下载后**立即对 binary 打 patch**（shell-out `grok update` 拿到的是原版 binary，还要 AIW 再补 patch）。`update_grok_cli` 下载完成后自动调用 `apply_grok_patches()`，实现「更新即 patch」。

**统一更新入口**：
- `aiw update` — 全量更新 claude/codex/gemini/grok 四个（Grok 下载后自动 patch）
- `aiw update grok` — 只更新 grok（同样下载后自动 patch）
- `aiw grok update [--version X.Y.Z]` — 也可走 grok 子命令入口（等价 `aiw update grok`）

### 6.3 detect_grok（patch 定位用）

```rust
// patcher/grok/install.rs
pub fn detect_grok() -> Option<GrokInstallation> {
    // 查 ~/.grok/bin/grok 软链 → ~/.grok/downloads/grok-linux-x86_64
    // 读 ~/.grok/version.json 得版本（0.2.99）
    // 返回 binary_path + version，供 patch 定位 + is_file_patched 用
}
```

下载源（知识库记录）：`https://x.ai/cli/grok-{version}-linux-x86_64`，fallback `https://storage.googleapis.com/grok-build-public-artifacts/cli/grok-{version}-linux-x86_64`。

### 6.3 Patch CLI（parser.rs + commands/patch.rs）

`PatchAction` 新增（与 Claude 的 disable-* 平级）：
```rust
GrokDisableRepoBundle,    // 类2 patch
GrokDisableDeployUpload,  // 类1 patch
GrokDisableTraceUpload,   // 类3 patch
GrokStatus,               // grok patch 状态
GrokRestore,              // grok patch 回滚
```

命令：
- `aiw grok patch apply` — 应用全部 Grok 上传 patch
- `aiw grok patch status` — 查 patch 状态（is_file_patched 检测）
- `aiw grok patch restore` — 从备份恢复
- `aiw grok update [--version X.Y.Z]` — 更新 grok binary

## 7. 错误处理

- **patch 失败不阻断**：每个 feature 独立 apply，一个失败不影响其他（复用 Claude patcher 模式，`match apply_file_patch { Ok=>✅, Err=>❌ }`）。
- **备份**：`ensure_binary_backup` 首次 patch 前备份 `grok-linux-x86_64.aiw-backup`，`restore` 可回滚。
- **版本探测**：`GrokVersion::from_string("0.2.99")`，patch 用字节模式匹配不依赖版本（跨版本稳定）。
- **锚点未找到**：若 tracing 字符串/slice ptr/text ref/call 字节模式任一未匹配（版本大改），patch 报 `PatternNotFound`，不崩溃，打印诊断信息（哪个锚点断了）。

## 8. 测试策略

### 8.1 单元测试

- 锚点字面量唯一性（`upload_with_batching: starting` 在 binary 出现 1 次）。
- call 字节模式匹配（`e8 rel32` + 前缀 `lea 0x3d0(%rsp),%rdi` + 后缀 `test; je` 的 regex 命中）。
- 等长约束（`e8 xx xx xx xx` 5 字节 == `31 c0 48 89 07` 5 字节）。
- `GrokVersion::from_string` 解析。
- 替换字节 `31 c0 48 89 07` objdump 反汇编验证 = `xor eax,eax; mov [rdi],rax`。

### 8.2 集成测试

- patch 后 binary 仍可执行：`grok --version` / `grok --help` 不崩溃。
- patch 幂等：重复 apply 不破坏（is_file_patched 检测已 patch 则跳过）。
- restore 回滚：patch 后 restore，binary 恢复原状。

### 8.3 运行时验证（L2 启用前置）

patch 后跑 grok session 实测：
1. **正常功能不崩**：`grok "hello"` 对话、`grok --worktree` 、`grok mcp list`、plan mode、goal 系统、skeptic report、memory 全部正常。
2. **上传阻断**：repo changes 不上传（`~/.grok/upload_queue` 不堆积、抓包无 GCS 流量）、`grok trace <id>` 不远端上传（`--local` 行为）、deploy 不上传。
3. **不卡死**：频繁 session 不堆积、不重试卡死。

验证通过则 patch 正式启用；验证失败（断正常功能）则记录放弃理由，退回仅 patch 已验证安全的点（类2 已静态证明，先启用类2）。

## 9. 跨版本稳定性

### 9.1 已验证版本

| 版本 | 类2 patch 点 | 字节模式 | 验证日期 |
|------|-------------|---------|---------|
| 0.2.93 | `0x4f04ba2`/`0x4f0a7ee` | `e8 rel32` + `lea 0x3d0(%rsp),%rdi` + `test;je` | 2026-07-14 |
| 0.2.99 | `0x51c5692`/`0x51c9ecb` | 同上（地址漂移，模式不变） | 2026-07-14 |

### 9.2 跨版本适配机制

- **不硬编码地址**：patch 锚点用「tracing 字符串 → slice ptr → text ref → call 字节模式」动态定位，新版本只要 tracing 字符串 + 调用约定不变即自动适配。
- **regex 通配 rel32**：call 指令的 rel32 位移跨版本变，用 regex `e8[\x00-\xff]{4}` 通配 + 前缀 `48 8d bc 24 d0 03 00 00`（`lea 0x3d0(%rsp),%rdi`）锚定，确保只匹配 upload 链路的 call。
- **版本探测**：`GrokVersion` 记录版本，patch 失败时打印版本 + 哪个锚点断了，便于新增版本支持。

### 9.3 添加新版本的步骤

1. `aiw grok update` 拉最新 binary（或指定版本）。
2. `aiw grok patch status` 检测锚点是否匹配（tracing 字符串 + call 字节模式）。
3. 匹配则无需改代码（动态定位自动适配）；不匹配则按 `docs/domain-knowledge/grok-build.md` 的定位方法重新逆向，更新 `targets.rs` 的锚点。
4. 运行时验证（§8.3）。

## 10. 范围与 YAGNI

### 本设计交付

- Grok Build 作为 AIW 管理 CLI：detect + update。
- `aiw update` 接入 Grok：全量更新含 grok，`aiw update grok` 支持单工具更新（§6.2）。
- Grok 更新后自动 patch：AIW 自己下载 binary 后立即打上传 patch（更新即 patch）。
- 三类上传 patch（类2 已定案，类1/3 实现阶段同类方法定位）。
- Claude/Grok 平级 patcher 重构（正本清源）。
- 跨版本动态定位机制。

### 明确不做（YAGNI）

- 不做 Grok 进程管理（process_tree 追踪、supervisor）——用户未要求。
- 不做 Grok provider env 注入——用户未要求。
- 不做 Grok 角色系统——用户未要求。
- 不做 Grok TUI 仪表盘集成——用户未要求。
- 不做字符串 URL patch——用户明确要求纯控制流 patch。

## 11. 风险与退路

| 风险 | 缓解 |
|------|------|
| 类1/3 定位失败（gRPC 流复杂）| 退回仅启用类2（已静态证明安全），类1/3 记录待后续 |
| 运行时验证发现断正常功能 | 该 patch 点不启用，记录放弃理由，更新知识库 |
| 新版本锚点漂移 | 动态定位 + regex 通配自动适配；不匹配则报 PatternNotFound 不崩溃 |
| patch 后 grok 崩溃 | restore 从备份回滚；备份机制确保可恢复 |
| Rust enum match 不依赖字符串（已避开）| 本设计不 patch enum 字符串名，只 patch call 指令字节，无此风险 |

## 12. 实现顺序（writing-plans 阶段细化）

1. Claude 侧下沉 `patcher/claude/`（正本清源，纯迁移 + import 改写）。
2. 新建 `patcher/grok/` 骨架（mod/registry/versions/targets/install）。
3. 类2 patch 实现（targets.rs 定位 + registry.rs 生成 pattern + FeatureType 变体）。
4. `cli_type.rs` + `cli_manager.rs` 的 Grok detect + `update_grok_cli`（AIW 自己下载）。
5. `aiw update` 接入 Grok：`initialize_tools` 加 grok CliTool + `execute_update` 加 `else if tool.command == "grok"` 特判 + 下载后自动调用 `apply_grok_patches`。
6. `commands/patch.rs` + `parser.rs` 的 Grok patch CLI（apply/status/restore）。
7. 类2 运行时验证（§8.3）：更新 + patch 后跑 grok session 实测。
8. 类1/3 定位 + 实现 + 验证。
9. 测试 + 文档（CLAUDE.md patch 矩阵 + aiw update 文案更新）。
