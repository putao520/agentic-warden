/// Google OAuth 2.0 Device Flow Client (RFC 8628)
///
/// 符合SPEC/ARCHITECTURE.md:305-338定义
/// 用于CLI工具的现代化认证方式，替代已废弃的OOB flow

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Device Flow 认证配置
///
/// 对应SPEC/DATA_MODEL.md:123-140
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Google OAuth 客户端 ID
    pub client_id: String,

    /// Google OAuth 客户端密钥
    pub client_secret: String,

    /// OAuth 作用域
    pub scopes: Vec<String>,

    /// 授权超时时间（秒）
    #[serde(default = "default_auth_timeout")]
    pub auth_timeout: u64,

    /// Device Flow 轮询间隔（秒）
    #[serde(default = "default_poll_interval")]
    pub poll_interval: u64,
}

fn default_auth_timeout() -> u64 {
    300 // 5 分钟
}

fn default_poll_interval() -> u64 {
    5 // Google 建议 5 秒间隔
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            scopes: vec![
                "https://www.googleapis.com/auth/drive.file".to_string(),
            ],
            auth_timeout: default_auth_timeout(),
            poll_interval: default_poll_interval(),
        }
    }
}

/// Device Code 响应
///
/// 对应SPEC/DATA_MODEL.md:154-169
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    /// 内部使用的设备代码（用于轮询）
    pub device_code: String,

    /// 用户输入的代码 (如 ABCD-EFGH)
    pub user_code: String,

    /// 验证 URL (如 https://google.com/device)
    pub verification_url: String,

    /// 完整的验证 URL（包含用户代码，可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_url_complete: Option<String>,

    /// 过期时间（秒）
    pub expires_in: u64,

    /// 轮询间隔（秒）
    pub interval: u64,
}

/// 令牌信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    /// 访问令牌
    pub access_token: String,

    /// 刷新令牌（可选）
    pub refresh_token: Option<String>,

    /// 过期时间（秒）
    pub expires_in: u64,

    /// 令牌类型（通常为 "Bearer"）
    pub token_type: String,

    /// 作用域
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// Device Flow 轮询错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum PollError {
    /// 授权待处理（需要继续轮询）
    AuthorizationPending,

    /// 轮询过快（应减慢速度）
    SlowDown,

    /// 访问被拒绝
    AccessDenied,

    /// 设备代码已过期
    ExpiredToken,

    /// 其他错误
    Other(String),
}

/// Device Flow 客户端
///
/// 对应SPEC/ARCHITECTURE.md:306-320
pub struct DeviceFlowClient {
    client_id: String,
    client_secret: String,
    http_client: reqwest::Client,
}

impl DeviceFlowClient {
    /// 创建新的 Device Flow 客户端
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            client_secret,
            http_client: reqwest::Client::new(),
        }
    }

    /// 从 AuthConfig 创建
    pub fn from_config(config: &AuthConfig) -> Self {
        Self::new(
            config.client_id.clone(),
            config.client_secret.clone(),
        )
    }

    /// 启动 Device Flow，返回设备代码和验证 URL
    ///
    /// 对应SPEC/ARCHITECTURE.md:313
    pub async fn start_device_flow(&self) -> Result<DeviceCodeResponse> {
        debug!("Starting Device Flow authentication");

        let params = [
            ("client_id", self.client_id.as_str()),
            ("scope", "https://www.googleapis.com/auth/drive.file"),
        ];

        let response = self
            .http_client
            .post("https://oauth2.googleapis.com/device/code")
            .form(&params)
            .send()
            .await
            .context("Failed to initiate device flow")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Device flow initiation failed: {}",
                error_text
            ));
        }

        let device_code_response: DeviceCodeResponse = response
            .json()
            .await
            .context("Failed to parse device code response")?;

        info!(
            "Device flow started. User code: {}, Verification URL: {}",
            device_code_response.user_code, device_code_response.verification_url
        );

        Ok(device_code_response)
    }

    /// 轮询检查授权状态
    ///
    /// 对应SPEC/ARCHITECTURE.md:316
    ///
    /// 返回:
    /// - Ok(TokenInfo) - 授权成功
    /// - Err(PollError::AuthorizationPending) - 需要继续等待
    /// - Err(PollError::SlowDown) - 轮询过快
    /// - Err(其他) - 授权失败
    pub async fn poll_for_token(&self, device_code: &str) -> Result<TokenInfo, PollError> {
        debug!("Polling for authorization token");

        let params = [
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ];

        let response = self
            .http_client
            .post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await
            .map_err(|e| PollError::Other(format!("Network error: {}", e)))?;

        let status = response.status();
        let body_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read response".to_string());

        if status.is_success() {
            // 授权成功
            let token_info: TokenInfo = serde_json::from_str(&body_text)
                .map_err(|e| PollError::Other(format!("Failed to parse token response: {}", e)))?;

            info!("Authorization successful");
            return Ok(token_info);
        }

        // 解析错误响应
        #[derive(Deserialize)]
        struct ErrorResponse {
            error: String,
            #[serde(default)]
            error_description: String,
        }

        let error_response: ErrorResponse = serde_json::from_str(&body_text)
            .map_err(|_| PollError::Other(format!("Failed to parse error response: {}", body_text)))?;

        match error_response.error.as_str() {
            "authorization_pending" => {
                debug!("Authorization still pending");
                Err(PollError::AuthorizationPending)
            }
            "slow_down" => {
                warn!("Polling too fast, slowing down");
                Err(PollError::SlowDown)
            }
            "access_denied" => {
                info!("User denied authorization");
                Err(PollError::AccessDenied)
            }
            "expired_token" => {
                info!("Device code expired");
                Err(PollError::ExpiredToken)
            }
            _ => Err(PollError::Other(format!(
                "{}: {}",
                error_response.error, error_response.error_description
            ))),
        }
    }

    /// 刷新访问令牌
    ///
    /// 对应SPEC/ARCHITECTURE.md:319
    pub async fn refresh_access_token(&self, refresh_token: &str) -> Result<TokenInfo> {
        debug!("Refreshing access token");

        let params = [
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];

        let response = self
            .http_client
            .post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await
            .context("Failed to refresh token")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Token refresh failed: {}", error_text));
        }

        let token_info: TokenInfo = response
            .json()
            .await
            .context("Failed to parse token refresh response")?;

        info!("Access token refreshed successfully");

        Ok(token_info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_config_defaults() {
        let config = AuthConfig::default();
        assert_eq!(config.auth_timeout, 300);
        assert_eq!(config.poll_interval, 5);
        assert!(!config.scopes.is_empty());
    }

    #[test]
    fn test_device_flow_client_creation() {
        let client = DeviceFlowClient::new(
            "test-client-id".to_string(),
            "test-client-secret".to_string(),
        );
        assert_eq!(client.client_id, "test-client-id");
        assert_eq!(client.client_secret, "test-client-secret");
    }

    #[test]
    fn test_poll_error_types() {
        assert_eq!(PollError::AuthorizationPending, PollError::AuthorizationPending);
        assert_eq!(PollError::SlowDown, PollError::SlowDown);
        assert_eq!(PollError::AccessDenied, PollError::AccessDenied);
        assert_eq!(PollError::ExpiredToken, PollError::ExpiredToken);
    }
}
