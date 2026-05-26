//! Debug/dev API-key bootstrap for mobile builds.
//!
//! The mobile `build.rs` can inject `DEEPSEEK_API_KEY` for debug APK testing.
//! Release builds intentionally ignore embedded/environment keys.

use deepseek_mobile_core::config::Config;

pub fn apply_dev_api_key_bootstrap(config: &mut Config) {
    deepseek_mobile_core::apply_dev_api_key_bootstrap(config);
    if !config.api_key.trim().is_empty() || !cfg!(debug_assertions) {
        return;
    }

    if let Some(key) = option_env!("DEEPSEEK_API_KEY") {
        let key = key.trim();
        if key.starts_with("sk-") {
            config.api_key = key.to_string();
        }
    }
}
