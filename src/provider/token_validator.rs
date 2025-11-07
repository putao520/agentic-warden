//! Token validation helper with lightweight network awareness.
//!
//! Provides fast heuristics to ensure the entered API token matches the
//! expected provider format while also surfacing potential network issues.

use super::config::{ModeType, Provider, ProvidersConfig, Region, SupportMode};
use super::network_detector::{ensure_status, NetworkDetector, NetworkStatus};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Central token validator.
pub struct TokenValidator {
    cache: HashMap<String, TokenValidationResult>,
    detector: NetworkDetector,
}

/// Result returned to the caller.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenValidationResult {
    pub status: TokenValidationStatus,
    pub message: String,
    pub warnings: Vec<String>,
    pub region: Option<Region>,
    pub endpoint: Option<String>,
    pub latency_ms: Option<u128>,
}

impl TokenValidationResult {
    pub fn is_valid(&self) -> bool {
        matches!(self.status, TokenValidationStatus::Valid)
    }

    pub fn failure(
        status: TokenValidationStatus,
        message: impl Into<String>,
        region: Option<Region>,
    ) -> Self {
        Self {
            status,
            message: message.into(),
            warnings: Vec::new(),
            region,
            endpoint: None,
            latency_ms: None,
        }
    }

    fn with_warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings = warnings;
        self
    }
}

/// High-level validation status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TokenValidationStatus {
    Valid,
    Missing,
    InvalidFormat,
    NetworkUnreachable,
    UnsupportedProvider,
}

impl TokenValidator {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            detector: NetworkDetector::new(),
        }
    }

    /// Validate a token for the specific provider/mode using heuristics and network context.
    pub async fn validate_with_network(
        &mut self,
        config: &ProvidersConfig,
        provider_id: &str,
        mode_type: &ModeType,
        token: &str,
    ) -> Result<TokenValidationResult> {
        let token = token.trim();
        if token.is_empty() {
            return Ok(TokenValidationResult::failure(
                TokenValidationStatus::Missing,
                "Token 不能为空",
                None,
            ));
        }

        let provider = config
            .get_provider(provider_id)
            .ok_or_else(|| anyhow!("Provider '{}' not found", provider_id))?;
        let Some(mode) = find_mode(provider, mode_type) else {
            return Ok(TokenValidationResult::failure(
                TokenValidationStatus::UnsupportedProvider,
                "该 Provider 不支持所选模式",
                None,
            ));
        };

        let network_status = ensure_status(&self.detector).await;
        let preferred_region = preferred_region_from_status(&network_status)
            .or_else(|| mode.config.regional_urls.keys().next().cloned());
        let Some((region, regional_config)) = pick_region(mode, preferred_region.as_ref()) else {
            return Ok(TokenValidationResult::failure(
                TokenValidationStatus::UnsupportedProvider,
                "该 Provider 未提供可用的区域配置",
                preferred_region,
            ));
        };

        if !matches_expected_format(&regional_config.auth_env_var, token) {
            return Ok(TokenValidationResult::failure(
                TokenValidationStatus::InvalidFormat,
                format!("Token 格式与 {} 不匹配", regional_config.auth_env_var),
                Some(region),
            )
            .with_warnings(vec!["请确认复制完整的 API Key".to_string()]));
        }

        let endpoint = provider
            .validation_endpoint
            .clone()
            .or_else(|| Some(regional_config.base_url.clone()));
        let mut warnings = build_warnings(
            &network_status,
            &region,
            config.has_token(provider_id, &region),
        );

        if provider.custom {
            warnings.push("自定义 Provider，无法自动校验服务端状态".to_string());
        }

        let result = TokenValidationResult {
            status: TokenValidationStatus::Valid,
            message: format!("格式校验通过 · 将使用{}线路", region_label(&region)),
            warnings,
            region: Some(region.clone()),
            endpoint,
            latency_ms: None,
        };

        self.cache.insert(token.to_string(), result.clone());

        Ok(result)
    }

    /// Return the last cached validation result for the token if available.
    pub fn cached_result(&self, token: &str) -> Option<&TokenValidationResult> {
        self.cache.get(token)
    }
}

fn find_mode<'a>(provider: &'a Provider, mode_type: &ModeType) -> Option<&'a SupportMode> {
    provider
        .support_modes
        .iter()
        .find(|mode| &mode.mode_type == mode_type)
        .or_else(|| provider.support_modes.iter().next())
}

fn preferred_region_from_status(status: &NetworkStatus) -> Option<Region> {
    match status.prefer_domestic() {
        Some(true) => Some(Region::MainlandChina),
        Some(false) => Some(Region::International),
        None => None,
    }
}

fn pick_region<'a>(
    mode: &'a SupportMode,
    preferred_region: Option<&Region>,
) -> Option<(Region, &'a crate::provider::config::RegionalConfig)> {
    if let Some(region) = preferred_region {
        if let Some(cfg) = mode.config.regional_urls.get(region) {
            return Some((region.clone(), cfg));
        }
    }

    mode.config
        .regional_urls
        .iter()
        .next()
        .map(|(region, cfg)| (region.clone(), cfg))
}

fn matches_expected_format(env_var: &str, token: &str) -> bool {
    let normalized = env_var.to_ascii_uppercase();
    let token = token.trim();

    if token.len() < 8 {
        return false;
    }

    if normalized.contains("OPENAI") {
        return token.starts_with("sk-");
    }

    if normalized.contains("ANTHROPIC") {
        return token.starts_with("sk-ant-")
            || token.starts_with("anthropic-")
            || token.starts_with("live_")
            || token.len() >= 24;
    }

    if normalized.contains("GOOGLE") {
        return token.starts_with("AIza") || token.len() >= 30;
    }

    true
}

fn region_label(region: &Region) -> &'static str {
    match region {
        Region::MainlandChina => "国内",
        Region::International => "国际",
    }
}

fn build_warnings(status: &NetworkStatus, region: &Region, has_token: bool) -> Vec<String> {
    let mut warnings = Vec::new();

    if matches!(region, Region::International) && status.should_warn_international() {
        warnings.push("当前国际网络不稳定，可能需要代理".to_string());
    } else if matches!(region, Region::MainlandChina) && status.should_warn_domestic() {
        warnings.push("当前国内网络不稳定".to_string());
    }

    if !has_token {
        warnings.push("尚未保存 Token".to_string());
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::config::ProvidersConfig;

    #[tokio::test]
    async fn rejects_empty_token() {
        let config = ProvidersConfig::create_default().unwrap();
        let mut validator = TokenValidator::new();
        let result = validator
            .validate_with_network(&config, "openrouter", &ModeType::OpenAICompatible, "")
            .await
            .unwrap();
        assert_eq!(result.status, TokenValidationStatus::Missing);
    }

    #[tokio::test]
    async fn accepts_reasonable_openai_token() {
        let config = ProvidersConfig::create_default().unwrap();
        let mut validator = TokenValidator::new();
        let result = validator
            .validate_with_network(
                &config,
                "openrouter",
                &ModeType::OpenAICompatible,
                "sk-test-1234567890",
            )
            .await
            .unwrap();
        assert!(result.is_valid());
        assert!(result.region.is_some());
    }
}
