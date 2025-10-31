# 第三方 API Provider 系统设计文档

## 1. 概述

### 1.1 设计目标
实现一个灵活的第三方 API Provider 配置系统,允许用户通过配置文件管理多个 API 提供商,并在启动 AI CLI 时动态注入环境变量到子进程。

### 1.2 核心原则
- **环境变量隔离**: 使用 `Command::env()` 将环境变量注入到 AI CLI 子进程,不污染父进程
- **配置集中管理**: 通过 `~/.agentic-warden/provider.json` 统一管理所有 provider
- **类型安全**: 使用 Rust 类型系统确保配置正确性
- **用户友好**: 提供交互式配置界面和清晰的错误提示

---

## 2. 架构设计

### 2.1 系统流程图

```
┌──────────────────────────────────────────────────────────────────┐
│  用户命令: agentic-warden codex -p openrouter "写一个函数"         │
└────────────────────────┬─────────────────────────────────────────┘
                         │
                         ▼
┌──────────────────────────────────────────────────────────────────┐
│  1. CLI 参数解析 (clap)                                           │
│     - ai_type: Codex                                             │
│     - provider: Some("openrouter")                               │
│     - prompt: "写一个函数"                                        │
└────────────────────────┬─────────────────────────────────────────┘
                         │
                         ▼
┌──────────────────────────────────────────────────────────────────┐
│  2. ProviderManager::load()                                      │
│     - 读取 ~/.agentic-warden/provider.json                       │
│     - 反序列化为 ProviderConfig                                   │
│     - 如果文件不存在,创建默认配置                                  │
└────────────────────────┬─────────────────────────────────────────┘
                         │
                         ▼
┌──────────────────────────────────────────────────────────────────┐
│  3. 获取 Provider 配置                                            │
│     - provider_name = "openrouter"                               │
│     - provider_config = config.providers.get("openrouter")       │
│     - 如果不存在 → ProviderError::ProviderNotFound               │
└────────────────────────┬─────────────────────────────────────────┘
                         │
                         ▼
┌──────────────────────────────────────────────────────────────────┐
│  4. 兼容性验证                                                     │
│     - provider_config.compatible_with.contains(&AiType::Codex)   │
│     - 如果不兼容 → ProviderError::IncompatibleProvider           │
└────────────────────────┬─────────────────────────────────────────┘
                         │
                         ▼
┌──────────────────────────────────────────────────────────────────┐
│  5. 构建子进程命令                                                 │
│     - let mut cmd = Command::new("codex");                       │
│     - cmd.arg(prompt);                                           │
└────────────────────────┬─────────────────────────────────────────┘
                         │
                         ▼
┌──────────────────────────────────────────────────────────────────┐
│  6. 动态注入环境变量到子进程                                        │
│     for (key, value) in &provider_config.env {                   │
│         cmd.env(key, value);  // ← 关键: 仅注入到子进程            │
│     }                                                             │
│     例如:                                                          │
│     - cmd.env("OPENAI_API_KEY", "sk-or-v1-xxx")                  │
│     - cmd.env("OPENAI_BASE_URL", "https://openrouter.ai/api/v1") │
└────────────────────────┬─────────────────────────────────────────┘
                         │
                         ▼
┌──────────────────────────────────────────────────────────────────┐
│  7. 启动子进程                                                     │
│     - cmd.spawn()?                                               │
│     - 子进程继承注入的环境变量                                      │
│     - 父进程环境变量不受影响                                        │
└────────────────────────┬─────────────────────────────────────────┘
                         │
                         ▼
┌──────────────────────────────────────────────────────────────────┐
│  8. AI CLI 使用注入的环境变量                                       │
│     codex 内部执行:                                               │
│     - OPENAI_API_KEY = "sk-or-v1-xxx"                            │
│     - OPENAI_BASE_URL = "https://openrouter.ai/api/v1"          │
│     - 发送请求到 OpenRouter                                       │
└──────────────────────────────────────────────────────────────────┘
```

### 2.2 关键设计点

#### 环境变量注入方式
**✅ 正确方式 (使用 Command::env())**:
```rust
let mut cmd = Command::new("codex");
cmd.arg(prompt);

// 动态注入环境变量到子进程
for (key, value) in &provider_config.env {
    cmd.env(key, value);
}

cmd.spawn()?; // 子进程拥有这些环境变量
```

**❌ 错误方式 (污染父进程)**:
```rust
// 不要这样做!
for (key, value) in &provider_config.env {
    std::env::set_var(key, value); // 会影响父进程
}

Command::new("codex").spawn()?;
```

---

## 3. 数据结构设计

### 3.1 核心数据结构

```rust
// src/provider/config.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Provider 配置文件根结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// JSON Schema (可选)
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    
    /// 所有 provider 配置
    pub providers: HashMap<String, Provider>,
    
    /// 默认 provider 名称
    pub default_provider: String,
}

/// 单个 Provider 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    /// Provider 描述信息
    pub description: String,
    
    /// 兼容的 AI 类型列表
    pub compatible_with: Vec<AiType>,
    
    /// 环境变量映射 (key = 环境变量名, value = 环境变量值)
    pub env: HashMap<String, String>,
}

/// AI 类型枚举
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AiType {
    Codex,
    Claude,
    Gemini,
}

impl std::fmt::Display for AiType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiType::Codex => write!(f, "codex"),
            AiType::Claude => write!(f, "claude"),
            AiType::Gemini => write!(f, "gemini"),
        }
    }
}

impl ProviderConfig {
    /// 创建默认配置
    pub fn default() -> Self {
        let mut providers = HashMap::new();
        providers.insert(
            "official".to_string(),
            Provider {
                description: "Official API endpoints (default)".to_string(),
                compatible_with: vec![AiType::Codex, AiType::Claude, AiType::Gemini],
                env: HashMap::new(),
            },
        );

        Self {
            schema: Some("https://agentic-warden.dev/schema/provider.json".to_string()),
            providers,
            default_provider: "official".to_string(),
        }
    }
}
```

### 3.2 错误类型

```rust
// src/provider/error.rs

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Provider '{0}' not found in configuration")]
    ProviderNotFound(String),
    
    #[error("Provider '{provider}' is not compatible with {ai_type}. Compatible AI types: {compatible}")]
    IncompatibleProvider {
        provider: String,
        ai_type: String,
        compatible: String,
    },
    
    #[error("Failed to load provider configuration: {0}")]
    ConfigLoadError(String),
    
    #[error("Failed to save provider configuration: {0}")]
    ConfigSaveError(String),
    
    #[error("Invalid provider configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Provider name '{0}' is reserved and cannot be used")]
    ReservedName(String),
    
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("JSON serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),
}

pub type ProviderResult<T> = Result<T, ProviderError>;
```

---

## 4. 核心模块实现

### 4.1 ProviderManager (管理器)

```rust
// src/provider/manager.rs

use super::config::{ProviderConfig, Provider, AiType};
use super::error::{ProviderError, ProviderResult};
use std::path::PathBuf;
use std::fs;
use std::collections::HashMap;

const PROVIDER_FILE_NAME: &str = "provider.json";
const AUTH_DIRECTORY: &str = ".agentic-warden";

pub struct ProviderManager {
    config_path: PathBuf,
    config: ProviderConfig,
}

impl ProviderManager {
    /// 创建新的 ProviderManager
    pub fn new() -> ProviderResult<Self> {
        let config_path = Self::get_config_path()?;
        let config = Self::load_or_create(&config_path)?;
        
        Ok(Self {
            config_path,
            config,
        })
    }
    
    /// 获取配置文件路径
    fn get_config_path() -> ProviderResult<PathBuf> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| ProviderError::ConfigLoadError("Cannot find home directory".to_string()))?;
        
        let config_dir = home_dir.join(AUTH_DIRECTORY);
        
        // 确保目录存在
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
            
            // 设置目录权限 (仅 Unix)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&config_dir)?.permissions();
                perms.set_mode(0o700); // rwx------
                fs::set_permissions(&config_dir, perms)?;
            }
        }
        
        Ok(config_dir.join(PROVIDER_FILE_NAME))
    }
    
    /// 加载或创建配置文件
    fn load_or_create(path: &PathBuf) -> ProviderResult<ProviderConfig> {
        if path.exists() {
            Self::load_from_file(path)
        } else {
            let config = ProviderConfig::default();
            Self::save_to_file(path, &config)?;
            Ok(config)
        }
    }
    
    /// 从文件加载配置
    fn load_from_file(path: &PathBuf) -> ProviderResult<ProviderConfig> {
        let content = fs::read_to_string(path)
            .map_err(|e| ProviderError::ConfigLoadError(e.to_string()))?;
        
        let config: ProviderConfig = serde_json::from_str(&content)
            .map_err(|e| ProviderError::ConfigLoadError(format!("Invalid JSON: {}", e)))?;
        
        Ok(config)
    }
    
    /// 保存配置到文件
    fn save_to_file(path: &PathBuf, config: &ProviderConfig) -> ProviderResult<()> {
        let json = serde_json::to_string_pretty(config)?;
        fs::write(path, json)?;
        
        // 设置文件权限 (仅 Unix)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path)?.permissions();
            perms.set_mode(0o600); // rw-------
            fs::set_permissions(path, perms)?;
        }
        
        Ok(())
    }
    
    /// 保存当前配置
    pub fn save(&self) -> ProviderResult<()> {
        Self::save_to_file(&self.config_path, &self.config)
    }
    
    /// 获取指定名称的 provider
    pub fn get_provider(&self, name: &str) -> ProviderResult<&Provider> {
        self.config.providers.get(name)
            .ok_or_else(|| ProviderError::ProviderNotFound(name.to_string()))
    }
    
    /// 获取默认 provider
    pub fn get_default_provider(&self) -> ProviderResult<(&String, &Provider)> {
        let name = &self.config.default_provider;
        let provider = self.get_provider(name)?;
        Ok((name, provider))
    }
    
    /// 验证 provider 与 AI 类型的兼容性
    pub fn validate_compatibility(&self, provider_name: &str, ai_type: AiType) -> ProviderResult<()> {
        let provider = self.get_provider(provider_name)?;
        
        if !provider.compatible_with.contains(&ai_type) {
            let compatible = provider.compatible_with.iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            
            return Err(ProviderError::IncompatibleProvider {
                provider: provider_name.to_string(),
                ai_type: ai_type.to_string(),
                compatible,
            });
        }
        
        Ok(())
    }
    
    /// 添加新 provider
    pub fn add_provider(&mut self, name: String, provider: Provider) -> ProviderResult<()> {
        if name == "official" {
            return Err(ProviderError::ReservedName(name));
        }
        
        self.config.providers.insert(name, provider);
        self.save()?;
        Ok(())
    }
    
    /// 删除 provider
    pub fn remove_provider(&mut self, name: &str) -> ProviderResult<()> {
        if name == "official" {
            return Err(ProviderError::ReservedName(name.to_string()));
        }
        
        if name == self.config.default_provider {
            return Err(ProviderError::InvalidConfig(
                format!("Cannot remove default provider '{}'. Set another default first.", name)
            ));
        }
        
        self.config.providers.remove(name)
            .ok_or_else(|| ProviderError::ProviderNotFound(name.to_string()))?;
        
        self.save()?;
        Ok(())
    }
    
    /// 设置默认 provider
    pub fn set_default(&mut self, name: &str) -> ProviderResult<()> {
        // 验证 provider 存在
        self.get_provider(name)?;
        
        self.config.default_provider = name.to_string();
        self.save()?;
        Ok(())
    }
    
    /// 列出所有 providers
    pub fn list_providers(&self) -> Vec<(&String, &Provider)> {
        self.config.providers.iter().collect()
    }
}
```

### 4.2 环境变量注入 (核心功能)

```rust
// src/provider/env_injector.rs

use std::collections::HashMap;
use std::process::Command;

pub struct EnvInjector;

impl EnvInjector {
    /// 将环境变量注入到 Command 对象
    /// 
    /// 这个方法不会修改当前进程的环境变量,
    /// 只会在启动子进程时传递这些环境变量
    pub fn inject_to_command(cmd: &mut Command, env_vars: &HashMap<String, String>) {
        for (key, value) in env_vars {
            cmd.env(key, value);
        }
    }
    
    /// 获取环境变量的安全显示版本 (隐藏敏感信息)
    pub fn mask_sensitive_value(key: &str, value: &str) -> String {
        // 包含 KEY, SECRET, TOKEN, PASSWORD 的变量需要隐藏
        let sensitive_keywords = ["KEY", "SECRET", "TOKEN", "PASSWORD"];
        
        if sensitive_keywords.iter().any(|kw| key.to_uppercase().contains(kw)) {
            Self::mask_api_key(value)
        } else {
            value.to_string()
        }
    }
    
    /// 隐藏 API 密钥 (仅显示前4位和后4位)
    fn mask_api_key(key: &str) -> String {
        if key.len() <= 8 {
            return "***".to_string();
        }
        format!("{}***{}", &key[..4], &key[key.len()-4..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_api_key() {
        assert_eq!(
            EnvInjector::mask_api_key("sk-ant-api-1234567890abcdef"),
            "sk-a***cdef"
        );
        
        assert_eq!(
            EnvInjector::mask_api_key("short"),
            "***"
        );
    }
    
    #[test]
    fn test_mask_sensitive_value() {
        assert_eq!(
            EnvInjector::mask_sensitive_value("OPENAI_API_KEY", "sk-1234567890"),
            "sk-1***890"
        );
        
        assert_eq!(
            EnvInjector::mask_sensitive_value("OPENAI_BASE_URL", "https://api.openai.com"),
            "https://api.openai.com"
        );
    }
}
```

---

## 5. CLI 集成实现

### 5.1 启动 AI CLI 时注入环境变量

```rust
// src/cli_manager.rs (修改部分)

use crate::provider::{ProviderManager, EnvInjector, AiType};
use std::process::Command;

pub async fn handle_codex_command(
    prompt: String,
    provider: Option<String>,
    // ... 其他参数
) -> Result<i32> {
    // 1. 加载 provider 配置
    let provider_manager = ProviderManager::new()?;
    
    // 2. 确定使用哪个 provider
    let (provider_name, provider_config) = if let Some(name) = provider {
        (name.clone(), provider_manager.get_provider(&name)?)
    } else {
        let (name, config) = provider_manager.get_default_provider()?;
        (name.clone(), config)
    };
    
    // 3. 验证兼容性
    provider_manager.validate_compatibility(&provider_name, AiType::Codex)?;
    
    // 4. 显示提示信息
    if provider_name != "official" {
        println!("🔌 Using provider: {} ({})", provider_name, provider_config.description);
    }
    
    // 5. 构建命令
    let mut cmd = Command::new("codex");
    cmd.arg(&prompt);
    
    // 6. ⭐ 核心: 动态注入环境变量到子进程 ⭐
    EnvInjector::inject_to_command(&mut cmd, &provider_config.env);
    
    // 7. 启动子进程
    println!("🚀 Launching codex...");
    let status = cmd.status()?;
    
    Ok(status.code().unwrap_or(1))
}

// 类似地实现 handle_claude_command 和 handle_gemini_command
```

### 5.2 Provider 管理命令实现

```rust
// src/provider/commands.rs

use super::manager::ProviderManager;
use super::config::{Provider, AiType};
use super::env_injector::EnvInjector;
use dialoguer::{Input, MultiSelect, Confirm};
use std::collections::HashMap;

pub struct ProviderCommands;

impl ProviderCommands {
    /// 列出所有 providers
    pub fn list() -> Result<()> {
        let manager = ProviderManager::new()?;
        let providers = manager.list_providers();
        
        if providers.is_empty() {
            println!("No providers configured.");
            return Ok(());
        }
        
        println!("\n📦 Available Providers:\n");
        
        for (name, provider) in providers {
            let is_default = name == &manager.config.default_provider;
            let marker = if is_default { " (default)" } else { "" };
            
            println!("  • {}{}", name, marker);
            println!("    Description: {}", provider.description);
            println!("    Compatible with: {}", 
                provider.compatible_with.iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            
            if !provider.env.is_empty() {
                println!("    Environment variables:");
                for (key, value) in &provider.env {
                    let display_value = EnvInjector::mask_sensitive_value(key, value);
                    println!("      {} = {}", key, display_value);
                }
            }
            println!();
        }
        
        Ok(())
    }
    
    /// 交互式添加 provider
    pub fn add(name: String) -> Result<()> {
        let mut manager = ProviderManager::new()?;
        
        // 检查是否已存在
        if manager.config.providers.contains_key(&name) {
            return Err(anyhow::anyhow!("Provider '{}' already exists. Use 'provider edit' to modify it.", name));
        }
        
        println!("\n📝 Adding new provider: {}\n", name);
        
        // 1. 输入描述
        let description: String = Input::new()
            .with_prompt("Description")
            .interact_text()?;
        
        // 2. 选择兼容的 AI 类型
        let ai_types = vec!["codex", "claude", "gemini"];
        let selections = MultiSelect::new()
            .with_prompt("Compatible with (use Space to select, Enter to confirm)")
            .items(&ai_types)
            .interact()?;
        
        let compatible_with: Vec<AiType> = selections.iter()
            .map(|&i| match ai_types[i] {
                "codex" => AiType::Codex,
                "claude" => AiType::Claude,
                "gemini" => AiType::Gemini,
                _ => unreachable!(),
            })
            .collect();
        
        // 3. 输入环境变量
        println!("\nEnvironment variables (press Enter with empty value to finish):");
        let mut env = HashMap::new();
        
        loop {
            let key: String = Input::new()
                .with_prompt("Variable name")
                .allow_empty(true)
                .interact_text()?;
            
            if key.is_empty() {
                break;
            }
            
            let value: String = Input::new()
                .with_prompt(&format!("{} value", key))
                .interact_text()?;
            
            env.insert(key, value);
        }
        
        // 4. 创建 provider
        let provider = Provider {
            description,
            compatible_with,
            env,
        };
        
        // 5. 保存
        manager.add_provider(name.clone(), provider)?;
        
        println!("\n✅ Provider '{}' added successfully!", name);
        
        // 6. 询问是否设为默认
        if Confirm::new()
            .with_prompt("Set as default provider?")
            .default(false)
            .interact()?
        {
            manager.set_default(&name)?;
            println!("✅ Set '{}' as default provider", name);
        }
        
        Ok(())
    }
    
    /// 显示 provider 配置
    pub fn show(name: String) -> Result<()> {
        let manager = ProviderManager::new()?;
        let provider = manager.get_provider(&name)?;
        
        println!("\n📦 Provider: {}\n", name);
        println!("Description: {}", provider.description);
        println!("Compatible with: {}", 
            provider.compatible_with.iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
        
        if !provider.env.is_empty() {
            println!("\nEnvironment variables:");
            for (key, value) in &provider.env {
                let display_value = EnvInjector::mask_sensitive_value(key, value);
                println!("  {} = {}", key, display_value);
            }
        }
        
        println!();
        Ok(())
    }
    
    /// 删除 provider
    pub fn remove(name: String) -> Result<()> {
        let mut manager = ProviderManager::new()?;
        
        if !Confirm::new()
            .with_prompt(&format!("Are you sure you want to remove provider '{}'?", name))
            .default(false)
            .interact()?
        {
            println!("Cancelled.");
            return Ok(());
        }
        
        manager.remove_provider(&name)?;
        println!("✅ Provider '{}' removed successfully!", name);
        
        Ok(())
    }
    
    /// 设置默认 provider
    pub fn set_default(name: String) -> Result<()> {
        let mut manager = ProviderManager::new()?;
        manager.set_default(&name)?;
        
        println!("✅ Set '{}' as default provider", name);
        Ok(())
    }
}
```

---

## 6. 使用示例

### 6.1 配置 OpenRouter

```bash
# 添加 OpenRouter provider
$ agentic-warden provider add openrouter

📝 Adding new provider: openrouter

Description: OpenRouter unified LLM gateway
Compatible with (use Space to select, Enter to confirm):
  [x] codex
  [x] claude
  [ ] gemini

Environment variables (press Enter with empty value to finish):
Variable name: OPENAI_API_KEY
OPENAI_API_KEY value: sk-or-v1-1234567890abcdef
Variable name: OPENAI_BASE_URL
OPENAI_BASE_URL value: https://openrouter.ai/api/v1
Variable name: 

✅ Provider 'openrouter' added successfully!

Set as default provider? [y/N]: n
```

生成的 `provider.json`:
```json
{
  "$schema": "https://agentic-warden.dev/schema/provider.json",
  "providers": {
    "official": {
      "description": "Official API endpoints (default)",
      "compatible_with": ["codex", "claude", "gemini"],
      "env": {}
    },
    "openrouter": {
      "description": "OpenRouter unified LLM gateway",
      "compatible_with": ["codex", "claude"],
      "env": {
        "OPENAI_API_KEY": "sk-or-v1-1234567890abcdef",
        "OPENAI_BASE_URL": "https://openrouter.ai/api/v1"
      }
    }
  },
  "default_provider": "official"
}
```

### 6.2 使用 Provider

```bash
# 使用 OpenRouter 调用 codex
$ agentic-warden codex -p openrouter "写一个快速排序"
🔌 Using provider: openrouter (OpenRouter unified LLM gateway)
🚀 Launching codex...
[codex 正常输出,使用 OpenRouter API]
```

### 6.3 查看配置

```bash
$ agentic-warden provider list

📦 Available Providers:

  • official (default)
    Description: Official API endpoints (default)
    Compatible with: codex, claude, gemini

  • openrouter
    Description: OpenRouter unified LLM gateway
    Compatible with: codex, claude
    Environment variables:
      OPENAI_API_KEY = sk-o***cdef
      OPENAI_BASE_URL = https://openrouter.ai/api/v1
```

---

## 7. 测试计划

### 7.1 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_config_serialization() {
        let config = ProviderConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ProviderConfig = serde_json::from_str(&json).unwrap();
        
        assert_eq!(config.default_provider, deserialized.default_provider);
    }
    
    #[test]
    fn test_compatibility_validation() {
        let mut providers = HashMap::new();
        providers.insert("test".to_string(), Provider {
            description: "Test".to_string(),
            compatible_with: vec![AiType::Codex],
            env: HashMap::new(),
        });
        
        let config = ProviderConfig {
            schema: None,
            providers,
            default_provider: "test".to_string(),
        };
        
        let manager = ProviderManager { config_path: PathBuf::new(), config };
        
        // 应该成功
        assert!(manager.validate_compatibility("test", AiType::Codex).is_ok());
        
        // 应该失败
        assert!(manager.validate_compatibility("test", AiType::Gemini).is_err());
    }
}
```

### 7.2 集成测试

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::env;

    #[test]
    fn test_env_injection_to_command() {
        let mut env_vars = HashMap::new();
        env_vars.insert("TEST_KEY".to_string(), "test_value".to_string());
        
        let mut cmd = Command::new("echo");
        EnvInjector::inject_to_command(&mut cmd, &env_vars);
        
        // 验证父进程环境变量未被修改
        assert!(env::var("TEST_KEY").is_err());
    }
}
```

---

## 8. 安全性考虑

### 8.1 文件权限
- `provider.json` 设置为 `0600` (仅用户可读写)
- 配置目录 `.agentic-warden` 设置为 `0700`

### 8.2 密钥保护
- 显示配置时自动隐藏包含 KEY/SECRET/TOKEN/PASSWORD 的变量值
- 仅显示前4位和后4位字符

### 8.3 环境变量隔离
- 使用 `Command::env()` 注入到子进程
- 不使用 `std::env::set_var()` 修改父进程环境

---

## 9. 性能优化

### 9.1 配置缓存
- ProviderManager 实例化时加载配置,避免重复读取文件

### 9.2 延迟验证
- 仅在实际启动 AI CLI 时才验证兼容性

---

## 10. 未来扩展

### 10.1 Provider 模板
预定义常见 provider 模板:
```bash
$ agentic-warden provider add-from-template litellm
```

### 10.2 环境变量继承
支持从当前环境继承变量:
```json
{
  "env": {
    "OPENAI_API_KEY": "${OPENAI_API_KEY}"
  }
}
```

### 10.3 Provider 测试
```bash
$ agentic-warden provider test openrouter
🧪 Testing provider: openrouter
✅ Connection successful
```

---

## 11. 实现检查清单

- [ ] 实现 `src/provider/config.rs` - 数据结构
- [ ] 实现 `src/provider/error.rs` - 错误类型
- [ ] 实现 `src/provider/manager.rs` - ProviderManager
- [ ] 实现 `src/provider/env_injector.rs` - 环境变量注入
- [ ] 实现 `src/provider/commands.rs` - Provider 管理命令
- [ ] 修改 `src/cli_manager.rs` - 集成 provider 参数
- [ ] 添加单元测试
- [ ] 添加集成测试
- [ ] 更新文档
- [ ] 更新 README.md 使用示例
