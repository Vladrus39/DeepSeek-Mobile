//! First-launch setup: Android storage path + dev API key from `.env` / debug build.

use crate::dev_api_key::apply_dev_api_key_bootstrap;
use crate::mobile_data_dir::{ensure_android_storage_initialized, resolve_data_dir};
use crate::settings_state::load_saved_config;
#[cfg(not(target_os = "android"))]
use crate::settings_state::{config_store, save_config};
use deepseek_mobile_core::config::Config;
use std::fs;
use std::sync::Once;

static STARTUP_ONCE: Once = Once::new();

/// Call at UI startup before reading settings/runtime paths.
pub fn startup() {
    STARTUP_ONCE.call_once(|| {
        ensure_android_storage_initialized();
        let data_dir = resolve_data_dir();
        if let Err(error) = fs::create_dir_all(&data_dir) {
            eprintln!("could not create data dir {}: {}", data_dir.display(), error);
        }
        let workspace = data_dir.join("workspace");
        if let Err(error) = fs::create_dir_all(&workspace) {
            eprintln!("could not create workspace {}: {}", workspace.display(), error);
        } else {
            let readme = workspace.join("README.md");
            if !readme.exists() {
                let _ = fs::write(
                    &readme,
                    "# DeepSeek Mobile workspace\n\nImport a ZIP from Files or clone a repo in Termux.\n",
                );
            }
        }
        seed_dev_secrets_if_needed();
    });
}

/// Suggested API key for onboarding prefill (debug `.env` / compile-time). Does not persist on Android.
pub fn prefill_api_key_for_onboarding() -> String {
    if let Some(config) = load_saved_config() {
        let key = config.api_key.trim().to_string();
        if key.starts_with("sk-") {
            return key;
        }
    }
    let mut config = Config::default();
    apply_dev_api_key_bootstrap(&mut config);
    config.api_key
}

/// Persist compile-time / env API key into encrypted secrets when the store is empty.
///
/// **Android:** skipped — users must confirm the key in onboarding or Settings (release-like UX).
/// **Desktop debug:** auto-seed for faster local iteration.
pub fn seed_dev_secrets_if_needed() {
    #[cfg(not(target_os = "android"))]
    {
        let store = config_store();
        let mut config = store.load_or_default();
        apply_dev_api_key_bootstrap(&mut config);

        let key = config.api_key.trim();
        if !key.starts_with("sk-") {
            return;
        }

        let needs_save = !store.secrets_path().exists()
            || store
                .load()
                .map(|loaded| loaded.api_key.trim().is_empty())
                .unwrap_or(true);

        if needs_save {
            if let Err(error) = save_config(&config) {
                eprintln!("dev config seed failed: {}", error);
            } else {
                eprintln!("dev API key seeded into {}", store.secrets_path().display());
            }
        }
    }
}

pub fn api_key_configured() -> bool {
    load_saved_config()
        .map(|config| config.api_key.trim().starts_with("sk-"))
        .unwrap_or(false)
}
