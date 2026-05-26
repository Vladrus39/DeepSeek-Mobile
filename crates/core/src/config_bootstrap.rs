//! Debug/dev bootstrap for API keys.

use crate::config::Config;

/// Fill `api_key` from process env in debug builds when empty.
///
/// Mobile debug APKs also have a mobile-crate bootstrap that can use compile-time
/// values injected by `crates/mobile/build.rs`. This core helper intentionally
/// never embeds secrets in release builds.
pub fn apply_dev_api_key_bootstrap(config: &mut Config) {
    if !config.api_key.trim().is_empty() || !cfg!(debug_assertions) {
        return;
    }

    if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
        let key = key.trim();
        if key.starts_with("sk-") {
            config.api_key = key.to_string();
        }
    }
}
