use crate::config::{DEBUG_ENV, LEGACY_DEBUG_ENV};
use std::env;
use std::sync::OnceLock;

static DEBUG_ENABLED: OnceLock<bool> = OnceLock::new();

fn enabled() -> bool {
    *DEBUG_ENABLED.get_or_init(|| {
        read_bool(DEBUG_ENV)
            .or_else(|| read_bool(LEGACY_DEBUG_ENV))
            .unwrap_or(false)
    })
}

fn read_bool(var: &str) -> Option<bool> {
    env::var(var)
        .ok()
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
}

pub fn debug(message: impl AsRef<str>) {
    if enabled() {
        eprintln!("[agentic-warden][debug] {}", message.as_ref());
    }
}

pub fn warn(message: impl AsRef<str>) {
    eprintln!("[agentic-warden][warn] {}", message.as_ref());
}
