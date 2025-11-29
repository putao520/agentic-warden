# NPM包发布指南

## 📋 一次性配置（第一次发布时）

### 步骤1：在NPM官网创建Package

1. 访问 [npmjs.com](https://www.npmjs.com)
2. 登录你的NPM账户（或注册新账户）
3. 在个人信息中确保：
   - 账户已验证邮箱
   - 已启用2FA（可选但推荐）

### 步骤2：生成NPM Access Token

**方式A：自动生成token（推荐）**

1. 访问 https://www.npmjs.com/settings/~/tokens
2. 点击 "Generate New Token"
3. 选择 **"Granular Access Token"**（粒度token）
4. 配置权限：
   - **Permissions**: `Read and write` to packages and package metadata
   - **Expiration**: 无限期 (Unlimited) 或设置足够长的期限
   - **Package access**: 选择 `Only selected packages`
   - **Packages**: 搜索并选择 `aiw` 包
5. 复制生成的token（只会显示一次）

**方式B：传统token（如果Granular不可用）**

1. 访问 https://www.npmjs.com/settings/~/tokens/create
2. 选择 "Automation"
3. 复制token

### 步骤3：配置GitHub Secrets

1. 进入GitHub仓库 → **Settings** → **Secrets and variables** → **Actions**
2. 点击 "New repository secret"
3. 创建新secret：
   - **Name**: `NPM_TOKEN`
   - **Value**: 粘贴刚才复制的NPM token
4. 点击 "Add secret"

**验证配置：**
```bash
# 本地测试（可选）
echo "//registry.npmjs.org/:_authToken=${NPM_TOKEN}" > ~/.npmrc
npm whoami  # 应该显示你的NPM用户名
```

---

## 🚀 每次发布流程

### 步骤1：本地检查

```bash
# 确保在master分支上
git checkout master
git pull origin master

# 检查版本（应该与要发布的版本一致）
cat SPEC/VERSION
cat npm-package/package.json | grep version

# 运行测试确保代码质量
cargo test --lib
```

### 步骤2：创建Git Tag

```bash
# 设置版本号（示例：v6.0.5）
VERSION="v6.0.5"

# 创建带注释的tag
git tag -a $VERSION -m "Release version ${VERSION#v}"

# 验证tag
git tag -l -n1 | grep $VERSION
```

### 步骤3：推送Tag到GitHub

```bash
# 推送tag（触发GitHub Actions）
git push origin $VERSION

# 或推送所有tags
git push origin --tags
```

### 步骤4：监控发布流程

1. 进入GitHub仓库 → **Actions**
2. 找到 "Release to NPM & GitHub" workflow
3. 等待执行完成（约10-15分钟）

**执行流程：**
```
build-binaries (3个并行) ──┐
                           ├──> publish-npm ✅
                           │
                           └──> publish-github-release ✅
```

**监控每个Job：**
- ✅ `build-binaries`: 编译3个平台
- ✅ `publish-npm`: 发布到NPM（查看输出验证）
- ✅ `publish-github-release`: 发布二进制到GitHub Release

### 步骤5：验证发布

**验证NPM包：**
```bash
# 在线验证
npm view aiw@<version>

# 例如：
npm view aiw@6.0.5

# 本地安装测试
npm install -g aiw@<version>
aiw --version
```

**验证GitHub Release：**
1. 进入GitHub仓库 → **Releases**
2. 确认新的release包含：
   - `aiw-linux-x86_64`
   - `aiw-linux-arm64`
   - `aiw-linux-armv7`
   - `SHA256SUMS`

---

## 📦 发布后产物

### NPM Package
```bash
npm install aiw@6.0.5
aiw --version

# 在node_modules中的结构
node_modules/aiw/
├── bin/
│   ├── aiw-linux-x64      # x86_64架构
│   ├── aiw-linux-arm64    # ARM64架构
│   └── aiw-linux-armv7    # ARMv7架构
├── index.js               # 多架构启动脚本
└── package.json
```

### GitHub Release
- 二进制下载链接：`https://github.com/putao520/agentic-warden/releases/tag/v6.0.5`
- 包含所有平台的预编译二进制和校验和

---

## 🔧 故障排除

### 问题1：NPM发布失败 - "403 Forbidden"

**原因：**
- NPM_TOKEN已过期或权限不足
- Package名称已被占用
- 版本号已发布过

**解决：**
```bash
# 1. 重新生成NPM_TOKEN
# 访问 https://www.npmjs.com/settings/~/tokens

# 2. 更新GitHub Secrets
# GitHub Settings → Secrets → 更新NPM_TOKEN

# 3. 确认包名未占用
npm view aiw@<version>  # 应返回404

# 4. 确认版本号未发布
npm view aiw versions | grep <version>
```

### 问题2：Docker编译失败

**症状：** Job `build-binaries` 失败

**解决：**
```bash
# 本地测试编译
./build-in-docker.sh build-image
./build-in-docker.sh x86_64-unknown-linux-musl

# 查看详细错误日志
# GitHub Actions → 查看failed job的完整输出
```

### 问题3：NPM_TOKEN权限不足

**症状：** "You do not have permission to publish this package"

**解决：**
1. 确认NPM账户是aiw package的owner
2. 重新生成Granular Access Token，确保权限包括：
   - `Publish` permission
   - `Read` permission
3. 确保token仅限特定package `aiw`

---

## 🔐 安全建议

### Token管理
- ✅ 使用Granular Access Token（粒度token）
- ✅ 限制token权限只到`aiw`包
- ✅ 设置合理的过期时间（建议1年）
- ✅ 定期轮换token（每年）
- ❌ 不要在代码中存储token
- ❌ 不要在GitHub公开显示token

### 版本管理
- ✅ 遵循语义化版本（Semantic Versioning）
- ✅ 保持SPEC/VERSION和package.json同步
- ✅ 每个版本创建Git Tag
- ✅ 在CHANGELOG中记录变更

### 验证发布
- ✅ 总是验证npm install后的功能
- ✅ 验证二进制校验和
- ✅ 确认所有平台的二进制都正确

---

## 📊 快速参考

| 操作 | 命令 |
|------|------|
| 查看当前NPM包版本 | `npm view aiw` |
| 查看所有版本 | `npm view aiw versions` |
| 安装特定版本 | `npm install aiw@6.0.5` |
| 检查token有效性 | `npm whoami` |
| 本地测试发布 | `npm publish --dry-run` |
| 撤销发布（24小时内） | `npm unpublish aiw@6.0.5` |

---

## 🎯 完整发布清单

```
发布前检查：
- [ ] SPEC/VERSION已更新
- [ ] npm-package/package.json版本一致
- [ ] CHANGELOG已更新
- [ ] 所有测试通过（cargo test）
- [ ] 代码已提交到master分支

发布执行：
- [ ] 创建Git Tag
- [ ] 推送Tag到GitHub
- [ ] 监控GitHub Actions执行
- [ ] 验证NPM包可安装
- [ ] 验证GitHub Release包含所有文件
- [ ] 验证二进制校验和正确

发布后验证：
- [ ] npm install -g aiw@<version> 成功
- [ ] aiw --version 工作正常
- [ ] GitHub Release页面正确
- [ ] 所有三个平台的二进制都可下载
```

---

**需要帮助？** 检查GitHub Actions日志或联系维护者 @putao520
