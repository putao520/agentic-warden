//! Intelligent provider recommendation engine.
//!
//! Consumes the provider configuration together with runtime network
//! conditions to rank the most suitable provider/mode combinations for
//! the selected AI CLI.

use crate::provider::config::{
    AiType, Provider, ProvidersConfig, RecommendationScenario, Region, SupportMode,
};
use crate::provider::network_detector::{ensure_status, NetworkDetector, NetworkStatus};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Recommendation returned to the TUI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub provider_id: String,
    pub provider: Provider,
    pub support_mode: SupportMode,
    pub recommended_region: Region,
    pub score: f32,
    pub reason: String,
    pub has_token: bool,
    pub warnings: Vec<String>,
}

/// Preferences used to fine tune the recommendation engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationPreferences {
    pub scenario: Option<RecommendationScenario>,
    pub prefer_official: bool,
    pub prefer_existing_tokens: bool,
    pub preferred_region_override: Option<Region>,
    pub excluded_providers: Vec<String>,
}

impl Default for RecommendationPreferences {
    fn default() -> Self {
        Self {
            scenario: None,
            prefer_official: false,
            prefer_existing_tokens: true,
            preferred_region_override: None,
            excluded_providers: Vec::new(),
        }
    }
}

impl RecommendationPreferences {
    fn merged_with(&self, other: &RecommendationPreferences) -> RecommendationPreferences {
        let mut merged = self.clone();

        if other.scenario.is_some() {
            merged.scenario = other.scenario.clone();
        }

        if other.preferred_region_override.is_some() {
            merged.preferred_region_override = other.preferred_region_override.clone();
        }

        if other.prefer_official != merged.prefer_official {
            merged.prefer_official = other.prefer_official;
        }

        if other.prefer_existing_tokens != merged.prefer_existing_tokens {
            merged.prefer_existing_tokens = other.prefer_existing_tokens;
        }

        if !other.excluded_providers.is_empty() {
            merged
                .excluded_providers
                .extend(other.excluded_providers.iter().cloned());
            merged.excluded_providers.sort();
            merged.excluded_providers.dedup();
        }

        merged
    }
}

/// Recommendation engine orchestrator.
pub struct RecommendationEngine {
    detector: NetworkDetector,
    preferences: RecommendationPreferences,
}

impl RecommendationEngine {
    pub fn new() -> Self {
        Self {
            detector: NetworkDetector::new(),
            preferences: RecommendationPreferences::default(),
        }
    }

    pub fn with_preferences(preferences: RecommendationPreferences) -> Self {
        Self {
            detector: NetworkDetector::new(),
            preferences,
        }
    }

    pub fn set_preferences(&mut self, preferences: RecommendationPreferences) {
        self.preferences = preferences;
    }

    pub fn get_preferences(&self) -> &RecommendationPreferences {
        &self.preferences
    }

    /// Build ranked recommendations for the specified AI CLI type.
    pub async fn get_recommendations(
        &self,
        config: &ProvidersConfig,
        ai_type: &AiType,
        override_preferences: &RecommendationPreferences,
    ) -> Result<Vec<Recommendation>> {
        let prefs = self.preferences.merged_with(override_preferences);
        let network_status = ensure_status(&self.detector).await;
        let preferred_region = prefs
            .preferred_region_override
            .clone()
            .or_else(|| preferred_region_from_status(&network_status));

        let mut recommendations = Vec::new();

        for (provider_id, provider) in &config.providers {
            if prefs
                .excluded_providers
                .iter()
                .any(|excluded| excluded.eq_ignore_ascii_case(provider_id))
            {
                continue;
            }

            if !provider.compatible_with.contains(ai_type) {
                continue;
            }

            let Some(mode) = config.select_best_mode(provider_id, ai_type) else {
                continue;
            };

            let Some((region, _regional_config)) = pick_region(mode, preferred_region.as_ref())
            else {
                continue;
            };

            let has_token = config.has_token(provider_id, &region);
            let warnings = build_warnings(&network_status, &region, has_token, &prefs, provider);
            let score = compute_score(
                provider_id,
                provider,
                mode,
                &region,
                has_token,
                config,
                &prefs,
                &network_status,
            );
            let reason = build_reason(
                provider,
                mode,
                has_token,
                provider_id == &config.default_provider,
                &prefs,
            );

            recommendations.push(Recommendation {
                provider_id: provider_id.clone(),
                provider: provider.clone(),
                support_mode: mode.clone(),
                recommended_region: region,
                score,
                reason,
                has_token,
                warnings,
            });
        }

        recommendations.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.provider_id.cmp(&b.provider_id))
        });

        Ok(recommendations)
    }
}

impl Default for RecommendationEngine {
    fn default() -> Self {
        Self::new()
    }
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

fn compute_score(
    provider_id: &str,
    provider: &Provider,
    mode: &SupportMode,
    region: &Region,
    has_token: bool,
    config: &ProvidersConfig,
    prefs: &RecommendationPreferences,
    network_status: &NetworkStatus,
) -> f32 {
    let mut score = (mode.priority as f32) / 100.0;

    if provider_id == config.default_provider {
        score += 0.15;
    }

    if provider.official {
        score += 0.1;
    }

    if provider.custom {
        score -= 0.05;
    }

    if has_token {
        score += 0.2;
    } else if prefs.prefer_existing_tokens {
        score -= 0.1;
    }

    if prefs.prefer_official && provider.official {
        score += 0.1;
    }

    if let Some(prefers_domestic) = network_status.prefer_domestic() {
        let region_is_domestic = matches!(region, Region::MainlandChina);
        if prefers_domestic == region_is_domestic {
            score += 0.05;
        } else {
            score -= 0.05;
        }
    }

    if let Some(scenario) = &prefs.scenario {
        score += match scenario {
            RecommendationScenario::ChinaMainland => {
                if matches!(region, Region::MainlandChina) {
                    0.1
                } else {
                    -0.1
                }
            }
            RecommendationScenario::International => {
                if matches!(region, Region::International) {
                    0.05
                } else {
                    -0.05
                }
            }
            RecommendationScenario::ClaudeCodePreferred => {
                if mode.mode_type.to_string().contains("Claude") {
                    0.05
                } else {
                    -0.05
                }
            }
            RecommendationScenario::CostEffective => {
                if provider.custom || !provider.official {
                    0.08
                } else {
                    -0.04
                }
            }
            RecommendationScenario::HighPerformance => {
                if mode.priority >= 90 {
                    0.05
                } else {
                    -0.05
                }
            }
        };
    }

    score.clamp(0.0, 1.0)
}

fn build_reason(
    provider: &Provider,
    mode: &SupportMode,
    has_token: bool,
    is_default: bool,
    prefs: &RecommendationPreferences,
) -> String {
    let mut parts = vec![format!("模式: {}", mode.name)];

    if provider.official {
        parts.push("官方渠道".to_string());
    }

    if is_default {
        parts.push("默认 Provider".to_string());
    }

    if has_token {
        parts.push("Token 已配置".to_string());
    }

    if prefs.prefer_official && provider.official {
        parts.push("符合官方优先策略".to_string());
    }

    if parts.len() == 1 {
        "综合评分推荐".to_string()
    } else {
        parts.join(" · ")
    }
}

fn build_warnings(
    status: &NetworkStatus,
    region: &Region,
    has_token: bool,
    prefs: &RecommendationPreferences,
    provider: &Provider,
) -> Vec<String> {
    let mut warnings = Vec::new();

    if matches!(region, Region::International) && status.should_warn_international() {
        warnings.push("当前国际网络不稳定，可能需要代理".to_string());
    } else if matches!(region, Region::MainlandChina) && status.should_warn_domestic() {
        warnings.push("当前国内网络不稳定".to_string());
    }

    if prefs.prefer_existing_tokens && !has_token {
        warnings.push("尚未配置 Token".to_string());
    }

    if provider.custom && provider.official {
        warnings.push("自定义配置覆盖官方 Provider".to_string());
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn builds_recommendations_for_default_config() {
        let config = ProvidersConfig::create_default().expect("default config");
        let engine = RecommendationEngine::new();
        let recs = engine
            .get_recommendations(
                &config,
                &AiType::Claude,
                &RecommendationPreferences::default(),
            )
            .await
            .expect("recommendations");

        assert!(
            !recs.is_empty(),
            "should recommend at least one provider for Claude"
        );
        assert!(recs
            .iter()
            .all(|rec| rec.provider.compatible_with.contains(&AiType::Claude)));
    }
}
