# 脚本使用说明

## 版本更新脚本 (update-version.sh)

### 功能
自动化同步项目版本号，确保所有文件保持一致。

### 更新的文件
- `VERSION` - 项目主版本文件
- `Cargo.toml` - Rust 项目版本
- `npm-package/package.json` - 由 GitHub Actions 自动更新

### 使用方法

```bash
# 基本用法
./scripts/update-version.sh <version>

# 示例：更新到版本 0.5.20
./scripts/update-version.sh 0.5.20
```

### 版本格式
必须使用 `major.minor.patch` 格式：
- ✅ `0.5.20`
- ✅ `1.0.0`
- ❌ `v0.5.20` (不要加 v)
- ❌ `0.5` (不完整)

### 完整工作流程

```bash
# 1. 运行脚本更新版本
./scripts/update-version.sh 0.5.20

# 2. 检查变更
git diff

# 3. 提交变更
git add .
git commit -m "feat: 版本更新到 0.5.20"

# 4. 创建并推送 tag
git tag v0.5.20
git push origin master
git push origin v0.5.20

# 5. GitHub Actions 将自动:
#    - 构建 Linux 和 Windows 二进制文件
#    - 更新 npm-package/package.json 版本
#    - 发布到 NPM
#    - 创建 GitHub Release
```

### 注意事项

1. **npm-package/package.json 由 CI/CD 自动更新**，无需手动修改
2. **版本号必须与 Git tag 保持一致**
3. **脚本会验证版本格式，不符合会报错**
4. **每次更新前会检查文件是否存在，避免误操作**