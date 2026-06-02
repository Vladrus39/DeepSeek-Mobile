//! GitHub release metadata for in-app update checks (Android sideload / debug releases).

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

pub const DEFAULT_GITHUB_REPO: &str = "Vladrus39/DeepSeek-Mobile";
pub const APK_ASSET_PREFIX: &str = "deepseek-mobile-";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppUpdateOffer {
    pub current_version: String,
    pub latest_version: String,
    pub tag_name: String,
    pub release_page_url: String,
    pub apk_download_url: Option<String>,
    pub release_notes: String,
}

#[derive(Clone, Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    draft: bool,
    prerelease: bool,
    assets: Vec<GithubAsset>,
}

#[derive(Clone, Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

/// Compare dotted numeric versions (`0.1.0` < `0.1.1` < `0.2.0`).
pub fn version_is_newer(latest: &str, current: &str) -> bool {
    let latest_parts = parse_version_parts(latest);
    let current_parts = parse_version_parts(current);
    latest_parts > current_parts
}

fn parse_version_parts(raw: &str) -> Vec<u32> {
    raw.trim()
        .trim_start_matches('v')
        .split('.')
        .filter_map(|part| part.parse::<u32>().ok())
        .collect()
}

pub fn apk_asset_name_for_version(version: &str) -> String {
    format!(
        "{APK_ASSET_PREFIX}{}.apk",
        version.trim().trim_start_matches('v')
    )
}

pub fn apk_download_url(repo: &str, tag_name: &str, version: &str) -> String {
    let tag = tag_name.trim();
    let asset = apk_asset_name_for_version(version);
    format!("https://github.com/{repo}/releases/download/{tag}/{asset}")
}

/// Fetch the newest non-draft GitHub release and compare to `current_version`.
pub async fn check_github_release_update(
    client: &reqwest::Client,
    repo: &str,
    current_version: &str,
) -> Result<Option<AppUpdateOffer>> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let response = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "DeepSeek-Mobile")
        .send()
        .await
        .context("GitHub releases request failed")?;
    if !response.status().is_success() {
        return Err(anyhow!(
            "GitHub releases HTTP {}: {}",
            response.status(),
            response.text().await.unwrap_or_default()
        ));
    }
    let release: GithubRelease = response.json().await.context("parse GitHub release JSON")?;
    if release.draft || release.prerelease {
        return Ok(None);
    }
    let latest_version = release.tag_name.trim().trim_start_matches('v').to_string();
    let current = current_version.trim().trim_start_matches('v');
    if !version_is_newer(&latest_version, current) {
        return Ok(None);
    }
    let apk_download_url = release
        .assets
        .iter()
        .find(|asset| asset.name == apk_asset_name_for_version(&latest_version))
        .map(|asset| asset.browser_download_url.clone())
        .or_else(|| Some(apk_download_url(repo, &release.tag_name, &latest_version)));
    Ok(Some(AppUpdateOffer {
        current_version: current.to_string(),
        latest_version: latest_version.clone(),
        tag_name: release.tag_name,
        release_page_url: release.html_url,
        apk_download_url,
        release_notes: release.body.unwrap_or_default(),
    }))
}

#[cfg(test)]
mod tests {
    use super::{apk_asset_name_for_version, version_is_newer};

    #[test]
    fn version_ordering() {
        assert!(version_is_newer("0.1.1", "0.1.0"));
        assert!(version_is_newer("0.2.0", "0.1.9"));
        assert!(!version_is_newer("0.1.0", "0.1.0"));
        assert!(!version_is_newer("0.1.0", "0.2.0"));
    }

    #[test]
    fn apk_asset_name() {
        assert_eq!(
            apk_asset_name_for_version("v0.1.2"),
            "deepseek-mobile-0.1.2.apk"
        );
    }
}
