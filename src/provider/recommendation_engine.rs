//! Intelligent Provider Recommendation Engine
//!
//! This module provides smart recommendation capabilities for AI providers
//! based on network environment, CLI type, and user preferences.

use anyhow::Result;

use crate::provider::{
    config::{AiType, ModeType, Provider, ProvidersConfig, RecommendationScenario, Region},
    network_detector::{NetworkDetector, NetworkStatus},
};

/// Recommendation result with provider and reasoning (Enhanced for v2.0)
#[derive(Debug, Clone)]
pub struct Recommendation {
    /// Provider ID
    pub provider_id: String,
    /// Provider information
    pub provider: Provider,
    /// Recommended support mode
    pub support_mode: crate::provider::config::SupportMode,
    /// Recommendation score (higher = better)
    pub score: u8,
    /// Reason for recommendation
    pub reason: String,
    /// Warnings or considerations
    pub warnings: Vec<String>,
    /// Recommended region
    pub recommended_region: Region,
    /// Has token configured
    pub has_token: bool,
}

/// Smart recommendation engine
#[derive(Debug, Clone)]
pub struct RecommendationEngine {
    network_detector: NetworkDetector,
}

impl RecommendationEngine {
    /// Create new recommendation engine
    pub fn new() -> Self {
        Self {
            network_detector: NetworkDetector::new(),
        }
    }

    /// Get intelligent recommendations based on context
    pub async fn get_recommendations(
        &self,
        providers_config: &ProvidersConfig,
        ai_type: &AiType,
        user_preferences: &RecommendationPreferences,
    ) -> Result<Vec<Recommendation>> {
        // Detect network environment
        let network_status = self.network_detector.detect().await?;

        // Determine recommended region
        let recommended_region = if network_status.is_china_mainland() {
            Region::MainlandChina
        } else {
            Region::International
        };

        // Get recommendations for each provider
        let mut recommendations = Vec::new();

        for (provider_id, provider) in &providers_config.providers {
            // Check if provider supports the AI type
            if !provider.compatible_with.contains(ai_type) {
                continue;
            }

            // Check if provider is in excluded list
            if user_preferences.excluded_providers.contains(provider_id) {
                continue;
            }

            for support_mode in &provider.support_modes {
                // Check if mode supports the recommended region
                if !support_mode
                    .config
                    .regional_urls
                    .contains_key(&recommended_region)
                {
                    continue;
                }

                // Calculate recommendation score
                let (final_score, reason, warnings) = self.calculate_recommendation_score(
                    provider_id,
                    provider,
                    support_mode,
                    &network_status,
                    user_preferences,
                    &recommended_region,
                );

                if final_score > 0 {
                    // Check if user has token configured
                    let has_token = providers_config
                        .get_token(provider_id, &recommended_region)
                        .is_some();

                    recommendations.push(Recommendation {
                        provider_id: provider_id.clone(),
                        provider: provider.clone(),
                        support_mode: support_mode.clone(),
                        score: final_score,
                        reason,
                        warnings,
                        recommended_region: recommended_region.clone(),
                        has_token,
                    });
                }
            }
        }

        // Sort by score (descending)
        recommendations.sort_by(|a, b| b.score.cmp(&a.score));

        // Apply max recommendations limit
        if let Some(max) = user_preferences.max_recommendations {
            recommendations.truncate(max);
        }

        Ok(recommendations)
    }

    /// Calculate detailed recommendation score with reasoning
    fn calculate_recommendation_score(
        &self,
        provider_id: &str,
        provider: &Provider,
        support_mode: &crate::provider::config::SupportMode,
        _network_status: &NetworkStatus,
        preferences: &RecommendationPreferences,
        recommended_region: &Region,
    ) -> (u8, String, Vec<String>) {
        let mut final_score = support_mode.priority;
        let mut reason_parts = Vec::new();
        let mut warnings = Vec::new();

        // Region availability bonus
        if recommended_region == &Region::MainlandChina
            && provider.regions.contains(&"CN".to_string())
        {
            final_score += 5;
            reason_parts.push("Has service nodes in mainland China".to_string());
        } else if recommended_region == &Region::International
            && provider.regions.contains(&"US".to_string())
        {
            final_score += 3;
            reason_parts.push("Stable international network connection".to_string());
        }

        // Official provider bonus
        if provider.official {
            final_score += 3;
            reason_parts.push("Official provider".to_string());
        }

        // Mode preference bonus
        if preferences.claude_code_preferred && support_mode.mode_type == ModeType::ClaudeCodeNative
        {
            final_score += 8;
            reason_parts.push("Supports Claude Code dedicated mode".to_string());
        }

        // Category-based bonuses
        if let Some(category) = &provider.category {
            match category.as_str() {
                "Domestic AI" if recommended_region == &Region::MainlandChina => {
                    final_score += 6;
                    reason_parts.push("Optimized for China network".to_string());
                }
                "Open Source AI" if preferences.cost_sensitive => {
                    final_score += 4;
                    reason_parts.push("Open source models, affordable".to_string());
                }
                _ => {}
            }
        }

        // Cost consideration
        if preferences.cost_sensitive {
            match provider_id {
                "deepseek" | "qwen" => {
                    final_score += 4;
                    reason_parts.push("Affordable".to_string());
                }
                _ => {}
            }
        }

        // Performance consideration
        if preferences.high_performance_required {
            match provider_id {
                "kimi" => {
                    final_score += 6;
                    reason_parts.push("Long context, high performance".to_string());
                }
                "glm" => {
                    final_score += 4;
                    reason_parts.push("High performance".to_string());
                }
                _ => {}
            }
        }

        // Network-specific considerations
        if recommended_region == &Region::MainlandChina {
            if !provider.regions.contains(&"CN".to_string()) {
                warnings.push(
                    "International providers may have unstable connections in mainland China"
                        .to_string(),
                );
                final_score = final_score.saturating_sub(10);
            }
        }

        // Claude Code specific considerations
        if preferences.claude_code_preferred {
            if support_mode.mode_type == ModeType::ClaudeCodeNative {
                reason_parts.push("Native Claude Code experience".to_string());
            } else if support_mode.mode_type == ModeType::OpenAICompatible {
                warnings.push(
                    "Using compatibility mode, some Claude Code features may be missing"
                        .to_string(),
                );
                final_score = final_score.saturating_sub(5);
            }
        }

        // Build final reason string
        let reason = if reason_parts.is_empty() {
            "Recommended to use".to_string()
        } else {
            reason_parts.join(", ")
        };

        (final_score, reason, warnings)
    }

    /// Determine recommendation scenario based on context (Legacy)
    fn determine_scenario(
        &self,
        network_status: &NetworkStatus,
        preferences: &RecommendationPreferences,
    ) -> RecommendationScenario {
        // Check if user is in China mainland
        if network_status.is_china_mainland() {
            if preferences.claude_code_preferred {
                RecommendationScenario::ClaudeCodePreferred
            } else if preferences.cost_sensitive {
                RecommendationScenario::CostEffective
            } else {
                RecommendationScenario::ChinaMainland
            }
        } else {
            // International user
            if preferences.claude_code_preferred {
                RecommendationScenario::ClaudeCodePreferred
            } else if preferences.cost_sensitive {
                RecommendationScenario::CostEffective
            } else if preferences.high_performance_required {
                RecommendationScenario::HighPerformance
            } else {
                RecommendationScenario::International
            }
        }
    }

    /// Get best single recommendation
    pub async fn get_best_recommendation(
        &self,
        providers_config: &ProvidersConfig,
        ai_type: &AiType,
        preferences: &RecommendationPreferences,
    ) -> Result<Option<Recommendation>> {
        let recommendations = self
            .get_recommendations(providers_config, ai_type, preferences)
            .await?;
        Ok(recommendations.into_iter().next())
    }

    /// Get providers that support specific mode type
    pub fn get_providers_with_mode_type(
        providers_config: &ProvidersConfig,
        mode_type: &ModeType,
        ai_type: &AiType,
    ) -> Vec<(String, Provider, crate::provider::config::SupportMode)> {
        let mut matching_providers = Vec::new();

        for (provider_id, provider) in &providers_config.providers {
            if !provider.compatible_with.contains(ai_type) {
                continue;
            }

            for support_mode in &provider.support_modes {
                if support_mode.mode_type == *mode_type {
                    matching_providers.push((
                        provider_id.clone(),
                        provider.clone(),
                        support_mode.clone(),
                    ));
                }
            }
        }

        // Sort by priority (descending)
        matching_providers.sort_by(|a, b| b.2.priority.cmp(&a.2.priority));
        matching_providers
    }

    /// Check if provider supports Claude Code natively
    pub fn supports_claude_code_native(provider: &Provider) -> bool {
        provider
            .support_modes
            .iter()
            .any(|mode| mode.mode_type == ModeType::ClaudeCodeNative)
    }

    /// Get fallback providers
    pub async fn get_fallback_recommendations(
        &self,
        providers_config: &ProvidersConfig,
        ai_type: &AiType,
        excluded_provider_ids: &[&str],
        preferences: &RecommendationPreferences,
    ) -> Result<Vec<Recommendation>> {
        let mut fallback_preferences = preferences.clone();
        fallback_preferences
            .excluded_providers
            .extend(excluded_provider_ids.iter().map(|s| s.to_string()));

        self.get_recommendations(providers_config, ai_type, &fallback_preferences)
            .await
    }

    /// Helper method to create a basic provider from provider ID
    fn create_basic_provider(&self, provider_id: &str) -> Provider {
        Provider {
            name: provider_id.to_string(),
            description: format!("Provider: {}", provider_id),
            icon: None,
            official: false,
            protected: false,
            custom: true,
            support_modes: vec![],
            compatible_with: vec![AiType::Claude, AiType::Codex],
            validation_endpoint: None,
            category: None,
            website: None,
            regions: vec![],
            env: std::collections::HashMap::new(),
        }
    }
}

impl Default for RecommendationEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// User preference settings for recommendations
#[derive(Debug, Clone, Default)]
pub struct RecommendationPreferences {
    /// User prefers Claude Code native mode
    pub claude_code_preferred: bool,
    /// User is cost sensitive
    pub cost_sensitive: bool,
    /// User requires high performance
    pub high_performance_required: bool,
    /// Preferred regions
    pub preferred_regions: Vec<String>,
    /// Excluded providers
    pub excluded_providers: Vec<String>,
    /// Maximum number of recommendations to return
    pub max_recommendations: Option<usize>,
}

impl RecommendationPreferences {
    /// Create preferences for Claude Code users in China
    pub fn claude_code_china() -> Self {
        Self {
            claude_code_preferred: true,
            cost_sensitive: false,
            high_performance_required: false,
            preferred_regions: vec!["CN".to_string()],
            excluded_providers: Vec::new(),
            max_recommendations: Some(5),
        }
    }

    /// Create preferences for cost-conscious users
    pub fn cost_focused() -> Self {
        Self {
            claude_code_preferred: false,
            cost_sensitive: true,
            high_performance_required: false,
            preferred_regions: Vec::new(),
            excluded_providers: Vec::new(),
            max_recommendations: Some(3),
        }
    }

    /// Create preferences for performance-focused users
    pub fn performance_focused() -> Self {
        Self {
            claude_code_preferred: false,
            cost_sensitive: false,
            high_performance_required: true,
            preferred_regions: vec!["US".to_string()],
            excluded_providers: Vec::new(),
            max_recommendations: Some(3),
        }
    }

    /// Add excluded provider
    pub fn exclude_provider(mut self, provider_id: &str) -> Self {
        self.excluded_providers.push(provider_id.to_string());
        self
    }

    /// Set preferred region
    pub fn prefer_region(mut self, region: &str) -> Self {
        self.preferred_regions.push(region.to_string());
        self
    }

    /// Set max recommendations
    pub fn max_recommendations(mut self, max: usize) -> Self {
        self.max_recommendations = Some(max);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_recommendation_engine() {
        // Test basic functionality without network dependency
        let engine = RecommendationEngine::new();
        let preferences = RecommendationPreferences::default();

        // Create a test config with some providers
        let providers_config = ProvidersConfig::create_default().unwrap();

        // This test now just verifies the engine doesn't panic and can handle empty configs
        let recommendations = engine
            .get_recommendations(&providers_config, &AiType::Claude, &preferences)
            .await;

        // Should not panic, even if empty recommendations
        assert!(recommendations.is_ok());

        // Empty config should result in empty recommendations
        let recs = recommendations.unwrap();
        assert_eq!(recs.len(), 0);
    }

    #[test]
    fn test_recommendation_preferences() {
        let prefs = RecommendationPreferences::claude_code_china()
            .exclude_provider("anthropic-direct")
            .prefer_region("CN")
            .max_recommendations(3);

        assert!(prefs.claude_code_preferred);
        assert!(
            prefs
                .excluded_providers
                .contains(&"anthropic-direct".to_string())
        );
        assert!(prefs.preferred_regions.contains(&"CN".to_string()));
        assert_eq!(prefs.max_recommendations, Some(3));
    }

    #[test]
    fn test_claude_code_native_support() {
        let test_provider = Provider {
            name: "Test".to_string(),
            description: "Test".to_string(),
            icon: None,
            official: false,
            protected: false,
            custom: false,
            support_modes: vec![crate::provider::config::SupportMode {
                mode_type: crate::provider::config::ModeType::ClaudeCodeNative,
                name: "Claude Code Native".to_string(),
                description: "Test mode".to_string(),
                priority: 100,
                config: crate::provider::config::ModeConfig {
                    regional_urls: std::collections::HashMap::new(),
                    models: None,
                    additional_env: None,
                    rate_limit: None,
                },
            }],
            compatible_with: vec![],
            validation_endpoint: None,
            category: None,
            website: None,
            regions: vec![],
            env: std::collections::HashMap::new(),
        };

        assert!(RecommendationEngine::supports_claude_code_native(
            &test_provider
        ));
    }
}
