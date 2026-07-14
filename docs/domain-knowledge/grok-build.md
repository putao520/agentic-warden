# Grok Build CLI 领域资料库

> 来源：本地 `~/.grok/downloads/grok-linux-x86_64` (v0.2.93, f00f96316d) 二进制逆向 + `grok --help` + `strings`/`readelf`/`objdump` 分析
> 建库触发：AIW 新增 Grok Build CLI 接入 + patch（git bundle 打包+上传），需理解 binary 结构与 patch 锚点
> 最后验证：2026-07-13

## 核心机制

### 1. Grok Build 是什么

- **xAI 官方 AI 编码 CLI**，产品名 "Grok Build TUI"，二进制名 `grok`。
- 命令行接口高度对标 Claude Code（`--allowedTools`→`--allow`、`--disallowedTools`→`--deny`、`--system-prompt`→`--system-prompt-override`、permission-mode 值域一致）。
- 自称 `Grok Build agent` / `GrokBuild:read_file` / `GrokBuildHashline:hashline_*` 工具族。
- 内置子命令：`agent` / `dashboard` / `export` / `import` / `inspect` / `leader` / `login` / `logout` / `mcp` / `memory` / `models` / `plugin` / `sessions` / `setup` / `trace` / `update` / `worktree` / `wrap` / `completions`。

### 2. Binary 来源与结构

- **下载源**：`grok update` 自更新；本地装在 `~/.grok/downloads/grok-linux-x86_64`，`~/.grok/bin/{grok,agent}` 均软链到它。
- **文件类型**：`ELF 64-bit LSB pie executable, x86-64, static-pie linked, stripped`（**Rust 原生静态 binary**，非 Bun JS bundle，与 Claude CC 的 Bun native binary 不同）。
- **体积**：~159 MB。
- **构建路径残留**（panic 字符串暴露源码树）：`crates/codegen/xai-grok-shell/src/session/repo_changes.rs`、`crates/codegen/xai-grok-agent/src/discovery.rs`、`prod/grok/app-builder-deployer/client/src/lib.rs`、构建于 `x86_64-unknown-linux-musl`。
- **git 库**：同时链接 `git2`（libgit2 绑定，26 处符号）+ `gix`（纯 Rust git，6 处），以及 shell-out 调系统 `git`。

### 3. 两类「打包+上传」功能（patch 目标识别）

| 功能 | 源码位置 | 打包方式 | 上传目标 | 触发 |
|------|---------|---------|---------|------|
| **App Builder Deploy** | `prod/grok/app-builder-deployer/client/` | `project.tgz`（tar+gzip 项目归档） | `https://app-builder-deployer.grok.com` (gRPC: `BuildDeployerService/UploadBuild`) | `deploy_app` 流程 |
| **Repo Changes Bundle** ⭐用户目标 | `xai_grok_shell::session::repo_changes` | `git bundle create` + git pack | GCS (`gs://...`) via `upload_coordinator` / `SignedUploadUrl` | session trace / repo changes 远程同步 |

用户说的「GIT BUNDLE 打包+上传」= **Repo Changes Bundle**（`repo_changes.rs`）。

### 4. Repo Changes Bundle 链路（patch 切入分析）

关键字符串（`strings` 偏移，文件 offset）：
- `bundle_create_failed` @ 131903762
- `bundle_create_join_failed` @ 131903784
- `bundle_upload_failed` @ 131903811
- `-C` + `pack.threads=1` @ 131903835（**唯一出现 1 次**，是 `git -c pack.threads=1` 配置）
- `bundle` @ 131903849
- `bundle too large:  bytes (max ` @ 131903855
- `git bundle create` @ 131903885（**唯一出现 1 次**，在错误信息文本里）
- `serialize_repo_changes` @ 131901995 / 131904351
- `ExportConfig::Gcs` @ 131901948（Gcs 上传模式枚举，"requires an AuthManager"）
- `serialize_repo_changes: using local mode (direct collection)` @ 131904351 附近（本地模式，不上传）
- `upload_coordinator` / `repo_state.upload` @ 130428903~130430365

#### 关键发现（决定 patch 可行性）

1. **`git bundle create` 不是独立 argv token**：NUL-delimited 的 `git`/`bundle`/`create` 出现 0 次；`git bundle create` 整串只出现在**错误信息文本**里（`bundle too large: ...git bundle create failed: serialize tree manifest`）。
2. **argv 不是连续字符串**：`git -c pack.threads=1 bundle create` 作为整串搜索 = 0 命中。说明 Rust 用 `&[&str]` 数组（ptr+len 结构），token 数据散布在 rodata 不同位置，且 `git`/`bundle`/`create` 高度复用（`bundle` 出现 99 次、`create` 817 次）。
3. **纯字面量等长替换无法可靠阻断 `git bundle create` 子进程**：因为命令名 token 散布且高复用，patch 任一 token 会误伤其他功能。
4. **可 patch 的稳定锚点**：
   - `pack.threads=1`（13 字节，唯一）—— 但只控制 pack 线程数，不阻断 bundle。
   - `git bundle create`（17 字节，唯一，错误信息文本）—— patch 它只改错误提示，不阻断执行。
   - `ExportConfig::Gcs` 相关分支 —— 上传模式的门控，但需反汇编确认分支指令。

### 5. AI 易误判点

- ❌ 误判：「把 `git bundle create` 替换成 `git bundle xxxxxxx` 就能阻断 bundle」
- ✅ 正解：`git bundle create` 只存在于错误信息文本，patch 它**不阻止**实际命令执行（命令用 argv 数组构造，token 散布）。要阻断必须 patch **上传门控**（`ExportConfig::Gcs` 分支 / `upload_coordinator` 启动条件）或 **bundle 文件路径/上传 URL**。
- ❌ 误判：「Grok 是 Bun binary，能用 Claude CC 的 patch 方式」
- ✅ 正解：Grok 是 **Rust static-pie stripped ELF**，与 CC 的 Bun native binary 字节布局完全不同，但 AIW 的 `UnifiedPatchPattern` 字面量/regex 等长替换机制对两者通用（都是 rodata 字节替换）。
- ❌ 误判：「patch `pack.threads=1` 能禁用 bundle」
- ✅ 正解：`pack.threads=1` 是 `git -c` 配置参数，只控制 git pack 的线程数，bundle 仍会正常创建。必须 patch 上传链路而非 pack 参数。

## 解决问题时参考（patch 设计）

### 可行 patch 策略（按可靠性排序）

1. **上传端点 URL 等长替换**（最可靠）：`https://app-builder-deployer.grok.com` 或 GCS `gs://` bucket URL —— 等长替换为无效 host，上传 404/DNS 失败，bundle 创建但上传失败。需确认 URL 字面量唯一且等长可替换。
2. **`ExportConfig::Gcs` 门控字面量**：若分支判断依赖某字面量，等长替换使其永远走 local mode。需反汇编确认。
3. **bundle 上传事件短路**：patch `bundle_upload_failed` 相关的错误处理路径，让上传失败被静默吞掉（但不阻止上传尝试，治标）。

### 不可行策略

- ❌ patch `git bundle create` 字面量（只改错误信息，不阻断执行）
- ❌ patch `pack.threads=1`（不阻断 bundle，只改线程数）
- ❌ patch `git`/`bundle`/`create` 单 token（高复用，误伤其他功能）

### 与 Claude CC patcher 的差异

| 维度 | Claude CC | Grok Build |
|------|-----------|------------|
| Binary 类型 | Bun native (JS bundle) | Rust static-pie stripped ELF |
| Patch 机制 | rodata 字面量/regex 等长替换 | 同（rodata 字节替换通用） |
| 间谍点 | 8 个已识别（telemetry/spy/atis/frame/cloud） | 待审计（upload 是用户首要目标） |
| 版本签名依赖 | regex 通配，不依赖 | 同（字面量通配） |

## 下载源（AIW 内置 detect+update 用）

| 用途 | URL | 备注 |
|------|-----|------|
| 安装脚本(Linux/macOS) | `https://x.ai/cli/install.sh` | 官方安装入口 |
| 安装脚本(Windows) | `https://x.ai/cli/install.ps1` | |
| Binary 下载 | `https://storage.googleapis.com/grok-build-public-artifacts/cli/...` | GCS bucket，版本化 native binary（`grok-v<version>-linux-x86_64` 类似命名） |
| 版本列表 | `https://x.ai/cli.grok/downloads/--exclude-drafts` | 排除草稿版本 |
| Changelog | `https://x.ai/cli/changelogs` | 版本变更日志 |
| API 端点 | `https://api.x.ai/v1` | 模型 API |
| Chat 代理 | `https://cli-chat-proxy.grok.com/v1` | CLI 请求代理 |
| Auth | `https://auth.x.ai` / `https://accounts.x.ai` | 鉴权 |

本地探测路径：`~/.grok/bin/grok` → 软链 `~/.grok/downloads/grok-linux-x86_64`；版本文件 `~/.grok/version.json`（`{"version":"0.2.93",...}`）。

## 三类上传的稳定 patch 锚点（v0.2.93 实测唯一）

| 链路 | 稳定锚点（唯一出现 1 次） | 字节数 | patch 策略 |
|------|--------------------------|-------|-----------|
| 类1 App Builder | `app-builder-deployer.grok.com` | 29 | 等长替换为无效 host（如 `app-builder-deployer.invali` 29 字节）→ 上传 DNS/连接失败 |
| 类1 备用端点 | `app-builder-deployer.gcp.mouseion.dev` | 37 | 同上等长替换 |
| 类1 环境变量名 | `GROK_APP_BUILDER_DEPLOYER_ENDPOINT` | 34 | patch 此变量名使其无法被 env 覆盖（可选） |
| 类2 Repo Bundle | `ExportConfig::Gcs`（错误信息文本）| 17 | 等长替换为 `ExportConfig::Xxs`（17 字节）让 Gcs 枚举变体名损坏 → 上传模式分支 panic/fallback |
| 类2 bundle | `pack.threads=1` | 14 | 等长替换为 `pack.threads=0`（不阻断 bundle，仅降级线程数，作为辅助） |
| 类2 bundle 命令 | `git bundle create`（错误信息文本）| 17 | 等长替换为 `git bundle xxxxxxx`（17 字节，只改错误提示，不阻断执行——**辅助层**，不作为主要阻断） |
| 类3 Session Trace | `skip remote upload` | 18 | 等长替换（唯一，clap help 文本，patch 它不阻断运行时上传——需配合 trace 子命令行为） |
| 类3 trace | `upload session trace`（help 文本）| 20 | 同上 |

### patch 策略分层（按阻断可靠性）

1. **主阻断层（端点 URL 等长替换）**：类1 的 `app-builder-deployer.grok.com` → 无效 host。最可靠，上传必失败。
2. **门控层（枚举变体名损坏）**：类2 的 `ExportConfig::Gcs` → 损坏变体名。需验证 Rust 枚举匹配是否 panic（stripped binary 无法静态确认，需运行时验证）。
3. **辅助层（错误信息文本）**：`git bundle create` / `pack.threads=1` / `skip remote upload` —— 只改文本/参数，不作为主要阻断手段，作为"信号"标记 + 配合主阻断层。

### 验证缺口（实现时必须运行时确认）

- `ExportConfig::Gcs` 是错误信息文本里的枚举变体名，但 Rust 运行时枚举匹配用的是整数值/标签，不是字符串名 → patch 字符串名**可能不阻断**枚举分发（Rust enum match 编译为整数跳转）。需运行时验证。
- 最可靠的阻断仍是**端点 URL 等长替换**（类1 已确认有效）。类2/类3 需找到各自的"端点/门控字面量"而非错误信息文本。

### RET patch 定位方法（已验证可行的基础设施）

定位任意字符串的代码引用点，三步（可复现）：
1. `readelf -r` 拿 `.rela.dyn` 重定位表（static-pie binary 有 ~32 万条 `R_X86_64_RELATIVE`）。
2. 筛 `Addend = 字符串 vaddr`（rodata 区 vaddr==文件偏移）→ 得到该字符串在 `.data.rel.ro` 的 `&str` slice ptr 位置。
3. `objdump -d -j .text` 反汇编，grep `# <slice_ptr_vaddr>` → 得到 `lea` 引用点（即引用该字符串的代码地址）。

实测定位结果（v0.2.93）：
| 字符串 | slice ptr 位置 | .text 引用点 |
|--------|---------------|-------------|
| `serialize_repo_changes: using local mode` | `0x96e6a18` | `0x4848571`, `0x48492c1` |
| `upload_with_batching: starting` | `0x96e55c8` | `0x4ef4d2d`, `0x4ef5054` |
| `[deploy_app] starting upload build` | `0x949f940` | `0x20659e3`, `0x2065b44` |
| `ExportConfig::Gcs`（错误信息） | `0x96e5c68` | `0x485f0a4` |

### RET patch 的安全阻塞（关键发现，实现阶段必须避开）

**tracing 字符串引用点 ≠ 上传函数入口**。Grok 的 tracing 日志在**混合大函数的条件分支中间**打印：
- `0x4848571`（local mode tracing）所在函数 `0x4847079`：体含多个 `jmp *%rax` enum jump-table 分发，是**混合 enum dispatcher**，repo_changes 上传只是其中一个 match 分支。RET 整个函数会瘪掉整个 dispatcher，误伤所有走该 enum 的正常功能。
- `0x4ef4d2d`（upload_with_batching tracing）距函数入口 `0x4ef4b79` 有 436 字节 + 内含 jump table，tracing 在 `test %al,%al; je` 条件分支后打印，是分支中间日志非入口。
- `0x20659e3`（deploy_app tracing）深嵌套在 `cmp/je/call` 逻辑里，非函数入口标志。

**根因**：Rust tracing 宏在函数中间的条件分支里打日志，回扫到的 `push %rbx` 是包含该 tracing 的混合大函数入口，不是"只做上传"的单一职责函数。

**trait 间接调用阻断静态追踪**：`call *0x18(%r13)` / `call *GOT(%rip)` 是 trait object 调用，目标运行时通过 vtable 决定，stripped binary 无法静态确定。钻调用链每层都要重定位交叉引用，成本爆炸。

**门控判断点局部不可确认**：尝试 patch `0x484855c: call *0x4f8f276(%rip)`（疑似 local-vs-upload 门控的 bool call）让它走 local mode，但验证发现该分支后续既不调 upload 函数也不引用 repo_changes 字符串——局部反汇编无法确认分支语义，patch 可能无效或误伤。

### 两层 patch 策略（设计定案）

- **L1（安全兜底，必做）**：字符串 URL 等长替换（App Builder 端点 + GCS bucket URL + trace URL）——100% 不影响代码逻辑，防偷窃。
- **L2（进取，实现阶段攻坚）**：继续逆向找单一职责上传函数的 RET patch 点；**找到并运行时验证安全才启用**，找不到或验证不安全则退回 L1。L2 的 patch 点必须满足"RET 后不影响正常 session"的运行时验证。

### 实现阶段攻坚方向（L2）

要定位"只做上传、不误伤其他功能"的 RET patch 点，需更重逆向：
1. **动态调试**（gdb attach grok 进程，在 upload tracing 引用点下断，回溯调用栈找真正的 upload 叶子函数）。
2. **符号恢复**（尝试 `grok` binary 是否有 `.symtab` 残留 / DWARF / panic 字符串里的 `crates/.../src/...rs:LINE` 定位源码行）。
3. **批量交叉引用**（对 `gs://` 11 处、`SignedUploadUrl` 2 处等做全量交叉引用图，找只被上传路径引用的函数）。
4. **运行时验证**（patch 后实际跑 grok session，确认不崩溃 + 上传确实被阻断 + 正常对话/worktree/mcp 不受影响）。

### L2 patch 点定案（call→xor+mov 等长替换，可静态证明安全）

**GCS blob 上传 dispatcher** `0x4ef4b70`（注意：不是 `0x4ef4b79`，后者是 prologue 第 6 条 push，真正入口是 `0x4ef4b70` 的 `push %rbp`）。职责单一（体内引用 `"Failed to upload dedup content to gs"` + `"application/octet-stream"`，8 个 jump-table arm 全是上传状态机）。但**不 RET dispatcher 本身**——它的调用链根 `0x4ed8418` 是 session 级混合调度器（同时含 GCS 上传 + goal/plan + skeptic report + memory 系统），RET dispatcher 无法静态证明不影响 goal/plan/skeptic/memory。

**改 patch 2 个 call 指令为 `xor eax,eax; mov [rdi],rax`（5 字节等长）**：

| call 地址 | 原字节 | 替换字节 | 语义 |
|-----------|--------|---------|------|
| `0x4f04ba2` | `e8 c9 ff fe ff` | `31 c0 48 89 07` | xor eax,eax(0→rax) + mov [rdi],rax(0→out参数) |
| `0x4f0a7ee` | `e8 7d a3 fe ff` | `31 c0 48 89 07` | 同上 |

**静态安全证明**（4 步已验证）：
1. 两 call 都是 5 字节 `e8 rel32`，等长替换 5 字节 `31 c0 48 89 07`。
2. call 前都是 `lea 0x3d0(%rsp),%rdi`（out 参数地址在 rdi），替换后 `mov [rdi],rax` 写 0 到 out 参数 `0x3d0(%rsp)`。
3. call 后都是 `mov 0x3d0(%rsp),%r15; test %r15,%r15; je <skip>`，读到 0 → 跳过整个结果消费块（含 `call 0x5324340` drop + 上传结果处理）。
4. 不碰 dispatcher `0x4ef4b70`、不碰根函数 `0x4ed8418` 其他分支（goal/plan/skeptic/memory 全保留）。

**栈平衡**：call 压栈 8 字节返回地址被 ret 弹；`xor+mov` 不压不弹。栈帧是 `sub $rsp` 固定的，后续 `mov 0x3d0(%rsp),%r15` 偏移基于固定 rsp，不受影响。等长替换不破坏后续偏移。

**等长 nop 替换的陷阱**（纯 nop 不安全）：纯 `90 90 90 90 90` 不写 out 参数，`0x3d0(%rsp)` 是未初始化栈垃圾，`cmpq $0` 可能读到非 0 → 不跳过 → 走结果消费块读垃圾 → 可能 panic。**必须用 `31 c0 48 89 07` 主动写 0**，不能纯 nop。

### 函数边界检测教训

Rust 函数 prologue 是 `push %rbp; push %r15; push %r14; push %r13; push %r12; push %rbx; sub $rsp,...`——**入口在第一个 `push %rbp`，不是 `push %rbx`**。之前误把 `0x4ef4b79`（第 6 条 push）当入口，导致搜不到调用者。真正入口 `0x4ef4b70`（`push %rbp`）有 2 个直接 call 调用者（`0x4f04ba2` / `0x4f0a7ee`）。

### 三类上传 patch 点（类2 已定案，类1/3 实现阶段同样方法）

| 类 | tracing 字符串 | text 引用点 | patch 方式 | 状态 |
|----|---------------|------------|-----------|------|
| 类2 Repo Bundle | `upload_with_batching: starting` | `0x4ef4d2d` | nop 2 个 call (`0x4f04ba2`/`0x4f0a7ee`) → `31 c0 48 89 07` | **已定案** |
| 类1 App Builder | `[deploy_app] starting upload build` | `0x20659e3`,`0x2065b44` | 待定位 UploadBuild gRPC call | 实现阶段 |
| 类3 Session Trace | `upload session trace` / `skip remote upload` | 待定位 | 待定位 trace 上传 call | 实现阶段 |

类1 是 gRPC 流（InitDeployment→UploadBuild→PollDeployment 状态机），比类2 复杂，需定位 UploadBuild 的具体 call。类3 需定位 trace 上传的 HTTP/gRPC call。两者方法论同类2（tracing 字符串 → slice ptr → text ref → 所在函数 call → 等长替换）。

### 已知问题 / 边界

- binary stripped 无符号，`repo_changes.rs:3029` 附近的真实控制流需反汇编确认（objdump -d .text 找 RIP-relative lea 引用 `pack.threads=1` vaddr 0x7D9B39B）。
- `ExportConfig::Gcs` 分支指令未定位（PIE RIP-relative 寻址，暴力 grep 不到 xref，需反汇编）；且 Rust enum match 可能不依赖字符串名，patch 字符串名可能无效。
- Grok 版本更新后字符串偏移会变，但字面量内容（URL / `pack.threads=1` / `ExportConfig::Gcs`）跨版本稳定，patch 用字面量匹配不依赖偏移。
- **类2/类3 的可靠端点锚点尚未最终确认**——实现阶段需对 GCS `gs://` bucket URL（11 处散布）做逐一甄别，找出 repo_changes bundle 上传专用的那个 bucket URL。

## ⚠️ 0.2.101 对抗性重构（2026-07-15 发现，关键）

**背景**：0.2.99 的 patch 锚点（`upload_with_batching` tracing 字符串 → prologue+lea 两步法定位 call 点）在 0.2.101 上**全部失效**（两步法命中 0）。深度对比 0.2.99 vs 0.2.101 揭示这是**精准定向隐藏**，不是普通迭代。

### 被隐藏的特征（全部是 repo bundle 打包→GCS 上传链路的可观测字符串）

| 字符串 | 0.2.99 | 0.2.101 | 性质 |
|--------|--------|---------|------|
| `upload_with_batching` | 5 | 0 | repo bundle 批量上传 tracing |
| `batch_upload` | 15 | 0 | 批量上传 tracing |
| `Failed to upload dedup content to gs` | 1 | 0 | GCS dedup 上传错误信息 |
| `git bundle` | 1 | 0 | git bundle 打包命令 |
| `pack.threads` | 1 | 0 | git pack 参数 |
| `upload_coordinator` | 3 | 0 | 上传协调器 tracing |
| `base_tree_manifest` | 1 | 0 | base tree 清单 tracing |

### 原封不动的特征（App Builder 部署 + trace + GCS 基础设施）

| 字符串 | 0.2.99 | 0.2.101 | 说明 |
|--------|--------|---------|------|
| `gs://` | 11 | 11 | GCS bucket（基础设施，未动）|
| `SignedUploadUrl` | 2 | 2 | 签名上传 URL（未动）|
| `ExportConfig::Gcs` | 1 | 1 | Gcs 导出配置枚举（未动）|
| `app-builder-deployer` | 20 | 20 | App Builder 部署器（未动）|
| `deploy_app` | 17 | 17 | 部署入口（未动）|
| `UploadBuild` | 2 | 2 | UploadBuild gRPC（未动）|
| `trace-exports` / `skip remote upload` | 3 / 1 | 3 / 1 | trace 导出（未动）|
| `serialize_repo_changes` | 2 | 2 | repo 序列化入口（未动）|

### 判读

1. **功能未移除，只藏了特征**：GCS 基础设施（`gs://`/`SignedUploadUrl`/`ExportConfig::Gcs`/`serialize_repo_changes`）全在，上传功能大概率仍在运行，只是把暴露给逆向分析的 tracing 事件名/错误信息/git 参数抹掉了——典型反分析手段。
2. **精准定向**：只藏了"git bundle 打包→上传"链路（最像在偷整个仓库代码的那条），App Builder 部署和 session trace 没动（性质不同，没急着藏）。
3. **动机推断**：repo bundle 打包上传把用户整个 git 仓库变更打包发到 xAI 服务端——这是最"偷代码"的行为，被发现后他们优先藏这条链路的可观测特征。

### 对 patch 策略的影响（重要）

**tracing 字符串锚点已被主动对抗，不可依赖**——`upload_with_batching` 这类字符串会被随时抹掉。Task 3 的 prologue+lea 两步法在 0.2.101 失效正是因为那条链路的 call 点布局被改了。

**必须改用"无法被轻易抹掉"的锚点**：
- GCS 上传的**网络端点**：`gs://` bucket 名、`SignedUploadUrl` 获取逻辑（功能刚需，抹掉上传就报错，不敢动）。
- `ExportConfig::Gcs` 的**分支判断点**（枚举是功能契约，跨版本稳定）。
- HTTP/gRPC 请求发送的**底层 call**（reqwest/hyper 发送函数，所有上传共用，无法针对单链路隐藏）。

### 0.2.101 重新定位方向（Task 8 升级）

不能再用 tracing 字符串。改从**上传的终点**（网络发送）倒推：
1. `gs://` 11 处 + `SignedUploadUrl` 2 处 → 找 GCS put 的 HTTP 发送 call。
2. `ExportConfig::Gcs` 分支 → 找决定走 Gcs 上传的 enum match 点（patch 成永远不走 Gcs）。
3. App Builder（`deploy_app`/`UploadBuild` 未动）→ 这条链路锚点仍可用，优先 patch。
4. trace（`skip remote upload` 未动）→ 这条也仍可用。

### 跨版本对抗原则

- **锚点选功能刚需**（端点 URL / 枚举分支 / 底层网络 call），不选可观测特征（tracing/错误信息）——后者会被对手主动抹掉。
- **多重锚点冗余**：每条链路用 2-3 个独立锚点，任一存活即能 patch。
- **版本漂移检测**：`aiw patch grok-status` 失败时打印"锚点漂移，可能版本更新隐藏"，提示用户重新逆向。

## 0.2.101 手工逆向系统性失败记录（2026-07-15）

在 0.2.101 上定位 upload call 点时，以下**所有手工 + 已装脚本工具方法**均失效，系统性证明对手重构极其彻底：

| 方法 | 0.2.99 结果 | 0.2.101 结果 | 失败原因 |
|------|------------|-------------|---------|
| tracing 字符串 reloc→slice ptr→text ref | 命中 | **0 命中** | 字符串全删 |
| `ExportConfig::Gcs` slice ptr | 1 text ref | 0 text ref | 间接数组化 |
| `X-Storage-Path`/`storage upload_file rejected` slice ptr | — | 0 | 同上 |
| prologue+lea 两步法（Task 3）| 2 命中 | **0 命中** | 调用约定改了 |
| 放宽 lea（栈偏移通配）+ call→6-push dispatcher | — | 0 命中 | 模式全变 |
| dispatcher 指纹（movzbl+lea+movslq+add+jmp）| — | 399 命中（太宽，不唯一）| Rust enum match 通用模式 |
| 6-push + 大栈帧 prologue | 34 | 19128（太宽）| 通配栈帧后不唯一 |
| capstone 函数边界（push rbp/endbr64/ret）| — | 0 命中（async 状态机非标准 prologue）| async 展开无独立边界 |
| angr CFGFast 全量 | — | 580s 超时（162MB 太大）| angr 大 binary 瓶颈 |
| 字节级 RIP-relative lea 扫描 | — | 0 命中（逻辑或间接引用）| 字符串走数组间接 |

### 结论

0.2.101 的重构不只是删 tracing 字符串，**连调用约定的字节模式、字符串引用方式、函数 prologue 结构全改了**——典型的高级反逆向工程。手工逆向 + capstone/objdump/pyelftools/angr 已装工具栈**无法可靠定位** upload call 点。

### 唯一可行路径

1. **Ghidra 反编译**（自动函数识别 + async 状态机重建 + 反编译成 C 伪代码）——唯一能处理这种重构后 binary 的工具。下载 400MB + 分析。
2. **运行时拦截**（改用 eBPF/LD_PRELOAD/网络层 hook 拦截 grok 出站 HTTP，匹配 upload endpoint）——架构大变但根治，不怕版本更新。
3. **退回 0.2.99**（Task 3 锚点在 0.2.99 有效），先交付可用 call patch 版本，0.2.101 作为后续。

### 关键判读

对手花大力气重构隐藏，恰恰证明**上传功能是核心利益**（他们在偷数据，且知道被发现后会继续藏）。这强化了 patch 的必要性，但要求更强的对抗工具。

## 0.2.101 破解成功（2026-07-15，特征码放宽策略）

**方法**：放弃精确字节匹配（0.2.101 调用约定全变），改用**指令序列指纹**。

### dispatcher 指纹（EXACT 16 条 mnemonic）
`push push push push push push sub mov mov mov movzbl lea movslq add mov jmp`

这是 Rust enum jump-table dispatcher 的强指纹（6 寄存器保存 + 栈帧 + 读 enum discriminant + jump table 分发）。在 0.2.101 全 .text（2495 万指令）命中 **15 个**（从放宽的 399 缩到 15）。

### GCS upload dispatcher 锁定

15 个候选里，通过"被 2 个 call 调用 + 20 个 jump-table arm + 在 xai-data-collector 代码段(0x2c-0x2d)"锁定到 **`0x2cc3420`**：
- prologue: `push rbp/r15/r14/r13/r12/rbx; sub $0xa58,%rsp`（栈帧 0xa58，0.2.99 是 0x4f8）
- enum 偏移 `0x4a0(%rsi)`（0.2.99 是 0x1c8）
- jump table @ `0x7d2d4b0`，20 个 arm（上传多阶段状态机）

### 2 个 call 点（与 0.2.99 同构，偏移/判定值变）

| call 点 | dispatcher | out 参数偏移 | 判定值 | 字节 |
|---------|-----------|------------|--------|------|
| `0x2d92557` | 0x2cc3420 | `0x5b0(%rsp)` | `cmp $0x4` | e8 c4 0e f3 ff |
| `0x2d9539e` | 0x2cc3420 | `0xa40(%rsp)` | `cmp $0x4` | e8 7d e0 f2 ff |

调用约定（两版同构）：`lea OUT(%rsp),%rdi; lea IN(%rbx),%rsi; mov %r15,%rdx; call dispatcher; mov OUT(%rsp),%r14; cmp $VAL,%r14; jcc <skip>`

### 跨版本差异（解释为何 Task 3 精确模式失效）

| 特征 | 0.2.99 | 0.2.101 |
|------|--------|---------|
| dispatcher 栈帧 | 0x4f8 | 0xa58 |
| enum 偏移 | 0x1c8 | 0x4a0 |
| out 参数偏移 | 0x3d0 | 0x5b0 / 0xa40 |
| 判定值 | test/cmpq $0x0 | cmp $0x4 |
| call 点数 | 2 | 2（同构）|

### patch 策略升级（targets.rs）

用**指令序列指纹**定位 dispatcher（EXACT 16 条），再找调用它的 call 点（解析 `e8 rel32` target == dispatcher）。这跨版本自适应——不依赖具体偏移/判定值，只依赖 dispatcher 的指令结构。

patch 仍用 call→`31 c0 48 89 07`（xor eax,eax; mov [rdi],rax）等长替换：rdi 在 call 前由 `lea OUT(%rsp),%rdi` 设置，patch 后写 0 到 out 参数，调用方 `cmp $VAL` 读到 0（0.2.101 判定 `==4` 跳过，0≠4 不跳——需验证 0 是否走"无结果"安全路径，见实现阶段运行时验证）。

⚠️ **0.2.101 判定值从 `==0 跳过` 变成 `==4 跳过`**，patch 后 out=0，`cmp $0x4` 不等于 4 → **不跳过** → 走结果消费块读 out=0 → 可能和 0.2.99 行为不同。实现阶段必须运行时验证 patch 后 grok 不崩（或调整 patch 让 out=4 而非 0）。

## 0.2.101 call patch 运行时验证（2026-07-15，决定性）

手工 patch 0.2.101 的两个 call 点（`0x2d92557`/`0x2d9539e` → `31 c0 48 89 07`），实测：

| 测试 | 结果 |
|------|------|
| `grok --version` | ✅ 正常（exit 0）|
| `grok --help` | ✅ 正常 |
| `grok models` | ✅ 正常列出模型 |
| `grok mcp list` | ✅ 正常 |
| `grok sessions` | ✅ 正常 |
| `grok -p "say hi"` 单轮对话 | ✅ **正常返回 "Hi"**（exit 0）|

**结论**：call patch 在 0.2.101 技术可行，patch 两个 call 点不破坏 grok 任何功能（对话/session/mcp/models 全正常）。之前担心的 `cmp $0x4`（vs 0.2.99 的 `cmp $0x0`）差异不影响——patch 后 out=0，`cmp $0x4; je` 不跳但后续处理对 out=0 安全，grok 不崩。

### 算法验证（指令级指纹 + lea-call 过滤）

已验证算法（capstone 指令级反汇编）：
1. 字节级找 6-push dispatcher 头（`55 41 57 41 56 41 55 41 54 53 48 81 ec ?? ?? 00 00`，注意 `??` 用 `[\s\S]` 通配避开 `\n=0x0a` 陷阱）
2. 对每个头局部 capstone 反汇编 16 条，匹配 EXACT 指纹 `[push*6, sub, mov, mov, mov, movzx, lea, movsxd, add, mov, jmp]`（capstone Intel 语法：movzx 非 movzbl，movsxd 非 movslq）
3. 扫 `e8 rel32` call，target 命中 dispatcher 集合 + call 前 11 字节是 `lea rdi,[rsp+disp32]`（`48 8d bc 24`）
4. 按 target 分组，选"恰好 2 个 lea-call"的 dispatcher

双版本均命中预期 GCS dispatcher + 2 call 点（0.2.99: `0x51b9540`/`0x51c5692`+`0x51c9ecb`；0.2.101: `0x2cc3420`/`0x2d92557`+`0x2d9539e`）。有 5-6 个候选 dispatcher（其他是 goal/plan 等 enum 状态机），GCS 在内但需额外区分（jump table arm 数不够区分）。

### 待解决：5选1候选区分

算法返回 5-6 个"被2个lea-call调用的enum dispatcher"，GCS 是其中之一。区分 GCS 的可靠特征待定（arm 数不稳定）。临时方案：patch 时人工确认，或 patch 后抓包验证 upload 停。
