# Agentic-Warden 配置管理

## 配置文件体系

### 1. 配置目录结构

#### 持久化配置目录（~/.agentic-warden/）
```
~/.agentic-warden/
├── provider.json              # Provider 配置文件
├── auth.json                  # Google Drive 认证信息
├── config.json                # 主配置文件
└── schema/                    # JSON Schema 文件
    ├── provider.json.schema   # Provider 配置 Schema
    └── config.json.schema     # 主配置 Schema
```

#### 运行时目录（/tmp/.agentic-warden/）
```
/tmp/.agentic-warden/
├── agentic-warden.log         # 日志文件
└── temp/                      # 临时文件目录
    ├── oauth_callback.html    # OAuth 回调页面
    └── download_cache/        # 下载缓存
```

**设计原则**：
- 持久化配置（provider、auth、config）保存在用户主目录，系统重启后保留
- 运行时数据（日志、临时文件）保存在 /tmp/，系统重启后自动清理
- 日志文件可在每次启动时重新创建，不需要长期保留

### 2. 配置文件优先级
1. 命令行参数 (最高优先级)
2. 环境变量
3. 用户配置文件 (`~/.agentic-warden/`)
4. 系统配置文件 (`/etc/agentic-warden/`)
5. 默认配置 (最低优先级)

## Provider 配置管理

### 1. Provider 配置文件格式

#### 1.1 完整配置示例
```json
{
  "$schema": "https://agentic-warden.dev/schema/provider.json",
  "version": "1.0.0",
  "format_version": 1,
  "providers": {
    "openrouter": {
      "name": "openrouter",
      "description": "OpenRouter 统一 LLM 网关，支持多种模型",
      "compatible_with": ["codex", "claude", "gemini"],
      "env": {
        "OPENAI_API_KEY": "sk-or-v1-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        "OPENAI_BASE_URL": "https://openrouter.ai/api/v1",
        "OPENAI_ORGANIZATION": ""
      },
      "metadata": {
        "website": "https://openrouter.ai",
        "documentation": "https://openrouter.ai/docs",
        "pricing_url": "https://openrouter.ai/pricing",
        "models": ["gpt-4", "claude-3-opus", "gemini-pro"]
      },
      "builtin": false,
      "created_at": "2025-11-04T10:00:00Z",
      "updated_at": "2025-11-04T10:00:00Z"
    },
    "litellm": {
      "name": "litellm",
      "description": "LiteLLM 本地代理服务器，提供统一的 API 接口",
      "compatible_with": ["codex", "claude", "gemini"],
      "env": {
        "ANTHROPIC_API_KEY": "sk-ant-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        "OPENAI_API_KEY": "sk-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        "ANTHROPIC_BASE_URL": "http://localhost:4000",
        "OPENAI_BASE_URL": "http://localhost:4000"
      },
      "metadata": {
        "server_address": "http://localhost:4000",
        "health_check_url": "http://localhost:4000/health",
        "timeout": 30
      },
      "builtin": false,
      "created_at": "2025-11-04T10:00:00Z",
      "updated_at": "2025-11-04T10:00:00Z"
    },
    "cloudflare": {
      "name": "cloudflare",
      "description": "Cloudflare AI Gateway，提供企业级 AI 服务",
      "compatible_with": ["codex", "claude", "gemini"],
      "env": {
        "CLOUDFLARE_API_TOKEN": "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        "CLOUDFLARE_ACCOUNT_ID": "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        "OPENAI_BASE_URL": "https://gateway.ai.cloudflare.com/v1/xxxxxxxxxxxxxx"
      },
      "metadata": {
        "account_type": "enterprise",
        "gateway_id": "xxxxxxxxxxxxxx",
        "rate_limit": 1000
      },
      "builtin": false,
      "created_at": "2025-11-04T10:00:00Z",
      "updated_at": "2025-11-04T10:00:00Z"
    },
    "official": {
      "name": "official",
      "description": "官方 API 直连，无需额外配置",
      "compatible_with": ["codex", "claude", "gemini"],
      "env": {},
      "metadata": {
        "website": "https://claude.ai",
        "direct_connection": true
      },
      "builtin": true,
      "created_at": "2025-11-04T10:00:00Z",
      "updated_at": "2025-11-04T10:00:00Z"
    }
  },
  "default_provider": "official",
  "settings": {
    "auto_refresh": true,
    "health_check_interval": 300,
    "connection_timeout": 30,
    "max_retries": 3,
    "validate_on_startup": true
  }
}
```

#### 1.2 Provider 配置字段说明
- **name**: Provider 唯一标识符
- **description**: 人类可读的描述信息
- **compatible_with**: 支持的 AI CLI 类型列表
- **env**: 环境变量映射表
- **metadata**: 额外的元数据信息
- **builtin**: 是否为内置 Provider
- **created_at/updated_at**: 创建和更新时间戳

#### 1.3 配置设置字段
- **auto_refresh**: 是否自动刷新 Provider 状态
- **health_check_interval**: 健康检查间隔（秒）
- **connection_timeout**: 连接超时时间（秒）
- **max_retries**: 最大重试次数
- **validate_on_startup**: 启动时是否验证配置

### 2. Provider 配置管理 API

#### 2.1 配置管理器接口
```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct ConfigManager {
    config_path: PathBuf,
    config: ProviderConfig,
    schema: serde_json::Value,
}

impl ConfigManager {
    /// 加载配置文件
    pub fn load() -> Result<Self>;

    /// 保存配置文件
    pub fn save(&self) -> Result<()>;

    /// 添加新的 Provider
    pub fn add_provider(&mut self, provider: Provider) -> Result<()>;

    /// 更新现有 Provider
    pub fn update_provider(&mut self, name: &str, provider: Provider) -> Result<()>;

    /// 删除 Provider
    pub fn remove_provider(&mut self, name: &str) -> Result<()>;

    /// 设置默认 Provider
    pub fn set_default(&mut self, name: &str) -> Result<()>;

    /// 获取 Provider
    pub fn get_provider(&self, name: &str) -> Option<&Provider>;

    /// 获取默认 Provider
    pub fn get_default_provider(&self) -> Option<&Provider>;

    /// 验证 Provider 配置
    pub fn validate_provider(&self, provider: &Provider) -> Result<()>;

    /// 验证所有 Provider 配置
    pub fn validate_all_providers(&self) -> Result<Vec<String>>;

    /// 获取环境变量映射
    pub fn get_env_vars(&self, provider_name: &str) -> Result<HashMap<String, String>>;

    /// 获取所有兼容指定 AI 类型的 Provider
    pub fn get_compatible_providers(&self, ai_type: AiType) -> Vec<&Provider>;

    /// 重置为默认配置
    pub fn reset_to_defaults(&mut self) -> Result<()>;

    /// 导出配置
    pub fn export_config(&self, path: &PathBuf) -> Result<()>;

    /// 导入配置
    pub fn import_config(&mut self, path: &PathBuf, merge: bool) -> Result<()>;
}
```

#### 2.2 Provider 验证规则
```rust
pub struct ProviderValidator;

impl ProviderValidator {
    /// 验证 Provider 名称
    pub fn validate_name(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(anyhow::anyhow!("Provider name cannot be empty"));
        }

        if name.len() > 50 {
            return Err(anyhow::anyhow!("Provider name too long (max 50 chars)"));
        }

        if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(anyhow::anyhow!("Provider name can only contain alphanumeric characters, hyphens, and underscores"));
        }

        Ok(())
    }

    /// 验证描述信息
    pub fn validate_description(description: &str) -> Result<()> {
        if description.is_empty() {
            return Err(anyhow::anyhow!("Description cannot be empty"));
        }

        if description.len() > 200 {
            return Err(anyhow::anyhow!("Description too long (max 200 chars)"));
        }

        Ok(())
    }

    /// 验证环境变量
    pub fn validate_env_vars(env_vars: &HashMap<String, String>) -> Result<()> {
        for (key, value) in env_vars {
            Self::validate_env_key(key)?;
            Self::validate_env_value(value)?;
        }
        Ok(())
    }

    fn validate_env_key(key: &str) -> Result<()> {
        if key.is_empty() {
            return Err(anyhow::anyhow!("Environment variable key cannot be empty"));
        }

        if !key.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(anyhow::anyhow!("Environment variable key '{}' contains invalid characters", key));
        }

        Ok(())
    }

    fn validate_env_value(value: &str) -> Result<()> {
        // 环境变量值可以为空（某些情况下）
        if value.len() > 1000 {
            return Err(anyhow::anyhow!("Environment variable value too long (max 1000 chars)"));
        }

        Ok(())
    }
}
```

### 3. 主配置文件管理

#### 3.1 主配置文件格式 (config.json)
```json
{
  "$schema": "https://agentic-warden.dev/schema/config.json",
  "version": "1.0.0",
  "general": {
    "auto_start_dashboard": false,
    "default_ai_cli": "claude",
    "log_level": "info",
    "enable_telemetry": false
  },
  "tui": {
    "theme": "default",
    "auto_refresh_interval": 2,
    "max_displayed_tasks": 50,
    "enable_animations": true,
    "keybindings": {
      "quit": ["q", "Q"],
      "escape": ["Esc"],
      "select_up": ["Up", "k"],
      "select_down": ["Down", "j"],
      "confirm": ["Enter"],
      "delete": ["d", "D"],
      "edit": ["e", "E"],
      "add": ["a", "A"]
    }
  },
  "process_tracking": {
    "scan_interval": 1,
    "max_instances": 100,
    "cleanup_dead_processes": true,
    "root_process_detection": "auto"
  },
  "sync": {
    "google_drive": {
      "auto_sync": false,
      "sync_interval": 3600,
      "exclude_patterns": [
        "*.tmp",
        "*.log",
        ".git/",
        "target/",
        "node_modules/"
      ],
      "max_file_size": 104857600,
      "parallel_uploads": 4
    }
  },
  "network": {
    "connection_timeout": 30,
    "max_retries": 3,
    "retry_delay": 5,
    "proxy": {
      "enabled": false,
      "http": "",
      "https": "",
      "no_proxy": ["localhost", "127.0.0.1"]
    }
  },
  "security": {
    "encrypt_auth_tokens": true,
    "token_expiry_check": true,
    "insecure_connections": false
  }
}
```

#### 3.2 配置字段说明

##### General (通用设置)
- **auto_start_dashboard**: 是否自动启动 Dashboard
- **default_ai_cli**: 默认 AI CLI 工具
- **log_level**: 日志级别 (trace, debug, info, warn, error)
- **enable_telemetry**: 是否启用遥测数据收集

##### TUI (终端用户界面)
- **theme**: TUI 主题 (default, dark, light)
- **auto_refresh_interval**: 自动刷新间隔（秒）
- **max_displayed_tasks**: 最大显示任务数
- **enable_animations**: 是否启用动画效果
- **keybindings**: 键盘快捷键绑定

##### Process Tracking (进程跟踪)
- **scan_interval**: 扫描间隔（秒）
- **max_instances**: 最大实例数
- **cleanup_dead_processes**: 是否清理死亡进程
- **root_process_detection**: 根进程检测模式 (auto, windows, linux, macos)

##### Sync (同步设置)
- **auto_sync**: 是否自动同步
- **sync_interval**: 同步间隔（秒）
- **exclude_patterns**: 排除文件模式
- **max_file_size**: 最大文件大小（字节）
- **parallel_uploads**: 并行上传数

##### Network (网络设置)
- **connection_timeout**: 连接超时（秒）
- **max_retries**: 最大重试次数
- **retry_delay**: 重试延迟（秒）
- **proxy**: 代理设置

##### Security (安全设置)
- **encrypt_auth_tokens**: 是否加密认证令牌
- **token_expiry_check**: 是否检查令牌过期
- **insecure_connections**: 是否允许不安全连接

### 4. 环境变量配置

#### 4.1 支持的环境变量
```bash
# 配置目录 override
AGENTIC_WARDEN_CONFIG_DIR="/path/to/config"

# 日志级别 override
AGENTIC_WARDEN_LOG_LEVEL="debug"

# 默认 Provider override
AGENTIC_WARDEN_DEFAULT_PROVIDER="openrouter"

# 网络代理设置
HTTP_PROXY="http://proxy.example.com:8080"
HTTPS_PROXY="http://proxy.example.com:8080"
NO_PROXY="localhost,127.0.0.1"

# Google Drive 配置
GOOGLE_CLIENT_ID="your-client-id"
GOOGLE_CLIENT_SECRET="your-client-secret"

# TUI 设置
AGENTIC_WARDEN_THEME="dark"
AGENTIC_WARDEN_NO_ANIMATIONS="1"

# 安全设置
AGENTIC_WARDEN_INSECURE="1"
```

#### 4.2 环境变量优先级
1. 命令行参数
2. 环境变量 (`AGENTIC_WARDEN_*`)
3. 用户配置文件
4. 默认值

### 5. 配置验证和迁移

#### 5.1 配置验证流程
```rust
pub struct ConfigValidator;

impl ConfigValidator {
    /// 验证完整配置
    pub fn validate_config(config: &Config) -> Result<Vec<String>> {
        let mut warnings = Vec::new();

        // 验证通用设置
        Self::validate_general(&config.general, &mut warnings);

        // 验证 TUI 设置
        Self::validate_tui(&config.tui, &mut warnings);

        // 验证进程跟踪设置
        Self::validate_process_tracking(&config.process_tracking, &mut warnings);

        // 验证同步设置
        Self::validate_sync(&config.sync, &mut warnings);

        // 验证网络设置
        Self::validate_network(&config.network, &mut warnings);

        // 验证安全设置
        Self::validate_security(&config.security, &mut warnings);

        Ok(warnings)
    }

    fn validate_general(general: &GeneralConfig, warnings: &mut Vec<String>) {
        if general.log_level == "trace" {
            warnings.push("Trace logging may impact performance".to_string());
        }

        if general.enable_telemetry {
            warnings.push("Telemetry is enabled - privacy data may be collected".to_string());
        }
    }

    // ... 其他验证方法
}
```

#### 5.2 配置迁移策略
```rust
pub struct ConfigMigrator;

impl ConfigMigrator {
    /// 迁移配置到最新版本
    pub fn migrate(config_path: &PathBuf) -> Result<()> {
        let config = Self::load_config(config_path)?;
        let current_version = config.version.parse::<semver::Version>()?;
        let target_version = semver::Version::parse("1.0.0")?;

        if current_version < target_version {
            Self::perform_migration(config_path, &current_version, &target_version)?;
        }

        Ok(())
    }

    fn perform_migration(
        config_path: &PathBuf,
        from_version: &semver::Version,
        to_version: &semver::Version,
    ) -> Result<()> {
        // 备份原配置
        let backup_path = config_path.with_extension("json.bak");
        std::fs::copy(config_path, &backup_path)?;

        // 执行具体迁移逻辑
        // ...

        Ok(())
    }
}
```

### 6. 配置热重载

#### 6.1 文件监控机制
```rust
use notify::{Watcher, RecursiveMode, RecommendedWatcher};
use std::sync::mpsc;

pub struct ConfigWatcher {
    watcher: RecommendedWatcher,
    config_path: PathBuf,
    reload_callback: Box<dyn Fn() + Send>,
}

impl ConfigWatcher {
    pub fn new<F>(config_path: PathBuf, callback: F) -> Result<Self>
    where
        F: Fn() + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx)?;

        watcher.watch(&config_path, RecursiveMode::NonRecursive)?;

        // 启动监控线程
        let callback = Box::new(callback);
        std::thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                match event {
                    Ok(notify::Event { kind: notify::EventKind::Modify(..), .. }) => {
                        callback();
                    }
                    _ => {}
                }
            }
        });

        Ok(Self {
            watcher,
            config_path,
            reload_callback: callback,
        })
    }
}
```

## 配置最佳实践

### 1. 安全性
- 敏感信息（API 密钥）使用环境变量
- 认证令牌加密存储
- 配置文件权限控制 (600)
- 避免在版本控制中提交敏感配置

### 2. 性能
- 配置文件大小控制在合理范围
- 避免频繁的配置文件读写
- 使用内存缓存减少磁盘 I/O
- 异步配置加载

### 3. 可维护性
- 使用 JSON Schema 验证配置
- 提供配置文件模板和示例
- 详细的配置文档和注释
- 版本化配置格式

### 4. 用户体验
- 友好的错误提示信息
- 配置验证和修复建议
- TUI 界面配置管理
- 配置导入导出功能