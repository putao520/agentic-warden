//! Token Validation System
//!
//! This module provides comprehensive token validation capabilities including
//! format validation, connection testing, and detailed information retrieval.

use anyhow::{Result, anyhow};
use serde_json::Value;
use std::time::Duration;

use crate::provider::config::{ModeType, ProvidersConfig};

/// Token validation result
#[derive(Debug, Clone)]
pub struct TokenValidationResult {
    /// Whether the token is valid
    pub is_valid: bool,
    /// Validation status
    pub status: ValidationStatus,
    /// Detailed message
    pub message: String,
    /// Token information (if available)
    pub token_info: Option<TokenInfo>,
    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,
}

/// Token validation status
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationStatus {
    /// Token format is invalid
    InvalidFormat,
    /// Network connection failed
    NetworkError,
    /// Authentication failed (401/403)
    AuthenticationFailed,
    /// Token is valid and working
    Valid,
    /// Rate limited
    RateLimited,
    /// Server error
    ServerError,
    /// Unknown error
    UnknownError,
}

/// Token information extracted from validation
#[derive(Debug, Clone)]
pub struct TokenInfo {
    /// Token owner/organization
    pub owner: Option<String>,
    /// Available models (if any)
    pub available_models: Option<Vec<String>>,
    /// Rate limits (if any)
    pub rate_limits: Option<RateLimitInfo>,
    /// Token expiration (if available)
    pub expires_at: Option<String>,
    /// Quota usage (if available)
    pub quota_usage: Option<QuotaInfo>,
}

/// Rate limit information
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    /// Requests per minute
    pub requests_per_minute: Option<u32>,
    /// Requests per day
    pub requests_per_day: Option<u32>,
    /// Tokens per minute
    pub tokens_per_minute: Option<u32>,
    /// Current usage
    pub current_usage: Option<u32>,
}

/// Quota information
#[derive(Debug, Clone)]
pub struct QuotaInfo {
    /// Total quota
    pub total_quota: Option<f64>,
    /// Used quota
    pub used_quota: Option<f64>,
    /// Remaining quota
    pub remaining_quota: Option<f64>,
    /// Quota period (monthly, daily, etc.)
    pub quota_period: Option<String>,
}

/// Token validator
pub struct TokenValidator {
    client: reqwest::Client,
}

impl TokenValidator {
    /// Create new token validator
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("agentic-warden/1.0 token-validator")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { client }
    }

    /// Validate token format without network request
    pub fn validate_format(mode: &ModeType, token: &str) -> TokenValidationResult {
        let is_valid = Self::check_token_format(mode, token);

        TokenValidationResult {
            is_valid,
            status: if is_valid {
                ValidationStatus::Valid
            } else {
                ValidationStatus::InvalidFormat
            },
            message: if is_valid {
                "Token format is valid".to_string()
            } else {
                Self::get_format_error_message(mode, token)
            },
            token_info: None,
            response_time_ms: None,
        }
    }

    /// Validate token with network request
    pub async fn validate_with_network(
        &self,
        providers_config: &ProvidersConfig,
        template_id: &str,
        mode: &ModeType,
        token: &str,
    ) -> Result<TokenValidationResult> {
        let start_time = std::time::Instant::now();

        // First check format
        let format_result = Self::validate_format(mode, token);
        if !format_result.is_valid {
            return Ok(format_result);
        }

        // Get validation endpoint from provider level
        let validation_endpoint = providers_config
            .get_provider(template_id)
            .and_then(|p| p.validation_endpoint.clone())
            .ok_or_else(|| {
                anyhow!(
                    "No validation endpoint available for provider {}",
                    template_id
                )
            })?;

        // Build request
        let mut request = self.client.get(&validation_endpoint);

        // Add authorization based on mode type
        request = match mode {
            ModeType::ClaudeCodeNative => request
                .header("Authorization", format!("Bearer {}", token))
                .header("anthropic-version", "2023-06-01"),
            ModeType::OpenAICompatible => {
                request.header("Authorization", format!("Bearer {}", token))
            }
            ModeType::GeminiNative => request.header("x-goog-api-key", token),
        };

        // Add common headers
        request = request
            .header("Content-Type", "application/json")
            .header("Accept", "application/json");

        // Execute request
        let response = request.send().await;
        let response_time = start_time.elapsed().as_millis() as u64;

        let result = match response {
            Ok(resp) => {
                let status = resp.status();
                let status_code = status.as_u16();

                match status_code {
                    200..=299 => {
                        // Success - try to extract token info
                        let token_info = self.extract_token_info(resp).await.ok();

                        TokenValidationResult {
                            is_valid: true,
                            status: ValidationStatus::Valid,
                            message: "Token is valid and working".to_string(),
                            token_info,
                            response_time_ms: Some(response_time),
                        }
                    }
                    401 | 403 => TokenValidationResult {
                        is_valid: false,
                        status: ValidationStatus::AuthenticationFailed,
                        message: "Authentication failed - invalid or expired token".to_string(),
                        token_info: None,
                        response_time_ms: Some(response_time),
                    },
                    429 => TokenValidationResult {
                        is_valid: false,
                        status: ValidationStatus::RateLimited,
                        message: "Rate limited - please try again later".to_string(),
                        token_info: None,
                        response_time_ms: Some(response_time),
                    },
                    500..=599 => TokenValidationResult {
                        is_valid: false,
                        status: ValidationStatus::ServerError,
                        message: format!("Server error: {}", status),
                        token_info: None,
                        response_time_ms: Some(response_time),
                    },
                    _ => TokenValidationResult {
                        is_valid: false,
                        status: ValidationStatus::UnknownError,
                        message: format!("Unexpected status code: {}", status),
                        token_info: None,
                        response_time_ms: Some(response_time),
                    },
                }
            }
            Err(e) => TokenValidationResult {
                is_valid: false,
                status: ValidationStatus::NetworkError,
                message: format!("Network error: {}", e),
                token_info: None,
                response_time_ms: Some(response_time),
            },
        };

        Ok(result)
    }

    /// Check token format based on mode
    fn check_token_format(mode: &ModeType, token: &str) -> bool {
        let trimmed_token = token.trim();

        match mode {
            ModeType::ClaudeCodeNative => {
                // Claude Code tokens typically start with sk-ant-
                trimmed_token.starts_with("sk-ant-") && trimmed_token.len() > 20
            }
            ModeType::OpenAICompatible => {
                // OpenAI compatible tokens can have various prefixes
                let valid_prefixes = ["sk-", "sk-or-v1-", "sk-proj-"];
                let has_valid_prefix = valid_prefixes
                    .iter()
                    .any(|prefix| trimmed_token.starts_with(prefix));
                has_valid_prefix && trimmed_token.len() > 20
            }
            ModeType::GeminiNative => {
                // Gemini tokens are typically longer strings, may not have obvious prefix
                trimmed_token.len() > 30
            }
        }
    }

    /// Get format error message
    fn get_format_error_message(mode: &ModeType, token: &str) -> String {
        let trimmed_token = token.trim();

        match mode {
            ModeType::ClaudeCodeNative => {
                if !trimmed_token.starts_with("sk-ant-") {
                    "Claude Code tokens should start with 'sk-ant-'".to_string()
                } else if trimmed_token.len() <= 20 {
                    "Token appears to be too short".to_string()
                } else {
                    "Invalid token format for Claude Code".to_string()
                }
            }
            ModeType::OpenAICompatible => {
                let valid_prefixes = ["sk-", "sk-or-v1-", "sk-proj-"];
                if !valid_prefixes
                    .iter()
                    .any(|prefix| trimmed_token.starts_with(prefix))
                {
                    format!(
                        "Token should start with one of: {}",
                        valid_prefixes.join(", ")
                    )
                } else if trimmed_token.len() <= 20 {
                    "Token appears to be too short".to_string()
                } else {
                    "Invalid token format for OpenAI compatible API".to_string()
                }
            }
            ModeType::GeminiNative => {
                if trimmed_token.len() <= 30 {
                    "Token appears to be too short for Gemini API".to_string()
                } else {
                    "Invalid token format for Gemini API".to_string()
                }
            }
        }
    }

    /// Extract token information from successful response
    async fn extract_token_info(&self, response: reqwest::Response) -> Result<TokenInfo> {
        // Extract headers before consuming response
        let rate_limits = self.extract_rate_limits_from_headers(response.headers());

        let body: Value = response.json().await?;

        // Try to extract model information
        let available_models = self.extract_models_from_response(&body);

        // Try to extract quota information
        let quota_usage = self.extract_quota_from_response(&body);

        Ok(TokenInfo {
            owner: self.extract_owner_from_response(&body),
            available_models,
            rate_limits,
            expires_at: None, // Most APIs don't expose this
            quota_usage,
        })
    }

    /// Extract available models from response
    fn extract_models_from_response(&self, body: &Value) -> Option<Vec<String>> {
        // Try different response formats
        if let Some(data) = body.get("data").and_then(|d| d.as_array()) {
            // OpenAI-style format
            let models: Vec<String> = data
                .iter()
                .filter_map(|item| item.get("id").and_then(|id| id.as_str()))
                .map(|s| s.to_string())
                .collect();

            if !models.is_empty() {
                return Some(models);
            }
        }

        // Try direct models array
        if let Some(models) = body.get("models").and_then(|m| m.as_array()) {
            let model_list: Vec<String> = models
                .iter()
                .filter_map(|m| m.as_str())
                .map(|s| s.to_string())
                .collect();

            if !model_list.is_empty() {
                return Some(model_list);
            }
        }

        None
    }

    /// Extract rate limits from response headers
    fn extract_rate_limits_from_headers(
        &self,
        headers: &reqwest::header::HeaderMap,
    ) -> Option<RateLimitInfo> {
        let mut info = RateLimitInfo {
            requests_per_minute: None,
            requests_per_day: None,
            tokens_per_minute: None,
            current_usage: None,
        };

        // Check for common rate limit headers
        if let Some(rpm) = headers.get("x-ratelimit-limit-requests-per-minute") {
            if let Ok(rpm_str) = rpm.to_str() {
                if let Ok(rpm_val) = rpm_str.parse::<u32>() {
                    info.requests_per_minute = Some(rpm_val);
                }
            }
        }

        if let Some(rpd) = headers.get("x-ratelimit-limit-requests-per-day") {
            if let Ok(rpd_str) = rpd.to_str() {
                if let Ok(rpd_val) = rpd_str.parse::<u32>() {
                    info.requests_per_day = Some(rpd_val);
                }
            }
        }

        if let Some(usage) = headers.get("x-ratelimit-remaining") {
            if let Ok(usage_str) = usage.to_str() {
                if let Ok(usage_val) = usage_str.parse::<u32>() {
                    info.current_usage = Some(usage_val);
                }
            }
        }

        // Return Some if we have any information
        if info.requests_per_minute.is_some()
            || info.requests_per_day.is_some()
            || info.current_usage.is_some()
        {
            Some(info)
        } else {
            None
        }
    }

    /// Extract owner information from response
    fn extract_owner_from_response(&self, body: &Value) -> Option<String> {
        // Try different possible owner fields
        let owner_fields = ["organization", "owner", "user", "account"];

        for field in &owner_fields {
            if let Some(owner) = body.get(field).and_then(|o| o.as_str()) {
                return Some(owner.to_string());
            }
        }

        None
    }

    /// Extract quota information from response
    fn extract_quota_from_response(&self, body: &Value) -> Option<QuotaInfo> {
        // This is highly API-specific, so we'll implement a basic version
        let mut info = QuotaInfo {
            total_quota: None,
            used_quota: None,
            remaining_quota: None,
            quota_period: None,
        };

        if let Some(usage) = body.get("usage").and_then(|u| u.as_object()) {
            if let Some(total) = usage.get("total").and_then(|t| t.as_f64()) {
                info.total_quota = Some(total);
            }
            if let Some(used) = usage.get("used").and_then(|u| u.as_f64()) {
                info.used_quota = Some(used);
            }
            if let Some(remaining) = usage.get("remaining").and_then(|r| r.as_f64()) {
                info.remaining_quota = Some(remaining);
            }
            if let Some(period) = usage.get("period").and_then(|p| p.as_str()) {
                info.quota_period = Some(period.to_string());
            }
        }

        // Return Some if we have any quota information
        if info.total_quota.is_some() || info.used_quota.is_some() || info.remaining_quota.is_some()
        {
            Some(info)
        } else {
            None
        }
    }

    /// Get validation status emoji for display
    pub fn get_status_emoji(status: &ValidationStatus) -> &'static str {
        match status {
            ValidationStatus::Valid => "✅",
            ValidationStatus::InvalidFormat => "❌",
            ValidationStatus::NetworkError => "🌐",
            ValidationStatus::AuthenticationFailed => "🔐",
            ValidationStatus::RateLimited => "⏱️",
            ValidationStatus::ServerError => "🔧",
            ValidationStatus::UnknownError => "❓",
        }
    }

    /// Get user-friendly status description
    pub fn get_status_description(status: &ValidationStatus) -> &'static str {
        match status {
            ValidationStatus::Valid => "Token 有效且可用",
            ValidationStatus::InvalidFormat => "Token 格式错误",
            ValidationStatus::NetworkError => "网络连接失败",
            ValidationStatus::AuthenticationFailed => "认证失败，Token 无效或已过期",
            ValidationStatus::RateLimited => "请求频率限制，请稍后重试",
            ValidationStatus::ServerError => "服务器错误",
            ValidationStatus::UnknownError => "未知错误",
        }
    }
}

impl Default for TokenValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_format_validation() {
        // Claude Code format
        let result = TokenValidator::validate_format(
            &ModeType::ClaudeCodeNative,
            "sk-ant-api03-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        );
        assert!(result.is_valid);

        let result = TokenValidator::validate_format(&ModeType::ClaudeCodeNative, "invalid-token");
        assert!(!result.is_valid);

        // OpenAI compatible format
        let result = TokenValidator::validate_format(
            &ModeType::OpenAICompatible,
            "sk-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        );
        assert!(result.is_valid);

        let result = TokenValidator::validate_format(
            &ModeType::OpenAICompatible,
            "sk-or-v1-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        );
        assert!(result.is_valid);
    }

    #[test]
    fn test_status_emoji() {
        assert_eq!(
            TokenValidator::get_status_emoji(&ValidationStatus::Valid),
            "✅"
        );
        assert_eq!(
            TokenValidator::get_status_emoji(&ValidationStatus::InvalidFormat),
            "❌"
        );
        assert_eq!(
            TokenValidator::get_status_emoji(&ValidationStatus::NetworkError),
            "🌐"
        );
    }

    #[test]
    fn test_status_description() {
        assert_eq!(
            TokenValidator::get_status_description(&ValidationStatus::Valid),
            "Token 有效且可用"
        );
        assert_eq!(
            TokenValidator::get_status_description(&ValidationStatus::InvalidFormat),
            "Token 格式错误"
        );
    }
}
