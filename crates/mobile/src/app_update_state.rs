//! In-app update check and APK download (Android sideload via GitHub Releases).

use crate::mobile_runtime_config::default_data_dir;
use deepseek_mobile_core::{
    check_github_release_update, AppUpdateOffer, DEFAULT_GITHUB_REPO,
};
use std::path::PathBuf;

const UPDATE_APK_NAME: &str = "deepseek-mobile-update.apk";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum AppUpdateUiPhase {
    #[default]
    Idle,
    Checking,
    UpToDate,
    UpdateAvailable(AppUpdateOffer),
    Downloading,
    Downloaded { path: PathBuf, version: String },
    Error(String),
}

#[derive(Clone, Debug, Default)]
pub struct AppUpdateUiState {
    pub phase: AppUpdateUiPhase,
}

impl AppUpdateUiState {
    pub fn current_version() -> String {
        option_env!("DEEPSEEK_MOBILE_VERSION")
            .unwrap_or(env!("CARGO_PKG_VERSION"))
            .to_string()
    }

    pub fn update_apk_path() -> PathBuf {
        default_data_dir().join("updates").join(UPDATE_APK_NAME)
    }

    pub fn status_line(&self) -> String {
        match &self.phase {
            AppUpdateUiPhase::Idle => format!("Version {}", Self::current_version()),
            AppUpdateUiPhase::Checking => "Checking GitHub for updates…".to_string(),
            AppUpdateUiPhase::UpToDate => format!(
                "Version {} is up to date",
                Self::current_version()
            ),
            AppUpdateUiPhase::UpdateAvailable(offer) => format!(
                "Update available: {} → {}",
                offer.current_version, offer.latest_version
            ),
            AppUpdateUiPhase::Downloading => "Downloading update APK…".to_string(),
            AppUpdateUiPhase::Downloaded { version, .. } => {
                format!("Downloaded v{version}. Tap Install update.")
            }
            AppUpdateUiPhase::Error(message) => format!("Update error: {message}"),
        }
    }

    pub fn begin_check(&mut self) {
        self.phase = AppUpdateUiPhase::Checking;
    }

    pub fn apply_check_result(&mut self, offer: Option<AppUpdateOffer>) {
        self.phase = match offer {
            Some(offer) => AppUpdateUiPhase::UpdateAvailable(offer),
            None => AppUpdateUiPhase::UpToDate,
        };
    }

    pub fn set_error(&mut self, message: impl Into<String>) {
        self.phase = AppUpdateUiPhase::Error(message.into());
    }

    pub fn begin_download(&mut self) {
        self.phase = AppUpdateUiPhase::Downloading;
    }

    pub fn apply_downloaded(&mut self, path: PathBuf, version: String) {
        self.phase = AppUpdateUiPhase::Downloaded { path, version };
    }

    pub async fn check_for_update(&mut self) -> Result<(), String> {
        self.begin_check();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(45))
            .build()
            .map_err(|error| error.to_string())?;
        let offer = check_github_release_update(
            &client,
            DEFAULT_GITHUB_REPO,
            &Self::current_version(),
        )
        .await
        .map_err(|error| error.to_string())?;
        self.apply_check_result(offer);
        Ok(())
    }

    pub async fn download_available_update(&mut self) -> Result<PathBuf, String> {
        let AppUpdateUiPhase::UpdateAvailable(offer) = self.phase.clone() else {
            return Err("No update is available to download".to_string());
        };
        let url = offer
            .apk_download_url
            .clone()
            .ok_or_else(|| "Release has no APK asset URL".to_string())?;
        self.begin_download();
        let path = Self::update_apk_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(600))
            .build()
            .map_err(|error| error.to_string())?;
        let bytes = client
            .get(url)
            .header("User-Agent", "DeepSeek-Mobile")
            .send()
            .await
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?
            .bytes()
            .await
            .map_err(|error| error.to_string())?;
        if bytes.len() < 100_000 {
            return Err("Downloaded file is too small to be a valid APK".to_string());
        }
        std::fs::write(&path, &bytes).map_err(|error| error.to_string())?;
        let version = offer.latest_version.clone();
        self.apply_downloaded(path.clone(), version);
        Ok(path)
    }

    pub fn downloaded_apk_path(&self) -> Option<PathBuf> {
        match &self.phase {
            AppUpdateUiPhase::Downloaded { path, .. } => Some(path.clone()),
            _ => None,
        }
    }
}
