# Docker 编译环境指南

**目标**: 在 Docker 容器中编译跨平台 musl 静态二进制，无需在主机安装任何工具

---

## 🎯 为什么使用 Docker 编译？

| 场景 | 传统方式 | Docker 方式 |
|------|---------|-----------|
| Linux 开发者编译 musl | 需要 musl-tools | ✅ 容器包含 |
| macOS 开发者编译 musl | ❌ 不可能 | ✅ 可以（cross） |
| Windows 开发者编译 musl | ❌ 不可能（WSL 复杂） | ✅ 可以 |
| 主机污染 | ❌ 安装工具链 | ✅ 完全隔离 |
| 环境一致性 | ⚠️ 依赖个人配置 | ✅ 100% 一致 |

---

## 🚀 快速开始（3 步）

### 1️⃣ 第一次使用：构建编译镜像

```bash
# 进入项目目录
cd /path/to/agentic-warden

# 构建编译环境镜像（下载 300MB，首次 5-10 分钟）
./build-in-docker.sh build-image
```

**输出示例:**
```
ℹ️  构建编译环境镜像...
Successfully built 7f3c4d8a9b2e
Successfully tagged aiw-builder:latest
✅ 镜像构建完成
```

### 2️⃣ 编译 Linux x86_64 静态二进制

```bash
./build-in-docker.sh x86_64-unknown-linux-musl
```

**输出示例:**
```
ℹ️  编译目标: x86_64-unknown-linux-musl
ℹ️  代码目录: /home/user/agentic-warden

Compiling aiw v6.0.4 ...
    Finished release target(s) in 3m 24s

✅ 编译完成！

📦 二进制路径: target/x86_64-unknown-linux-musl/release/aiw
📊 文件信息: ELF 64-bit LSB executable, x86-64, version 1 (SYSV), statically linked
💾 文件大小: 48M
🔒 状态: 完全静态链接 ✅
```

### 3️⃣ 使用编译的二进制

```bash
# 验证静态链接
file target/x86_64-unknown-linux-musl/release/aiw
# 输出: ... statically linked ...

# 运行
./target/x86_64-unknown-linux-musl/release/aiw --version

# 分发（无需任何依赖）
scp target/x86_64-unknown-linux-musl/release/aiw user@server:/app/
```

---

## 📋 完整命令参考

### 构建镜像

```bash
./build-in-docker.sh build-image
# 首次构建镜像（包含 Rust、musl-tools、编译链）
# 后续使用会缓存
```

### 编译特定目标

```bash
# Linux x86_64 静态二进制
./build-in-docker.sh x86_64-unknown-linux-musl

# Linux ARM64 静态二进制（树莓派等）
./build-in-docker.sh aarch64-unknown-linux-musl

# Linux ARMv7 静态二进制
./build-in-docker.sh armv7-unknown-linux-musleabihf
```

### 编译所有目标

```bash
./build-in-docker.sh all
# 编译所有 musl 目标并生成多个二进制
```

### 进入容器交互式 shell

```bash
./build-in-docker.sh shell
# 进入 Docker 容器的 bash
# 可以手动运行 cargo 命令
# 修改代码后可以在容器中直接重新编译
```

### 清理 Docker 缓存

```bash
./build-in-docker.sh clean
# 删除停止的容器（节省空间）
```

### 显示帮助

```bash
./build-in-docker.sh help
```

---

## 🔧 工作原理

### 容器架构

```
主机（Linux/macOS/Windows）
├── 代码目录（通过 -v 挂载）
│   └── 代码文件
│       └── build-in-docker.sh（构建脚本）
│
└── Docker 容器（aiw-builder）
    ├── Rust 编译器
    ├── musl-tools
    ├── 编译链
    └── /workspace（挂载的代码）
        └── 编译结果写回主机
```

### `-v` 挂载原理

```bash
docker run --rm -v "$(pwd):/workspace" aiw-builder:latest cargo build ...
             ↑                    ↑
        主机代码目录        容器工作目录
```

**作用**:
- 主机的代码目录挂载为容器的 `/workspace`
- 容器内编译修改 `target/` 目录
- 修改会自动同步回主机
- 容器停止后，二进制保留在主机

---

## 💡 高级用法

### 自定义编译命令

```bash
# 直接运行 docker 命令
docker run --rm \
    -v "$(pwd):/workspace" \
    aiw-builder:latest \
    cargo build --release --target x86_64-unknown-linux-musl --verbose

# 运行特定的编译检查
docker run --rm \
    -v "$(pwd):/workspace" \
    aiw-builder:latest \
    cargo clippy --target x86_64-unknown-linux-musl

# 运行测试
docker run --rm \
    -v "$(pwd):/workspace" \
    aiw-builder:latest \
    cargo test --target x86_64-unknown-linux-musl
```

### 编译多个目标到同一个目录

```bash
# 编译所有目标
./build-in-docker.sh all

# 结果结构
target/
├── x86_64-unknown-linux-musl/release/aiw      （~48MB）
├── aarch64-unknown-linux-musl/release/aiw     （~52MB）
└── armv7-unknown-linux-musleabihf/release/aiw （~40MB）
```

### 在容器中修改并重新编译

```bash
# 进入容器
./build-in-docker.sh shell

# 容器内（/workspace 是项目目录）
cd /workspace
vim src/main.rs        # 修改代码

# 重新编译
cargo build --release --target x86_64-unknown-linux-musl

# 退出容器
exit
```

### 使用自定义 Dockerfile

编辑 `Dockerfile.build` 添加额外工具：

```dockerfile
# 添加到 Dockerfile.build
RUN apt-get install -y lldb    # 调试工具
RUN apt-get install -y valgrind # 内存检查

# 重新构建镜像
./build-in-docker.sh build-image
```

---

## 📊 编译时间 & 资源

### 首次编译

```
镜像下载:    5-10 分钟（300MB）
编译时间:    3-5 分钟
磁盘占用:    ~3GB（包含 Rust + 编译缓存）
```

### 后续编译

```
编译时间:    2-3 分钟（使用缓存）
磁盘占用:    增量（仅新文件）
```

### 系统要求

| 资源 | 最小 | 推荐 |
|------|------|------|
| CPU | 2 核 | 4 核 |
| RAM | 2GB | 4GB |
| 磁盘 | 5GB | 10GB |
| Docker | 20.10+ | 最新 |

---

## 🐛 故障排查

### 问题 1: Docker 未找到

```
❌ Docker 未安装
```

**解决**:
```bash
# 检查 Docker
docker --version

# 安装 Docker
# Linux: sudo apt-get install docker.io
# macOS: brew install docker
# Windows: 下载 Docker Desktop
```

### 问题 2: 权限拒绝（Linux）

```
permission denied while trying to connect to the Docker daemon
```

**解决**:
```bash
# 添加用户到 docker 组
sudo usermod -aG docker $USER

# 重新登录或运行
newgrp docker
```

### 问题 3: 容器磁盘满

```
no space left on device
```

**解决**:
```bash
# 清理 Docker
./build-in-docker.sh clean

# 或者
docker system prune -a
```

### 问题 4: 编译速度慢

**可能原因**:
- 首次编译（正常）
- 磁盘 IO 慢（使用 SSD）
- Docker 配置（增加 CPU 分配）

**优化**:
```bash
# 在 Docker Desktop 设置中增加 CPU/RAM
# 或运行调试版本查看详细信息
./build-in-docker.sh x86_64-unknown-linux-musl --verbose
```

---

## 📦 分发编译的二进制

### 生成发布包

```bash
# 编译所有目标
./build-in-docker.sh all

# 创建发布目录
mkdir -p releases
cp target/x86_64-unknown-linux-musl/release/aiw releases/aiw-linux-x86_64
cp target/aarch64-unknown-linux-musl/release/aiw releases/aiw-linux-arm64
cp target/armv7-unknown-linux-musleabihf/release/aiw releases/aiw-linux-armv7

# 计算校验和
sha256sum releases/* > releases/SHA256SUMS

# 上传到 GitHub Release
gh release create v6.0.5 releases/*
```

### 最终产物

```
releases/
├── aiw-linux-x86_64         # 48MB（完全静态）
├── aiw-linux-arm64          # 52MB（完全静态）
├── aiw-linux-armv7          # 40MB（完全静态）
└── SHA256SUMS               # 校验和文件
```

**特点**:
- ✅ 零外部依赖
- ✅ 适用任何 Linux 发行版
- ✅ 无需安装任何库
- ✅ 下载即用

---

## 🎯 CI/CD 集成

### GitHub Actions 示例

```yaml
# .github/workflows/build.yml
name: Docker Build

on: [push]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Build with Docker
        run: |
          chmod +x build-in-docker.sh
          ./build-in-docker.sh all

      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: binaries
          path: target/*/release/aiw
```

---

## ✅ 检查清单

编译前验证：
- [ ] Docker 已安装并运行
- [ ] 项目目录有 `Dockerfile.build`
- [ ] 项目目录有 `build-in-docker.sh`
- [ ] 脚本有执行权限（`chmod +x build-in-docker.sh`）

编译后验证：
- [ ] 二进制生成在 `target/*/release/aiw`
- [ ] 文件大小合理（30-50MB）
- [ ] 验证静态链接：`file target/.../aiw | grep -i static`

---

## 🚀 下一步

1. ✅ 构建镜像：`./build-in-docker.sh build-image`
2. ✅ 编译目标：`./build-in-docker.sh x86_64-unknown-linux-musl`
3. ✅ 验证二进制：`file target/x86_64-unknown-linux-musl/release/aiw`
4. ✅ 分发使用！

---

**完全的跨平台、无依赖、容器隔离的编译流程！** 🐳
