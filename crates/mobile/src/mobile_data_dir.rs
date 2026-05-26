//! Canonical mobile data directory (config, secrets, runtime store, workspace).

use std::path::{Path, PathBuf};
use std::sync::{Once, OnceLock};

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
static INIT_ONCE: Once = Once::new();

/// Set the data directory (Android JNI or tests). Also sets `DEEPSEEK_MOBILE_DATA_DIR`.
pub fn set_mobile_data_dir(path: impl AsRef<Path>) {
    let path = path.as_ref().to_path_buf();
    let _ = std::env::set_var("DEEPSEEK_MOBILE_DATA_DIR", path.to_string_lossy().as_ref());
    if let Some(existing) = DATA_DIR.get() {
        if existing != &path {
            return;
        }
        return;
    }
    let _ = DATA_DIR.set(path);
}

pub fn mobile_data_dir() -> Option<&'static Path> {
    DATA_DIR.get().map(PathBuf::as_path)
}

/// Prefer explicit set dir, then env var, then repo-local fallback for desktop dev.
pub fn resolve_data_dir() -> PathBuf {
    if let Some(path) = mobile_data_dir() {
        return path.to_path_buf();
    }
    if let Ok(path) = std::env::var("DEEPSEEK_MOBILE_DATA_DIR") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }
    PathBuf::from(".deepseek-mobile")
}

#[cfg(target_os = "android")]
pub fn ensure_android_storage_initialized() {
    INIT_ONCE.call_once(|| {
        if mobile_data_dir().is_some() {
            return;
        }
        if let Ok(path) = std::env::var("DEEPSEEK_MOBILE_DATA_DIR") {
            if !path.trim().is_empty() {
                set_mobile_data_dir(path);
                return;
            }
        }
        // MainActivity should call NativeBridge.initMobileDataDir before UI uses storage.
    });
}

#[cfg(not(target_os = "android"))]
pub fn ensure_android_storage_initialized() {}
