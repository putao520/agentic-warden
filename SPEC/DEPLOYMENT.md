# Agentic-Warden 部署说明

## 部署概览

### 1. 部署架构
```
┌─────────────────────────────────────────┐
│              用户环境                   │
│  ┌─────────────┐  ┌─────────────────┐  │
│  │   CLI Tool  │  │     TUI App     │  │
│  │             │  │                 │  │
│  │ agentic-    │  │ agentic-warden  │  │
│  │ warden cli  │  │ (dashboard)     │  │
│  └─────────────┘  └─────────────────┘  │
└─────────────────────────────────────────┘
         │                    │
         ▼                    ▼
┌─────────────────────────────────────────┐
│          本地配置和服务                  │
│  ┌─────────────┐  ┌─────────────────┐  │
│  │Provider JSON│  │   Auth Files    │  │
│  │             │  │                 │  │
│  │~/.agentic-  │  │~/.agentic-      │  │
│  │warden/      │  │warden/auth.json │  │
│  └─────────────┘  └─────────────────┘  │
│  ┌─────────────┐  ┌─────────────────┐  │
│  │Shared Memory│  │  Log Files      │  │
│  │             │  │                 │  │
│  │/dev/shm/    │  │~/.agentic-      │  │
│  │agentic-     │  │warden/          │  │
│  │warden-*     │  │warden.log       │  │
│  └─────────────┘  └─────────────────┘  │
└─────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────┐
│            外部服务                     │
│  ┌─────────────┐  ┌─────────────────┐  │
│  │Google Drive │  │  AI Providers   │  │
│  │             │  │                 │  │
│  │API + OAuth  │  │OpenRouter/LiteLLM│ │
│  └─────────────┘  └─────────────────┘  │
└─────────────────────────────────────────┘
```

### 2. 部署模式

#### 2.1 单机部署（推荐）
- **适用场景**: 个人开发者、小团队
- **安装方式**: Cargo 安装或二进制下载
- **配置管理**: 本地配置文件
- **数据存储**: 本地文件系统

#### 2.2 容器化部署
- **适用场景**: CI/CD 环境、标准化部署
- **安装方式**: Docker 容器
- **配置管理**: 环境变量 + 配置挂载
- **数据存储**: 卷挂载

#### 2.3 系统服务部署
- **适用场景**: 服务器环境、后台运行
- **安装方式**: 系统包管理器
- **配置管理**: 系统配置文件
- **数据存储**: 系统目录

## 单机部署

### 1. 系统要求

#### 1.1 操作系统支持
- **Linux**: Ubuntu 20.04+, Debian 11+, CentOS 8+, RHEL 9+
- **macOS**: 11.0+ (Big Sur+)
- **Windows**: 10+ (Build 19041+)

#### 1.2 硬件要求
- **CPU**: 双核心 2.0GHz+
- **内存**: 最小 512MB，推荐 2GB+
- **存储**: 最小 100MB 可用空间
- **网络**: 互联网连接（用于 Google Drive 和 AI Provider）

#### 1.3 软件依赖
- **Rust**: 1.70+ （仅开发时需要）
- **Git**: 用于 AI CLI 工具管理
- **浏览器**: Google OAuth 授权需要

### 2. 安装方式

#### 2.1 预编译二进制安装（推荐）
```bash
# Linux (x86_64)
curl -L https://github.com/your-org/agentic-warden/releases/latest/download/agentic-warden-linux-x86_64.tar.gz | tar xz
sudo mv agentic-warden /usr/local/bin/

# macOS (Apple Silicon)
curl -L https://github.com/your-org/agentic-warden/releases/latest/download/agentic-warden-macos-aarch64.tar.gz | tar xz
sudo mv agentic-warden /usr/local/bin/

# Windows (x86_64)
# 从 GitHub Releases 下载 agentic-warden-windows-x86_64.zip
# 解压并添加到 PATH
```

#### 2.2 Cargo 安装
```bash
# 从 crates.io 安装
cargo install agentic-warden

# 从源码安装最新版
git clone https://github.com/your-org/agentic-warden.git
cd agentic-warden
cargo install --path .
```

#### 2.3 包管理器安装
```bash
# Homebrew (macOS)
brew tap your-org/agentic-warden
brew install agentic-warden

# Debian/Ubuntu
wget -qO- https://your-org.github.io/agentic-warden/apt/gpg.key | sudo apt-key add -
echo "deb https://your-org.github.io/agentic-warden/apt stable main" | sudo tee /etc/apt/sources.list.d/agentic-warden.list
sudo apt update
sudo apt install agentic-warden

# RPM (Fedora/CentOS)
sudo dnf install https://github.com/your-org/agentic-warden/releases/latest/download/agentic-warden-x86_64.rpm
```

### 3. 初始配置

#### 3.1 创建配置目录
```bash
# 配置目录会自动创建，但也可以手动创建
mkdir -p ~/.agentic-warden
mkdir -p ~/.agentic-warden/temp
mkdir -p ~/.agentic-warden/schema
```

#### 3.2 验证安装
```bash
# 检查版本
agentic-warden --version

# 检查帮助
agentic-warden --help

# 启动 Dashboard
agentic-warden
```

#### 3.3 基本配置
```bash
# 设置默认 Provider（可选）
agentic-warden provider set-default official

# 添加 OpenRouter Provider（示例）
agentic-warden provider add openrouter \
  --description "OpenRouter 统一 LLM 网关" \
  --compatible-with codex,claude,gemini \
  --env OPENAI_API_KEY="your-openrouter-key" \
  --env OPENAI_BASE_URL="https://openrouter.ai/api/v1"
```

### 4. AI CLI 工具配置

#### 4.1 安装支持的 AI CLI
```bash
# 安装 Claude CLI
npm install -g @anthropic-ai/claude-cli

# 安装 Codex CLI（示例）
pip install codex-cli

# 安装 Gemini CLI（示例）
cargo install gemini-cli
```

#### 4.2 验证 AI CLI 可用性
```bash
# 检查可用的 AI CLI
agentic-warden status

# 应该显示已安装的 AI CLI 工具
```

## 容器化部署

### 1. Docker 镜像

#### 1.1 官方镜像
```bash
# 拉取最新镜像
docker pull ghcr.io/your-org/agentic-warden:latest

# 拉取特定版本
docker pull ghcr.io/your-org/agentic-warden:v0.1.0
```

#### 1.2 从源码构建
```bash
git clone https://github.com/your-org/agentic-warden.git
cd agentic-warden
docker build -t agentic-warden .
```

#### 1.3 Dockerfile
```dockerfile
# Dockerfile
FROM rust:1.75-alpine AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM alpine:3.18

RUN apk add --no-cache \
    ca-certificates \
    git \
    curl \
    bash

COPY --from=builder /app/target/release/agentic-warden /usr/local/bin/

ENV AGENTIC_WARDEN_CONFIG_DIR=/config
VOLUME ["/config"]

EXPOSE 8080

ENTRYPOINT ["agentic-warden"]
CMD ["--help"]
```

### 2. 容器运行

#### 2.1 基本运行
```bash
# 运行 Dashboard
docker run -it --rm \
  -v ~/.agentic-warden:/config \
  -e AGENTIC_WARDEN_CONFIG_DIR=/config \
  ghcr.io/your-org/agentic-warden:latest \
  dashboard
```

#### 2.2 后台服务
```bash
# 作为后台服务运行
docker run -d \
  --name agentic-warden \
  -v ~/.agentic-warden:/config \
  -e AGENTIC_WARDEN_CONFIG_DIR=/config \
  --restart unless-stopped \
  ghcr.io/your-org/agentic-warden:latest \
  status --daemon
```

#### 2.3 Docker Compose
```yaml
# docker-compose.yml
version: '3.8'

services:
  agentic-warden:
    image: ghcr.io/your-org/agentic-warden:latest
    container_name: agentic-warden
    restart: unless-stopped
    volumes:
      - ~/.agentic-warden:/config
      - /tmp:/tmp
    environment:
      - AGENTIC_WARDEN_CONFIG_DIR=/config
      - RUST_LOG=info
    ports:
      - "8080:8080"
    command: ["dashboard"]
```

### 3. Kubernetes 部署

#### 3.1 ConfigMap
```yaml
# k8s/configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: agentic-warden-config
data:
  provider.json: |
    {
      "providers": {
        "official": {
          "description": "Official API",
          "compatible_with": ["codex", "claude", "gemini"],
          "env": {}
        }
      },
      "default_provider": "official"
    }
```

#### 3.2 Deployment
```yaml
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: agentic-warden
spec:
  replicas: 1
  selector:
    matchLabels:
      app: agentic-warden
  template:
    metadata:
      labels:
        app: agentic-warden
    spec:
      containers:
      - name: agentic-warden
        image: ghcr.io/your-org/agentic-warden:latest
        ports:
        - containerPort: 8080
        volumeMounts:
        - name: config
          mountPath: /config
        - name: shared-memory
          mountPath: /dev/shm
        env:
        - name: AGENTIC_WARDEN_CONFIG_DIR
          value: "/config"
        - name: RUST_LOG
          value: "info"
        resources:
          requests:
            memory: "512Mi"
            cpu: "250m"
          limits:
            memory: "2Gi"
            cpu: "1000m"
      volumes:
      - name: config
        configMap:
          name: agentic-warden-config
      - name: shared-memory
        emptyDir:
          medium: Memory
```

## 系统服务部署

### 1. Systemd 服务（Linux）

#### 1.1 创建服务文件
```ini
# /etc/systemd/system/agentic-warden.service
[Unit]
Description=Agentic-Warden AI CLI Manager
After=network.target

[Service]
Type=simple
User=warden
Group=warden
WorkingDirectory=/opt/agentic-warden
ExecStart=/usr/local/bin/agentic-warden status --daemon
Restart=always
RestartSec=5
Environment=AGENTIC_WARDEN_CONFIG_DIR=/etc/agentic-warden
Environment=RUST_LOG=info

# 安全设置
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/etc/agentic-warden /var/log/agentic-warden

[Install]
WantedBy=multi-user.target
```

#### 1.2 创建用户和目录
```bash
# 创建系统用户
sudo useradd --system --home-dir /opt/agentic-warden --shell /bin/false warden

# 创建目录
sudo mkdir -p /etc/agentic-warden
sudo mkdir -p /var/log/agentic-warden
sudo mkdir -p /opt/agentic-warden

# 设置权限
sudo chown -R warden:warden /etc/agentic-warden
sudo chown -R warden:warden /var/log/agentic-warden
sudo chown -R warden:warden /opt/agentic-warden
```

#### 1.3 启用和启动服务
```bash
# 重新加载 systemd
sudo systemctl daemon-reload

# 启用服务（开机自启）
sudo systemctl enable agentic-warden

# 启动服务
sudo systemctl start agentic-warden

# 检查状态
sudo systemctl status agentic-warden
```

### 2. macOS LaunchAgent

#### 2.1 创建 plist 文件
```xml
# ~/Library/LaunchAgents/com.github.agentic-warden.plist
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.github.agentic-warden</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/agentic-warden</string>
        <string>status</string>
        <string>--daemon</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>EnvironmentVariables</key>
    <dict>
        <key>AGENTIC_WARDEN_CONFIG_DIR</key>
        <string>~/.agentic-warden</string>
    </dict>
    <key>StandardOutPath</key>
    <string>~/.agentic-warden/launchd.log</string>
    <key>StandardErrorPath</key>
    <string>~/.agentic-warden/launchd.err</string>
</dict>
</plist>
```

#### 2.2 加载服务
```bash
# 加载 LaunchAgent
launchctl load ~/Library/LaunchAgents/com.github.agentic-warden.plist

# 启动服务
launchctl start com.github.agentic-warden

# 检查状态
launchctl list | grep agentic-warden
```

### 3. Windows 服务

#### 3.1 使用 NSSM (Non-Sucking Service Manager)
```powershell
# 下载并安装 NSSM
# https://nssm.cc/download

# 安装服务
nssm install AgenticWarden "C:\Program Files\agentic-warden\agentic-warden.exe"
nssm set AgenticWarden Arguments "status --daemon"
nssm set AgenticWarden DisplayName "Agentic-Warden AI CLI Manager"
nssm set AgenticWarden Description "Manages AI CLI tools and processes"
nssm set AgenticWarden Start SERVICE_AUTO_START

# 设置环境变量
nssm set AgenticWarden Environment AGENTIC_WARDEN_CONFIG_DIR=C:\ProgramData\agentic-warden

# 设置日志
nssm set AgenticWarden AppStdout "C:\ProgramData\agentic-warden\stdout.log"
nssm set AgenticWarden AppStderr "C:\ProgramData\agentic-warden\stderr.log"

# 启动服务
nssm start AgenticWarden
```

## 配置管理

### 1. 环境变量配置

#### 1.1 支持的环境变量
```bash
# 配置目录
export AGENTIC_WARDEN_CONFIG_DIR="/path/to/config"

# 日志级别
export AGENTIC_WARDEN_LOG_LEVEL="info"

# 默认 Provider
export AGENTIC_WARDEN_DEFAULT_PROVIDER="official"

# 网络代理
export HTTP_PROXY="http://proxy.example.com:8080"
export HTTPS_PROXY="http://proxy.example.com:8080"
export NO_PROXY="localhost,127.0.0.1"

# Google Drive 配置
export GOOGLE_CLIENT_ID="your-client-id"
export GOOGLE_CLIENT_SECRET="your-client-secret"

# 安全设置
export AGENTIC_WARDEN_INSECURE="false"
export AGENTIC_WARDEN_ENCRYPT_TOKENS="true"
```

#### 1.2 环境配置文件
```bash
# ~/.bashrc 或 ~/.zshrc
export AGENTIC_WARDEN_CONFIG_DIR="$HOME/.agentic-warden"
export AGENTIC_WARDEN_LOG_LEVEL="info"

# 自动补全
eval "$(agentic-warden --completion)"
```

### 2. 生产环境配置

#### 2.1 生产环境检查清单
- [ ] 配置文件权限设置正确 (600 或 640)
- [ ] 认证令牌加密存储
- [ ] 日志轮转配置
- [ ] 监控和告警设置
- [ ] 备份策略制定
- [ ] 安全更新机制

#### 2.2 安全配置
```bash
# 设置配置目录权限
chmod 700 ~/.agentic-warden
chmod 600 ~/.agentic-warden/provider.json
chmod 600 ~/.agentic-warden/auth.json

# 注意：日志文件保存在系统临时目录，不需要设置权限
# Linux/macOS: /tmp/.agentic-warden/logs/
# Windows: %TEMP%\.agentic-warden\logs\
```

## 监控和维护

### 1. 健康检查

#### 1.1 系统健康检查
```bash
# 检查进程状态
ps aux | grep agentic-warden

# 检查配置文件
agentic-warden config validate

# 检查连接
agentic-warden health-check

# 检查共享内存
ipcs -m | grep agentic-warden
```

#### 1.2 日志监控

日志文件保存在系统临时目录：
- Linux/macOS: `/tmp/.agentic-warden/logs/<PID>.log`
- Windows: `%TEMP%\.agentic-warden\logs\<PID>.log`

```bash
# 实时查看日志（Linux/macOS）
tail -f /tmp/.agentic-warden/logs/*.log

# 查看错误日志
grep ERROR /tmp/.agentic-warden/logs/*.log

# 查看性能日志
grep "perf" /tmp/.agentic-warden/logs/*.log
```

**Windows PowerShell**:
```powershell
# 实时查看日志
Get-Content "$env:TEMP\.agentic-warden\logs\*.log" -Wait -Tail 50

# 查看错误日志
Select-String -Path "$env:TEMP\.agentic-warden\logs\*.log" -Pattern "ERROR"
```

### 2. 备份和恢复

#### 2.1 配置备份
```bash
#!/bin/bash
# backup.sh

BACKUP_DIR="$HOME/agentic-warden-backup-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$BACKUP_DIR"

# 备份配置文件
cp -r ~/.agentic-warden "$BACKUP_DIR/"

# 备份共享内存状态（如果需要）
ipcs -m > "$BACKUP_DIR/shared_memory.txt"

# 压缩备份
tar czf "$BACKUP_DIR.tar.gz" "$BACKUP_DIR"
rm -rf "$BACKUP_DIR"

echo "Backup created: $BACKUP_DIR.tar.gz"
```

#### 2.2 恢复配置
```bash
#!/bin/bash
# restore.sh

BACKUP_FILE="$1"

if [ -z "$BACKUP_FILE" ]; then
    echo "Usage: $0 <backup-file.tar.gz>"
    exit 1
fi

# 停止服务
systemctl --user stop agentic-warden 2>/dev/null || true

# 备份当前配置
mv ~/.agentic-warden ~/.agentic-warden.backup.$(date +%Y%m%d-%H%M%S)

# 恢复配置
tar xzf "$BACKUP_FILE" -C ~/

# 重启服务
systemctl --user start agentic-warden 2>/dev/null || true

echo "Configuration restored from $BACKUP_FILE"
```

### 3. 性能优化

#### 3.1 系统优化
```bash
# 增加共享内存限制
echo 'kernel.shmmax = 2147483648' | sudo tee -a /etc/sysctl.conf
echo 'kernel.shmall = 524288' | sudo tee -a /etc/sysctl.conf
sudo sysctl -p

# 优化文件描述符限制
echo 'fs.file-max = 65536' | sudo tee -a /etc/sysctl.conf
```

#### 3.2 应用优化
```json
{
  "process_tracking": {
    "scan_interval": 1,
    "max_instances": 50,
    "cleanup_dead_processes": true
  },
  "tui": {
    "auto_refresh_interval": 2,
    "max_displayed_tasks": 20
  },
  "network": {
    "connection_timeout": 15,
    "max_retries": 2
  }
}
```

## 故障排除

### 1. 常见问题

#### 1.1 启动失败
```bash
# 检查依赖
agentic-warden --version

# 检查配置
agentic-warden config validate

# 检查权限
ls -la ~/.agentic-warden

# 重新初始化配置
rm -rf ~/.agentic-warden
agentic-warden init
```

#### 1.2 进程跟踪问题
```bash
# 清理死掉的进程
agentic-warden cleanup

# 重建共享内存
agentic-warden shared-memory rebuild

# 检查权限
sudo ipcs -m
```

#### 1.3 Google Drive 授权问题
```bash
# 重新授权
rm ~/.agentic-warden/auth.json
agentic-warden push --auth

# 检查网络连接
curl -I https://www.googleapis.com

# 验证客户端配置
agentic-warden config show google
```

### 2. 日志分析

#### 2.1 日志级别设置
```bash
# 临时设置
RUST_LOG=debug agentic-warden status

# 永久设置
echo 'export RUST_LOG=debug' >> ~/.bashrc
```

#### 2.2 结构化日志
```json
{
  "timestamp": "2025-11-04T10:30:00Z",
  "level": "ERROR",
  "module": "sync::google_drive_service",
  "message": "Authentication failed",
  "task_id": "task-123",
  "error": "Invalid token"
}
```

## 更新和升级

### 1. 自动更新

#### 1.1 内置更新检查
```bash
# 检查更新
agentic-warden update check

# 下载并安装更新
agentic-warden update install

# 查看更新日志
agentic-warden update changelog
```

#### 1.2 包管理器更新
```bash
# Cargo
cargo install agentic-warden --force

# Homebrew
brew upgrade agentic-warden

# APT
sudo apt update && sudo apt upgrade agentic-warden

# Docker
docker pull ghcr.io/your-org/agentic-warden:latest
```

### 2. 配置迁移

#### 2.1 版本兼容性
- 配置文件格式向后兼容
- 自动迁移旧配置格式
- 备份原配置文件

#### 2.2 迁移脚本
```bash
#!/bin/bash
# migrate.sh

OLD_VERSION="0.1.0"
NEW_VERSION="0.2.0"

echo "Migrating from $OLD_VERSION to $NEW_VERSION..."

# 备份配置
cp ~/.agentic-warden/provider.json ~/.agentic-warden/provider.json.backup

# 执行迁移
agentic-warden config migrate --from $OLD_VERSION --to $NEW_VERSION

echo "Migration completed. Please review the changes."
```