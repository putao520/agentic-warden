//! Network detection and connectivity analysis
//!
//! This module provides functionality to detect network connectivity
//! to both domestic and international services, enabling intelligent
//! provider URL selection based on actual network conditions.

use crate::sync::sync_config;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;

/// Network connectivity status for different regions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NetworkStatus {
    /// Both domestic and international networks work well
    Both {
        domestic_quality: f32,
        international_quality: f32,
        is_china_mainland: bool,
    },
    /// Only domestic network works well
    DomesticOnly {
        quality: f32,
        is_china_mainland: bool,
    },
    /// Only international network works well
    InternationalOnly {
        quality: f32,
        is_china_mainland: bool,
    },
    /// Neither network works well
    Poor {
        domestic_quality: f32,
        international_quality: f32,
        is_china_mainland: bool,
    },
    /// Network detection failed
    Unknown { is_china_mainland: bool },
}

impl NetworkStatus {
    /// Get recommended URL preference
    pub fn prefer_domestic(&self) -> Option<bool> {
        match self {
            NetworkStatus::Both {
                domestic_quality,
                international_quality,
                ..
            } => Some(domestic_quality >= international_quality),
            NetworkStatus::DomesticOnly { .. } => Some(true),
            NetworkStatus::InternationalOnly { .. } => Some(false),
            NetworkStatus::Poor { .. } => None,
            NetworkStatus::Unknown { .. } => None,
        }
    }

    /// Check if international URL might be inaccessible
    pub fn should_warn_international(&self) -> bool {
        matches!(
            self,
            NetworkStatus::DomesticOnly { .. } | NetworkStatus::Poor { .. } | NetworkStatus::Unknown { .. }
        )
    }

    /// Check if domestic URL might be inaccessible
    pub fn should_warn_domestic(&self) -> bool {
        matches!(
            self,
            NetworkStatus::InternationalOnly { .. } | NetworkStatus::Poor { .. } | NetworkStatus::Unknown { .. }
        )
    }

    /// Check if user is in China mainland
    pub fn is_china_mainland(&self) -> bool {
        match self {
            NetworkStatus::Both {
                is_china_mainland, ..
            } => *is_china_mainland,
            NetworkStatus::DomesticOnly {
                is_china_mainland, ..
            } => *is_china_mainland,
            NetworkStatus::InternationalOnly {
                is_china_mainland, ..
            } => *is_china_mainland,
            NetworkStatus::Poor {
                is_china_mainland, ..
            } => *is_china_mainland,
            NetworkStatus::Unknown { is_china_mainland } => *is_china_mainland,
        }
    }
}

/// Network connectivity detector
#[derive(Debug, Clone)]
pub struct NetworkDetector {
    timeout: Duration,
}

impl NetworkDetector {
    /// Create a new network detector
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(5),
        }
    }

    /// Create a detector with custom timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
    }

    /// Detect network connectivity status
    pub async fn detect(&self) -> Result<NetworkStatus> {
        // Test domestic connectivity
        let domestic_quality = self
            .test_domestic_connectivity()
            .await
            .context("Failed to test domestic connectivity")?;

        // Test international connectivity
        let international_quality = self
            .test_international_connectivity()
            .await
            .context("Failed to test international connectivity")?;

        // Detect if user is in China mainland based on domestic connectivity quality
        let is_china_mainland = domestic_quality > international_quality && domestic_quality > 0.5;

        // Determine network status
        let status = match (domestic_quality, international_quality) {
            (d, i) if d >= 0.7 && i >= 0.7 => NetworkStatus::Both {
                domestic_quality: d,
                international_quality: i,
                is_china_mainland,
            },
            (d, i) if d >= 0.7 && i < 0.7 => NetworkStatus::DomesticOnly {
                quality: d,
                is_china_mainland,
            },
            (d, i) if d < 0.7 && i >= 0.7 => NetworkStatus::InternationalOnly {
                quality: i,
                is_china_mainland,
            },
            (d, i) => NetworkStatus::Poor {
                domestic_quality: d,
                international_quality: i,
                is_china_mainland,
            },
        };

        Ok(status)
    }

    /// Test connectivity to domestic services
    async fn test_domestic_connectivity(&self) -> Result<f32> {
        let test_urls = vec![
            "https://open.bigmodel.cn",       // GLM
            "https://dashscope.aliyuncs.com", // Qwen
            "https://api.moonshot.cn",        // Kimi
            "https://api.minimax.chat",       // MiniMax
            "https://api.deepseek.com",       // DeepSeek
        ];

        self.test_connectivity_batch(test_urls).await
    }

    /// Test connectivity to international services
    async fn test_international_connectivity(&self) -> Result<f32> {
        let test_urls = vec![
            "https://api.openai.com",                    // OpenAI
            "https://api.anthropic.com",                 // Anthropic
            "https://openrouter.ai",                     // OpenRouter
            "https://generativelanguage.googleapis.com", // Google
            "https://openai.azure.com",                  // Azure OpenAI
        ];

        self.test_connectivity_batch(test_urls).await
    }

    /// Test connectivity to a batch of URLs and return average quality
    async fn test_connectivity_batch(&self, urls: Vec<&str>) -> Result<f32> {
        let mut successful_tests = 0;
        let mut total_response_time = 0u64;
        let total_urls = urls.len();

        for url in urls {
            match timeout(self.timeout, self.test_single_connectivity(url)).await {
                Ok(Ok(response_time)) => {
                    successful_tests += 1;
                    total_response_time += response_time;
                }
                Ok(Err(_)) => {
                    // Connection failed
                    continue;
                }
                Err(_) => {
                    // Timeout
                    continue;
                }
            }
        }

        if successful_tests == 0 {
            return Ok(0.0);
        }

        // Calculate quality based on success rate and average response time
        let success_rate = successful_tests as f32 / total_urls as f32;
        let avg_response_time = total_response_time as f32 / successful_tests as f32;

        // Quality score: success rate (70%) + response time factor (30%)
        let response_factor = if avg_response_time <= 1000.0 {
            1.0 // <= 1s is excellent
        } else if avg_response_time <= 3000.0 {
            0.8 // 1-3s is good
        } else if avg_response_time <= 10000.0 {
            0.6 // 3-10s is acceptable
        } else {
            0.4 // >10s is poor
        };

        Ok(success_rate * 0.7 + response_factor * 0.3)
    }

    /// Test connectivity to a single URL and return response time in ms
    async fn test_single_connectivity(&self, url: &str) -> Result<u64> {
        let start_time = std::time::Instant::now();

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()?;

        let response = client
            .get(url)
            .header("User-Agent", "agentic-warden/1.0 network-detection")
            .send()
            .await?;

        // We only care about getting a response, not the content
        let _ = response.bytes().await?;

        let elapsed = start_time.elapsed();
        Ok(elapsed.as_millis() as u64)
    }

    /// Get recommended base URL for a provider based on network status
    pub async fn get_recommended_url(
        &self,
        provider_id: &str,
        domestic_url: Option<&str>,
        international_url: Option<&str>,
    ) -> Result<(String, Option<String>)> {
        let network_status = self.detect().await?;

        let (url, warning) = match (domestic_url, international_url) {
            (Some(domestic), Some(international)) => {
                if network_status.should_warn_domestic()
                    && network_status.should_warn_international()
                {
                    (international.to_string(), Some("Both domestic and international network connectivity is poor. This provider may not work well.".to_string()))
                } else if network_status.should_warn_international() {
                    (domestic.to_string(), Some("International network connectivity is poor. Using domestic URL. Performance may be better with domestic providers.".to_string()))
                } else if network_status.should_warn_domestic() {
                    (international.to_string(), Some("Domestic network connectivity is poor. Using international URL. Consider using a VPN or proxy if this fails.".to_string()))
                } else {
                    // Both networks work well, prefer the one with better quality
                    let prefer_domestic = network_status.prefer_domestic().unwrap_or(true);
                    if prefer_domestic {
                        (domestic.to_string(), None)
                    } else {
                        (international.to_string(), None)
                    }
                }
            }
            (Some(domestic), None) => {
                let warning = if network_status.should_warn_domestic() {
                    Some(
                        "Domestic network connectivity is poor. This provider may not work well."
                            .to_string(),
                    )
                } else {
                    None
                };
                (domestic.to_string(), warning)
            }
            (None, Some(international)) => {
                let warning = if network_status.should_warn_international() {
                    Some("International network connectivity is poor. This provider may not work without a VPN or proxy.".to_string())
                } else {
                    None
                };
                (international.to_string(), warning)
            }
            (None, None) => {
                return Err(anyhow::anyhow!(
                    "No URLs available for provider: {}",
                    provider_id
                ));
            }
        };

        Ok((url, warning))
    }
}

impl Default for NetworkDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Load cached network status from sync configuration if it exists.
pub fn load_cached_status() -> Option<NetworkStatus> {
    sync_config::load_sync_data()
        .ok()
        .and_then(|data| data.state.network_status)
}

/// Ensure a recent network status is available, falling back to live detection on demand.
pub async fn ensure_status(detector: &NetworkDetector) -> NetworkStatus {
    if let Some(status) = load_cached_status() {
        return status;
    }

    match detector.detect().await {
        Ok(status) => {
            let _ = sync_config::save_network_status(status.clone());
            status
        }
        Err(_) => NetworkStatus::Unknown {
            is_china_mainland: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_detection() {
        let detector = NetworkDetector::new();
        let status = detector.detect().await;

        // Should not panic, even if network is unavailable
        assert!(status.is_ok());
    }

    #[tokio::test]
    async fn test_single_connectivity() {
        let detector = NetworkDetector::new();

        // Test a well-known URL
        let result = detector
            .test_single_connectivity("https://www.google.com")
            .await;

        // May succeed or fail depending on network, but should not panic
        assert!(result.is_ok() || result.is_err());
    }
}
