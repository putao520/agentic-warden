//! Config-related helpers.

use crate::commands::market::source::{MarketError, MarketErrorCode, MarketResult};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub fn parse_env_pairs(env_vars: &[String]) -> MarketResult<HashMap<String, String>> {
    let mut map = HashMap::new();
    for item in env_vars {
        let mut parts = item.splitn(2, '=');
        let key = parts.next().unwrap_or_default();
        let value = parts.next();
        if key.trim().is_empty() || value.is_none() {
            return Err(MarketError::new(
                MarketErrorCode::InvalidEnvironment,
                format!("Invalid env var format: {}", item),
            ));
        }
        map.insert(key.to_string(), value.unwrap().to_string());
    }
    Ok(map)
}

pub fn parse_installed_at(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}
