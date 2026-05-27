//! Encrypted on-disk storage for API keys and other secrets.
//!
//! Public settings live in `config.json` without secrets. Sensitive values are
//! stored in `secrets.enc` under the same data directory.

use crate::config::Config;
use anyhow::{anyhow, Context, Result};
use chacha20poly1305::aead::{Aead, KeyInit, OsRng};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const SECRETS_FILE: &str = "secrets.enc";
const KEY_FILE: &str = "device.key";

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
struct AppSecrets {
    #[serde(default)]
    pub api_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_token: Option<String>,
}

/// Non-secret fields persisted as plain JSON.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicConfig {
    pub model: String,
    pub auto_mode: bool,
    pub model_mode: crate::config::ModelMode,
    pub execution_mode: crate::config::ExecutionMode,
    pub thinking_level: crate::config::ThinkingLevel,
    pub external_access: crate::config::ExternalAccessMode,
    pub github_repo: Option<String>,
    pub github_branch: Option<String>,
    pub auto_commit_push: bool,
    #[serde(default)]
    pub trusted_external_paths: Vec<String>,
}

impl From<&Config> for PublicConfig {
    fn from(config: &Config) -> Self {
        Self {
            model: config.model.clone(),
            auto_mode: config.auto_mode,
            model_mode: config.model_mode.clone(),
            execution_mode: config.execution_mode.clone(),
            thinking_level: config.thinking_level.clone(),
            external_access: config.external_access.clone(),
            github_repo: config.github_repo.clone(),
            github_branch: config.github_branch.clone(),
            auto_commit_push: config.auto_commit_push,
            trusted_external_paths: config.trusted_external_paths.clone(),
        }
    }
}

impl PublicConfig {
    fn into_config(self, secrets: AppSecrets) -> Config {
        Config {
            api_key: secrets.api_key,
            model: self.model,
            auto_mode: self.auto_mode,
            model_mode: self.model_mode,
            execution_mode: self.execution_mode,
            thinking_level: self.thinking_level,
            external_access: self.external_access,
            github_token: secrets.github_token,
            github_repo: self.github_repo,
            github_branch: self.github_branch,
            auto_commit_push: self.auto_commit_push,
            trusted_external_paths: self.trusted_external_paths,
        }
    }
}

pub struct ConfigStore {
    base_dir: PathBuf,
}

impl ConfigStore {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    pub fn config_path(&self) -> PathBuf {
        self.base_dir.join("config.json")
    }

    pub fn secrets_path(&self) -> PathBuf {
        self.base_dir.join(SECRETS_FILE)
    }

    pub fn key_path(&self) -> PathBuf {
        self.base_dir.join(KEY_FILE)
    }

    pub fn save(&self, config: &Config) -> Result<()> {
        fs::create_dir_all(&self.base_dir)?;
        let public = PublicConfig::from(config);
        fs::write(self.config_path(), serde_json::to_string_pretty(&public)?)?;
        let secrets = AppSecrets {
            api_key: config.api_key.clone(),
            github_token: config.github_token.clone(),
        };
        write_encrypted_secrets(&self.key_path(), &self.secrets_path(), &secrets)?;
        Ok(())
    }

    pub fn load(&self) -> Result<Config> {
        let public_path = self.config_path();
        if !public_path.exists() {
            return Ok(Config::default());
        }
        let public: PublicConfig = serde_json::from_str(
            &fs::read_to_string(&public_path)
                .with_context(|| format!("read {}", public_path.display()))?,
        )?;
        let secrets = if self.secrets_path().exists() {
            read_encrypted_secrets(&self.key_path(), &self.secrets_path())?
        } else {
            // Legacy plain config.json that still contains secrets.
            legacy_secrets_from_plain_config(&public_path)?
        };
        Ok(public.into_config(secrets))
    }

    pub fn load_or_default(&self) -> Config {
        self.load().unwrap_or_default()
    }
}

fn legacy_secrets_from_plain_config(public_path: &Path) -> Result<AppSecrets> {
    let legacy: Config = serde_json::from_str(&fs::read_to_string(public_path)?)?;
    Ok(AppSecrets {
        api_key: legacy.api_key,
        github_token: legacy.github_token,
    })
}

fn load_or_create_device_key(key_path: &Path) -> Result<[u8; 32]> {
    if key_path.exists() {
        let bytes = fs::read(key_path)?;
        if bytes.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            return Ok(key);
        }
        return Err(anyhow!("device.key has invalid length {}", bytes.len()));
    }
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    if let Some(parent) = key_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(key_path, key)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(key_path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(key)
}

fn write_encrypted_secrets(
    key_path: &Path,
    secrets_path: &Path,
    secrets: &AppSecrets,
) -> Result<()> {
    let key = load_or_create_device_key(key_path)?;
    let cipher = ChaCha20Poly1305::new_from_slice(&key)
        .map_err(|error| anyhow!("invalid cipher key: {}", error))?;
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let plaintext = serde_json::to_vec(secrets)?;
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_ref())
        .map_err(|error| anyhow!("encrypt secrets: {}", error))?;
    let mut payload = nonce_bytes.to_vec();
    payload.extend_from_slice(&ciphertext);
    fs::write(secrets_path, payload)?;
    Ok(())
}

fn read_encrypted_secrets(key_path: &Path, secrets_path: &Path) -> Result<AppSecrets> {
    let key = load_or_create_device_key(key_path)?;
    let cipher = ChaCha20Poly1305::new_from_slice(&key)
        .map_err(|error| anyhow!("invalid cipher key: {}", error))?;
    let payload = fs::read(secrets_path)?;
    if payload.len() < 13 {
        return Err(anyhow!("secrets.enc is too short"));
    }
    let (nonce_bytes, ciphertext) = payload.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow!("failed to decrypt secrets.enc (wrong device.key?)"))?;
    Ok(serde_json::from_slice(&plaintext)?)
}

#[cfg(test)]
mod tests {
    use super::ConfigStore;
    use crate::config::Config;
    use std::fs;

    fn temp_dir(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "deepseek-config-store-{}-{}",
            label,
            std::process::id()
        ))
    }

    #[test]
    fn roundtrip_encrypts_secrets_separately_from_public_config() {
        let dir = temp_dir("roundtrip");
        let store = ConfigStore::new(&dir);
        let config = Config {
            api_key: "sk-test-secret".to_string(),
            github_token: Some("ghp_test".to_string()),
            ..Config::default()
        };
        store.save(&config).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.api_key, "sk-test-secret");
        assert_eq!(loaded.github_token.as_deref(), Some("ghp_test"));

        let public_json = fs::read_to_string(store.config_path()).unwrap();
        assert!(!public_json.contains("sk-test-secret"));
        assert!(!public_json.contains("ghp_test"));
        assert!(store.secrets_path().exists());
        assert!(store.key_path().exists());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn legacy_plain_config_still_loads() {
        let dir = temp_dir("legacy");
        fs::create_dir_all(&dir).unwrap();
        let legacy = Config {
            api_key: "sk-legacy".to_string(),
            ..Config::default()
        };
        fs::write(
            dir.join("config.json"),
            serde_json::to_string_pretty(&legacy).unwrap(),
        )
        .unwrap();
        let loaded = ConfigStore::new(&dir).load().unwrap();
        assert_eq!(loaded.api_key, "sk-legacy");
        let _ = fs::remove_dir_all(dir);
    }
}
