# 手动发布 NPM 包指南

本指南说明如何手动发布 agentic-warden 到 NPM，支持 NPX 启动。

## 📌 前提条件

1. **已配置好的 NPM 环境**
   - Node.js >= 14.0.0
   - NPM >= 6.0.0
   - 已在 NPM 注册账号：https://www.npmjs.com

2. **项目已准备好**
   - 所有测试通过
   - 版本号已更新
   - Git 工作区干净

3. **NPM Token**（可选，用于 CI/CD）
   - 访问 https://www.npmjs.com/settings/tokens
   - 创建 "Publish" 类型的 Token

---

## 🚀 快速发布（推荐）

使用自动发布脚本：

```bash
# 1. 检查环境并预览发布内容（dry-run）
./scripts/publish-npm.sh

# 2. 实际发布
./scripts/publish-npm.sh publish
```

脚本会：
- ✅ 验证环境
- ✅ 同步版本号
- ✅ 构建项目
- ✅ 准备二进制文件
- ✅ 登录 NPM（如有需要）
- ✅ 预览包内容
- ✅ 发布到 NPM

---

## 🛠️ 手动逐步发布

### 步骤 1: 验证环境

```bash
# 检查 Node.js 和 NPM
node --version  # 应 >= 14.0.0
npm --version   # 应 >= 6.0.0

# 检查 Rust
cargo --version

# 检查是否登录 NPM
npm whoami

# 如未登录，执行：
npm login
```

### 步骤 2: 同步版本号

```bash
# 从 Cargo.toml 获取版本
CARGO_VERSION=$(grep '^version' Cargo.toml | sed 's/version = "\(.*\)"/\1/' | tr -d '"')

# 更新 package.json
npm version --allow-same-version --no-git-tag-version "$CARGO_VERSION"

echo "版本已更新为: $CARGO_VERSION"
```

### 步骤 3: 构建项目

```bash
# 构建 release 版本
cargo build --release

# 验证二进制文件存在
ls -lh target/release/agentic-warden*
```

### 步骤 4: 准备二进制文件

```bash
# 检测当前平台
PLATFORM=$(node -e "
const os = require('os');
const platform = os.platform();
const arch = os.arch();
if (platform === 'linux' && arch === 'x64') console.log('x86_64-unknown-linux-gnu');
else if (platform === 'linux' && arch === 'arm64') console.log('aarch64-unknown-linux-gnu');
else if (platform === 'darwin' && arch === 'x64') console.log('x86_64-apple-darwin');
else if (platform === 'darwin' && arch === 'arm64') console.log('aarch64-apple-darwin');
else if (platform === 'win32' && arch === 'x64') console.log('x86_64-pc-windows-msvc');
else console.log('unknown');
")

# 创建目录
mkdir -p "binaries/$PLATFORM"

# 复制二进制文件
cp target/release/agentic-warden* "binaries/$PLATFORM/"
chmod +x "binaries/$PLATFORM/agentic-warden" 2>/dev/null || true

echo "二进制文件已复制到: binaries/$PLATFORM/"
```

### 步骤 5: 验证 NPM 包

```bash
# 预览即将发布的文件
npm pack --dry-run

# 你应该能看到：
# - package.json
# - bin/agentic-warden.js
# - bin/install.js
# - .npmignore
# - binaries/{platform}/agentic-warden
```

### 步骤 6: 发布到 NPM

```bash
# 发布到 NPM（公开包）
npm publish --access public

# 如果看到类似输出，说明成功：
# + agentic-warden@0.4.5
```

### 步骤 7: 验证发布

```bash
# 检查 NPM 页面
open "https://www.npmjs.com/package/agentic-warden"

# 测试安装
npm install -g agentic-warden
agentic-warden --version

# 或者使用 NPX
npx agentic-warden --help
```

---

## 🎯 验证发布成功

发布成功后，用户可以通过以下方式使用：

### 方式 1: NPX（推荐）
```bash
# 无需安装，直接使用
npx agentic-warden --help
npx agentic-warden dashboard
npx agentic-warden claude "Analyze code"

# 指定版本
npx agentic-warden@0.4.5 status
```

### 方式 2: 全局安装
```bash
# 安装一次，永久使用
npm install -g agentic-warden

# 直接使用
agentic-warden --help
agentic-warden status
agentic-warden push
```

### 方式 3: 本地安装
```bash
# 在项目中安装
npm install --save-dev agentic-warden

# 使用 npx
npx agentic-warden --help

# 或添加到 package.json scripts
{
  "scripts": {
    "ai": "agentic-warden"
  }
}

npm run ai -- status
```

---

## 🔧 故障排查

### 问题 1: 版本号不匹配

**症状**：Cargo.toml 和 package.json 版本不同

**解决**：
```bash
# 同步版本号
CARGO_VERSION=$(grep '^version' Cargo.toml | sed 's/version = "\(.*\)"/\1/' | tr -d '"')
npm version --allow-same-version --no-git-tag-version "$CARGO_VERSION"
```

### 问题 2: 未登录 NPM

**症状**：`npm whoami` 返回错误

**解决**：
```bash
npm login
# 输入用户名、密码、邮箱
```

### 问题 3: 包名已被占用

**症状**：`You do not have permission to publish "agentic-warden"`

**解决**：
```bash
# 检查包是否已存在
npm view agentic-warden

# 如果已存在，需要联系所有者或更改包名
# 编辑 package.json 中的 "name" 字段
```

### 问题 4: 二进制文件缺失

**症状**：发布包中没有二进制文件

**解决**：
```bash
# 确保二进制文件存在
ls -lh binaries/*/agentic-warden*

# 如果不存在，重新构建和复制
cargo build --release
PLATFORM=$(node -e "...")
cp target/release/agentic-warden "binaries/$PLATFORM/"
```

### 问题 5: 权限错误

**症状**：`EACCES: permission denied`

**解决**：
```bash
# 修复 npm 权限（推荐）
sudo chown -R $(whoami) $(npm config get prefix)/{lib/node_modules,bin,share}

# 或使用 npx（无需全局安装）
npx agentic-warden
```

---

## 📦 包内容结构

发布后，NPM 包包含以下内容：

```
agentic-warden/
├── package.json              # 包配置
├── .npmignore               # 忽略列表
├── README.md                # 文档
├── bin/
│   ├── agentic-warden.js    # 包装器脚本
│   └── install.js           # 安装脚本
├── binaries/
│   └── x86_64-unknown-linux-gnu/  # 当前平台
│       └── agentic-warden   # 二进制文件
└── ...
```

---

## 🔄 更新版本

### 修改版本号

```bash
# 更新 Cargo.toml
# 手动编辑 version = "0.4.6"

# 同步更新 package.json
npm version --allow-same-version --no-git-tag-version "0.4.6"

# 提交更改
git add Cargo.toml package.json
git commit -m "chore: bump version to 0.4.6"
git tag v0.4.6
git push origin v0.4.6
```

### 重新发布

```bash
# 按照上述 "快速发布" 或 "手动逐步发布" 流程
./scripts/publish-npm.sh publish
```

---

## 📚 相关文档

- [README-NPM.md](./README-NPM.md) - 完整的 NPM 发布指南
- [package.json](./package.json) - NPM 包配置
- [bin/install.js](./bin/install.js) - 安装脚本
- [.npmignore](./.npmignore) - 忽略列表
- [.github/workflows/release.yml](./.github/workflows/release.yml) - CI/CD 配置

---

## ❓ 获取帮助

如需帮助：
- 查看完整文档：[README-NPM.md](./README-NPM.md)
- 检查 GitHub Issues
- 或联系项目维护者
