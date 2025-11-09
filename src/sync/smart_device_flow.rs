/// Smart Device Flow Authenticator
///
/// 提供高级Device Flow认证接口，适用于TUI层
/// 对应SPEC/MODULES.md:445-460的smart_device_flow.rs职责

use crate::sync::device_flow_client::{
    AuthConfig, DeviceCodeResponse, DeviceFlowClient, PollError, TokenInfo,
};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time;
use tracing::{debug, info, warn};

/// 认证状态（适用于TUI）
///
/// 对应SPEC/DATA_MODEL.md:718-736的AuthStatus枚举
/// 扩展了原有AuthState以支持Device Flow特性
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuthState {
    /// 初始化中
    Initializing,

    /// 显示设备代码，等待用户访问 URL 并授权
    WaitingForUser {
        verification_url: String,
        user_code: String,
        expires_at: DateTime<Utc>,
    },

    /// 轮询检查授权状态
    Polling {
        verification_url: String,
        user_code: String,
        expires_at: DateTime<Utc>,
    },

    /// 授权成功
    Authenticated {
        access_token: String,
        refresh_token: Option<String>,
        expires_at: Option<DateTime<Utc>>,
    },

    /// 授权失败
    Failed { message: String },

    /// 设备代码过期
    Expired,

    /// 用户拒绝授权
    AccessDenied,
}

impl AuthState {
    fn with_error<E: std::fmt::Display>(err: E) -> Self {
        Self::Failed {
            message: err.to_string(),
        }
    }
}

/// 授权对话框状态
///
/// 对应SPEC/DATA_MODEL.md:697-715
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthDialogState {
    /// 验证 URL (https://google.com/device)
    pub verification_url: String,

    /// 当前授权状态
    pub auth_status: AuthStatus,

    /// 用户代码 (如 ABCD-EFGH)
    pub user_code: String,

    /// 过期倒计时（秒）
    pub expires_in: u64,

    /// 轮询间隔（秒）
    pub poll_interval: u64,

    /// 错误信息
    pub error_message: Option<String>,
}

/// 授权状态（简化版，用于AuthDialogState）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuthStatus {
    WaitingForUser,
    Polling,
    Authorized,
    Failed(String),
    Expired,
    AccessDenied,
}

/// 内部状态
struct SmartDeviceFlowInner {
    client: DeviceFlowClient,
    state: RwLock<AuthState>,
    auth_file_path: PathBuf,
}

/// Smart Device Flow 认证器
///
/// 封装 DeviceFlowClient，提供高级认证接口
#[derive(Clone)]
pub struct SmartDeviceFlowAuthenticator {
    inner: Arc<SmartDeviceFlowInner>,
}

impl SmartDeviceFlowAuthenticator {
    /// 创建新的认证器
    pub fn new(config: AuthConfig, auth_file_path: PathBuf) -> Self {
        let client = DeviceFlowClient::from_config(&config);

        Self {
            inner: Arc::new(SmartDeviceFlowInner {
                client,
                state: RwLock::new(AuthState::Initializing),
                auth_file_path,
            }),
        }
    }

    /// 从现有令牌创建认证器
    pub fn with_existing_tokens(
        config: AuthConfig,
        token_info: TokenInfo,
        auth_file_path: PathBuf,
    ) -> Self {
        let client = DeviceFlowClient::from_config(&config);

        let expires_at = if token_info.expires_in > 0 {
            Some(Utc::now() + Duration::seconds(token_info.expires_in as i64))
        } else {
            None
        };

        let initial_state = AuthState::Authenticated {
            access_token: token_info.access_token,
            refresh_token: token_info.refresh_token,
            expires_at,
        };

        Self {
            inner: Arc::new(SmartDeviceFlowInner {
                client,
                state: RwLock::new(initial_state),
                auth_file_path,
            }),
        }
    }

    /// 启动 Device Flow 认证流程
    ///
    /// 返回设备代码响应，包含用户需要访问的URL和输入的代码
    pub async fn start_device_flow(&self) -> Result<DeviceCodeResponse> {
        debug!("Starting Device Flow authentication");

        // 启动Device Flow
        let device_response = self.inner.client.start_device_flow().await?;

        let expires_at = Utc::now() + Duration::seconds(device_response.expires_in as i64);

        // 更新状态为等待用户授权
        {
            let mut state = self.inner.state.write().await;
            *state = AuthState::WaitingForUser {
                verification_url: device_response.verification_url.clone(),
                user_code: device_response.user_code.clone(),
                expires_at,
            };
        }

        info!(
            "Device flow started. User code: {}, Verification URL: {}",
            device_response.user_code, device_response.verification_url
        );

        Ok(device_response)
    }

    /// 轮询授权状态直到成功或失败
    ///
    /// 自动处理轮询间隔和重试逻辑
    pub async fn poll_until_authorized(
        &self,
        device_code: &str,
        poll_interval: u64,
        timeout_seconds: u64,
    ) -> Result<TokenInfo> {
        debug!("Starting to poll for authorization");

        let start_time = Utc::now();
        let timeout = Duration::seconds(timeout_seconds as i64);
        let mut current_interval = time::Duration::from_secs(poll_interval);

        loop {
            // 检查超时
            if Utc::now() - start_time > timeout {
                let mut state = self.inner.state.write().await;
                *state = AuthState::Expired;
                return Err(anyhow!("Device code expired (timeout)"));
            }

            // 等待轮询间隔
            time::sleep(current_interval).await;

            // 轮询授权状态
            match self.inner.client.poll_for_token(device_code).await {
                Ok(token_info) => {
                    // 授权成功
                    info!("Authorization successful");

                    let expires_at = if token_info.expires_in > 0 {
                        Some(Utc::now() + Duration::seconds(token_info.expires_in as i64))
                    } else {
                        None
                    };

                    // 更新状态
                    {
                        let mut state = self.inner.state.write().await;
                        *state = AuthState::Authenticated {
                            access_token: token_info.access_token.clone(),
                            refresh_token: token_info.refresh_token.clone(),
                            expires_at,
                        };
                    }

                    // 保存令牌到文件
                    self.save_tokens(&token_info).await?;

                    return Ok(token_info);
                }
                Err(poll_error) => match poll_error {
                    PollError::AuthorizationPending => {
                        // 继续等待
                        debug!("Authorization still pending, continuing to poll");
                        // 更新状态为轮询中
                        if let AuthState::WaitingForUser {
                            verification_url,
                            user_code,
                            expires_at,
                        } = &*self.inner.state.read().await
                        {
                            let mut state = self.inner.state.write().await;
                            *state = AuthState::Polling {
                                verification_url: verification_url.clone(),
                                user_code: user_code.clone(),
                                expires_at: *expires_at,
                            };
                        }
                        continue;
                    }
                    PollError::SlowDown => {
                        // 轮询过快，增加间隔
                        warn!("Polling too fast, slowing down");
                        current_interval += time::Duration::from_secs(poll_interval);
                        continue;
                    }
                    PollError::AccessDenied => {
                        let mut state = self.inner.state.write().await;
                        *state = AuthState::AccessDenied;
                        return Err(anyhow!("User denied authorization"));
                    }
                    PollError::ExpiredToken => {
                        let mut state = self.inner.state.write().await;
                        *state = AuthState::Expired;
                        return Err(anyhow!("Device code expired"));
                    }
                    PollError::Other(msg) => {
                        let mut state = self.inner.state.write().await;
                        *state = AuthState::with_error(&msg);
                        return Err(anyhow!("Authorization failed: {}", msg));
                    }
                },
            }
        }
    }

    /// 刷新访问令牌
    pub async fn refresh_access_token(&self) -> Result<TokenInfo> {
        let refresh_token = {
            let state = self.inner.state.read().await;
            match &*state {
                AuthState::Authenticated { refresh_token, .. } => {
                    refresh_token.clone().ok_or_else(|| anyhow!("No refresh token available"))?
                }
                _ => return Err(anyhow!("Not authenticated")),
            }
        };

        let token_info = self.inner.client.refresh_access_token(&refresh_token).await?;

        let expires_at = if token_info.expires_in > 0 {
            Some(Utc::now() + Duration::seconds(token_info.expires_in as i64))
        } else {
            None
        };

        // 更新状态
        {
            let mut state = self.inner.state.write().await;
            *state = AuthState::Authenticated {
                access_token: token_info.access_token.clone(),
                refresh_token: token_info.refresh_token.clone().or(Some(refresh_token)),
                expires_at,
            };
        }

        // 保存令牌
        self.save_tokens(&token_info).await?;

        Ok(token_info)
    }

    /// 获取当前状态
    pub async fn get_state(&self) -> AuthState {
        self.inner.state.read().await.clone()
    }

    /// 获取访问令牌（如果已认证）
    pub async fn get_access_token(&self) -> Option<String> {
        let state = self.inner.state.read().await;
        match &*state {
            AuthState::Authenticated { access_token, .. } => Some(access_token.clone()),
            _ => None,
        }
    }

    /// 检查是否已认证
    pub async fn is_authenticated(&self) -> bool {
        matches!(*self.inner.state.read().await, AuthState::Authenticated { .. })
    }

    /// 保存令牌到文件
    async fn save_tokens(&self, token_info: &TokenInfo) -> Result<()> {
        let json = serde_json::to_string_pretty(token_info)
            .context("Failed to serialize token info")?;

        tokio::fs::write(&self.inner.auth_file_path, json)
            .await
            .context("Failed to write auth file")?;

        debug!("Tokens saved to {:?}", self.inner.auth_file_path);
        Ok(())
    }

    /// 从文件加载令牌
    pub async fn load_tokens_from_file(auth_file_path: &Path) -> Result<TokenInfo> {
        let content = tokio::fs::read_to_string(auth_file_path)
            .await
            .context("Failed to read auth file")?;

        let token_info: TokenInfo = serde_json::from_str(&content)
            .context("Failed to parse auth file")?;

        Ok(token_info)
    }
}

impl Default for SmartDeviceFlowAuthenticator {
    fn default() -> Self {
        Self::new(
            AuthConfig::default(),
            PathBuf::from(".agentic-warden/auth.json"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_state_error() {
        let state = AuthState::with_error("Test error");
        assert!(matches!(state, AuthState::Failed { .. }));
    }

    #[test]
    fn test_auth_state_equality() {
        let state1 = AuthState::Initializing;
        let state2 = AuthState::Initializing;
        assert_eq!(state1, state2);

        let state3 = AuthState::Expired;
        let state4 = AuthState::Expired;
        assert_eq!(state3, state4);
    }

    #[tokio::test]
    async fn test_authenticator_creation() {
        let config = AuthConfig::default();
        let auth = SmartDeviceFlowAuthenticator::new(
            config,
            PathBuf::from("/tmp/test_auth.json"),
        );

        let state = auth.get_state().await;
        assert!(matches!(state, AuthState::Initializing));
    }

    #[tokio::test]
    async fn test_is_authenticated() {
        let config = AuthConfig::default();
        let auth = SmartDeviceFlowAuthenticator::new(
            config,
            PathBuf::from("/tmp/test_auth.json"),
        );

        assert!(!auth.is_authenticated().await);
    }
}
