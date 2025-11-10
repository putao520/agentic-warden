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

**注意**: Agentic-Warden **不管理** AI CLI 的原生配置文件：
- `~/.claude/config` - Claude CLI 原生配置
- `~/.codex/config` - Codex CLI 原生配置
- `~/.gemini/config` - Gemini CLI 原生配置

这些配置文件由各AI CLI自己管理，Agentic-Warden只通过环境变量注入来动态切换Provider。

#### 运行时目录（系统临时目录/.agentic-warden/）
```
<TEMP>/.agentic-warden/         # <TEMP> 由 std::env::temp_dir() 动态获取
├── agentic-warden.log          # 日志文件
└── temp/                       # 临时文件目录
    ├── oauth_callback.html     # OAuth 回调页面
    └── download_cache/         # 下载缓存
```

**平台路径映射**：
- Linux/macOS: `/tmp/.agentic-warden/`
- Windows: `%TEMP%\.agentic-warden\`

**设计原则**：
- 持久化配置（provider、auth、config）保存在用户主目录，系统重启后保留
- 运行时数据（日志、临时文件）保存在系统临时目录，系统重启后自动清理
- 使用 `std::env::temp_dir()` 动态获取临时目录路径，确保跨平台兼容性
- 日志文件可在每次启动时重新创建，不需要长期保留

### 2. 配置文件优先级
1. 命令行参数 (最高优先级)
2. 环境变量
3. 用户配置文件 (`~/.agentic-warden/`)
4. 系统配置文件 (`/etc/agentic-warden/`)
5. 默认配置 (最低优先级)

## Provider 配置管理

### Provider vs AI CLI 配置关系

**Provider配置** (`provider.json`):
- 管理第三方API提供商的配置（OpenRouter、LiteLLM等）
- 定义环境变量映射（API密钥、Base URL等）
- 启动AI CLI时通过 `-p` 参数选择，自动注入环境变量

**AI CLI原生配置** (各CLI自己的配置文件):
- Agentic-Warden **不修改、不管理** 这些配置
- AI CLI的模型选择、超时设置等保持原生配置方式
- 用户直接使用各CLI的原生配置命令（如 `claude config set model ...`）

**示例对比**:
```bash
# ❌ Agentic-Warden 不做这些
agentic-warden config set claude.model "claude-3-opus"

# ✅ 正确做法：使用原生CLI配置
claude config set model "claude-3-opus"

# ✅ Agentic-Warden 只管理Provider切换
agentic-warden agent claude -p openrouter "task"  # 使用openrouter
agentic-warden agent claude -p official "task"     # 使用官方API
```

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
  "version": "1.0.0",
  "general": {
    "default_ai_cli": "claude",
    "log_level": "info"
  },
  "process_tracking": {
    "scan_interval": 1,
    "max_instances": 100,
    "cleanup_dead_processes": true
  },
  "sync": {
    "google_drive": {
      "auto_sync": false,
      "exclude_patterns": [
        "*.tmp",
        "*.log",
        ".git/",
        "target/",
        "node_modules/"
      ]
    }
  }
}
```

#### 3.2 配置字段说明

##### General (通用设置)
- **default_ai_cli**: 默认 AI CLI 工具 (claude, codex, gemini)
- **log_level**: 日志级别 (info, warn, error)

##### Process Tracking (进程跟踪)
- **scan_interval**: 扫描间隔（秒）
- **max_instances**: 最大实例数
- **cleanup_dead_processes**: 是否清理死亡进程

##### Sync (同步设置)
- **auto_sync**: 是否自动同步到Google Drive
- **exclude_patterns**: 排除文件模式列表

### 4. 环境变量配置

#### 4.1 支持的环境变量
```bash
# 配置目录
AGENTIC_WARDEN_CONFIG_DIR="/path/to/config"

# 日志级别
AGENTIC_WARDEN_LOG_LEVEL="info"

# 默认 Provider
AGENTIC_WARDEN_DEFAULT_PROVIDER="openrouter"

# Google Drive OAuth
GOOGLE_CLIENT_ID="your-client-id"
GOOGLE_CLIENT_SECRET="your-client-secret"
```

#### 4.2 环境变量优先级
1. 命令行参数
2. 环境变量 (`AGENTIC_WARDEN_*`)
3. 用户配置文件
4. 默认值

### 5. 配置错误处理
对于配置文件格式错误或版本不兼容的情况，提供清晰的错误提示信息，引导用户修正配置。不提供自动迁移功能以保持简洁性。


## 配置最佳实践

### 1. 安全性
- 敏感信息（API 密钥）使用环境变量
- 配置文件权限控制 (600)
- 避免在版本控制中提交敏感配置

### 2. 简洁性
- 保持配置文件简单明了
- 提供清晰的错误提示
- 只配置必要的选项