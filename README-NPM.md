# NPM 发布指南

本指南说明如何将 agentic-warden 发布到 NPM 注册表，支持通过 NPX 启动。

## 目录结构

```
agentic-warden/
├── package.json              # NPM 包配置
├── .npmignore               # NPM 忽略文件
├── bin/
│   ├── agentic-warden.js    # 主包装器脚本
│   └── install.js           # 安装脚本（下载二进制）
├── binaries/                # 二进制文件（由 CI 生成）
│   ├── x86_64-unknown-linux-gnu/
│   ├── aarch64-unknown-linux-gnu/
│   ├── x86_64-apple-darwin/
│   ├── aarch64-apple-darwin/
│   └── x86_64-pc-windows-msvc/
└── .github/workflows/
    └── release.yml          # 包含 NPM 发布任务
```

## 工作原理

1. **发布流程**（GitHub Actions）：
   - 当创建 tag (v*) 时触发
   - 构建多平台二进制文件
   - 上传到 GitHub Release
   - 发布到 NPM

2. **安装流程**（用户）：
   ```bash
   npm install -g agentic-warden
   # 或
   npx agentic-warden --help
   ```
   - 执行 `postinstall` 脚本
   - 检测平台架构
   - 从 GitHub Release 下载对应二进制
   - 解压到 `binaries/{platform}/`

3. **运行流程**：
   ```bash
   agentic-warden [command]
   ```
   - 调用 `bin/agentic-warden.js`
   - 检测平台并执行对应二进制

## 配置 NPM Token

在 GitHub 仓库设置中添加 NPM_TOKEN：

1. 登录 NPM：
   ```bash
   npm login
   ```

2. 创建 Access Token：
   - 访问 https://www.npmjs.com/settings/tokens
   - 点击 "Create New Token"
   - 类型："Publish"
   - 范围：选择 "Packages: Publish"

3. 添加到 GitHub Secrets：
   - 仓库 Settings -> Secrets and variables -> Actions
   - New repository secret
   - Name: `NPM_TOKEN`
   - Value: 粘贴 NPM token

## 发布新版本

### 自动发布（推荐）

1. 更新版本号：
   ```bash
   # 更新 Cargo.toml
   # 更新 package.json
   npm version patch  # 或 minor / major
   ```

2. 创建并推送 tag：
   ```bash
   git tag v0.4.6
   git push origin v0.4.6
   ```

3. GitHub Actions 自动：
   - 构建所有平台二进制
   - 创建 GitHub Release
   - 发布到 NPM

### 手动发布

```bash
# 1. 构建项目
cargo build --release

# 2. 准备二进制（复制到 binaries/{platform}/）
mkdir -p binaries/x86_64-unknown-linux-gnu
cp target/release/agentic-warden binaries/x86_64-unknown-linux-gnu/

# 3. 发布到 NPM
npm publish --access public
```

## 用户安装指南

### 方式一：使用 NPX（推荐，无需安装）

```bash
# 直接运行（自动下载并执行）
npx agentic-warden --help
npx agentic-warden dashboard
npx agentic-warden push
npx agentic-warden pull

# 指定版本
npx agentic-warden@0.4.5 --help
```

### 方式二：全局安装

```bash
# 安装
npm install -g agentic-warden

# 使用
agentic-warden --help
agentic-warden dashboard
```

### 方式三：本地安装

```bash
# 在项目中安装
npm install --save-dev agentic-warden

# 使用 npx
npx agentic-warden --help

# 或者添加到 package.json scripts
{
  "scripts": {
    "ai": "agentic-warden"
  }
}

# 然后运行
npm run ai -- push
```

## 故障排查

### 问题 1：二进制下载失败

**症状**：`Failed to download: HTTP 404`

**原因**：GitHub Release 不存在或版本不匹配

**解决**：
- 检查版本号是否匹配 `package.json`
- 确保已创建 GitHub Release
- 检查 NPM token 是否有权限

### 问题 2：平台不支持

**症状**：`Unsupported platform: linux arm`

**原因**：用户平台不在支持列表

**解决**：添加新平台支持：
1. 在 `package.json` 的 `os` 和 `cpu` 中添加
2. 在 `release.yml` 中添加构建任务
3. 在 `install.js` 中添加平台检测

### 问题 3：权限错误

**症状**：`EACCES: permission denied`

**解决**：
```bash
# 修复 npm 权限
sudo chown -R $(whoami) $(npm config get prefix)/{lib/node_modules,bin,share}

# 或使用 npx（推荐）
npx agentic-warden
```

## 支持的 Node.js 版本

- Node.js >= 14.0.0
- NPM >= 6.0.0

## 优势

✅ **零安装**：无需手动下载和配置
✅ **多平台**：自动检测并下载对应二进制
✅ **自动更新**：使用 `@latest` 自动获取最新版本
✅ **隔离**：不影响系统其他工具
✅ **便捷**：一条命令即可使用

## 注意事项

⚠️ **首次使用需要下载**：第一次运行时会从 GitHub 下载二进制（约 20-30MB）

⚠️ **网络要求**：需要访问 GitHub Releases 下载二进制

⚠️ **磁盘空间**：会在 `node_modules/agentic-warden/binaries/` 存储平台专用二进制

## 相关文件

- `package.json` - NPM 包配置
- `bin/install.js` - 安装脚本
- `bin/agentic-warden.js` - 主包装器
- `.github/workflows/release.yml` - 发布工作流
- `.npmignore` - 发布时忽略的文件
