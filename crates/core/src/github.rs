//! GitHub REST API client.
//!
//! Provides authenticated access to GitHub repositories, pull requests,
//! issues, and content operations. Uses the GitHub REST API v3 with
//! personal access token authentication.

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const GITHUB_API_BASE: &str = "https://api.github.com";

/// Parsed GitHub repository reference: "owner/repo"
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitHubRepo {
    pub owner: String,
    pub name: String,
}

impl GitHubRepo {
    pub fn parse(repo: &str) -> Result<Self> {
        let parts: Vec<&str> = repo.trim().split('/').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err(anyhow!(
                "invalid GitHub repository format: '{}'. Expected 'owner/repo'",
                repo
            ));
        }
        Ok(Self {
            owner: parts[0].to_string(),
            name: parts[1].to_string(),
        })
    }

    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }
}

/// GitHub branch information
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitHubBranch {
    pub name: String,
    pub sha: String,
}

/// GitHub repository information
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitHubRepoInfo {
    pub full_name: String,
    pub description: Option<String>,
    pub default_branch: String,
    pub private: bool,
    pub html_url: String,
    pub clone_url: String,
    pub ssh_url: String,
}

/// GitHub pull request
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitHubPullRequest {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub html_url: String,
    pub head_branch: String,
    pub base_branch: String,
    pub created_at: String,
    pub updated_at: String,
}

/// GitHub issue
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitHubIssue {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub html_url: String,
    pub created_at: String,
    pub updated_at: String,
    pub labels: Vec<String>,
}

/// GitHub content entry (file or directory)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitHubContentEntry {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub entry_type: String,
    pub sha: String,
    pub size: u64,
    pub html_url: Option<String>,
}

/// GitHub file content with decoded text
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitHubFileContent {
    pub path: String,
    pub sha: String,
    pub content: String,
    pub size: u64,
}

/// Result of committing a file change via GitHub API
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitHubCommitResult {
    pub sha: String,
    pub html_url: String,
}

pub struct GitHubClient {
    client: Client,
    token: String,
}

impl GitHubClient {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            token: token.into(),
        }
    }

    fn headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", self.token)
                .parse()
                .unwrap(),
        );
        headers.insert(
            "Accept",
            "application/vnd.github+json".parse().unwrap(),
        );
        headers.insert("X-GitHub-Api-Version", "2022-11-28".parse().unwrap());
        headers.insert(
            "User-Agent",
            "DeepSeek-Mobile/0.1.0".parse().unwrap(),
        );
        headers
    }

    /// Get repository information
    pub async fn get_repo(&self, repo: &GitHubRepo) -> Result<GitHubRepoInfo> {
        let url = format!("{}/repos/{}", GITHUB_API_BASE, repo.full_name());
        let response = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await
            .context("get repo info")?;
        check_response(&response)?;
        Ok(response.json().await.context("parse repo info")?)
    }

    /// List branches
    pub async fn list_branches(&self, repo: &GitHubRepo) -> Result<Vec<GitHubBranch>> {
        let url = format!("{}/repos/{}/branches", GITHUB_API_BASE, repo.full_name());
        let response = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await
            .context("list branches")?;
        check_response(&response)?;
        Ok(response.json().await.context("parse branches")?)
    }

    /// Get default branch
    pub async fn get_default_branch(&self, repo: &GitHubRepo) -> Result<String> {
        let info = self.get_repo(repo).await?;
        Ok(info.default_branch)
    }

    /// Get file contents from repository
    pub async fn get_file_content(
        &self,
        repo: &GitHubRepo,
        path: &str,
        branch: Option<&str>,
    ) -> Result<GitHubFileContent> {
        let url = format!(
            "{}/repos/{}/contents/{}",
            GITHUB_API_BASE,
            repo.full_name(),
            path.trim_start_matches('/')
        );
        let mut request = self.client.get(&url).headers(self.headers());
        if let Some(branch) = branch {
            request = request.query(&[("ref", branch)]);
        }
        let response = request.send().await.context("get file content")?;
        check_response(&response)?;
        let value: Value = response.json().await.context("parse file content")?;
        let content = value
            .get("content")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("missing content field"))?;
        let decoded = String::from_utf8(
            base64_decode_ignore_whitespace(content)
                .map_err(|e| anyhow!("base64 decode: {}", e))?,
        )
        .context("utf-8 decode file content")?;
        let decoded_len = decoded.len() as u64;
        Ok(GitHubFileContent {
            path: value["path"].as_str().unwrap_or(path).to_string(),
            sha: value["sha"].as_str().unwrap_or("").to_string(),
            content: decoded,
            size: decoded_len,
        })
    }

    /// List directory contents
    pub async fn list_contents(
        &self,
        repo: &GitHubRepo,
        path: &str,
        branch: Option<&str>,
    ) -> Result<Vec<GitHubContentEntry>> {
        let url = format!(
            "{}/repos/{}/contents/{}",
            GITHUB_API_BASE,
            repo.full_name(),
            path.trim_start_matches('/').trim_end_matches('/')
        );
        let mut request = self.client.get(&url).headers(self.headers());
        if let Some(branch) = branch {
            request = request.query(&[("ref", branch)]);
        }
        let response = request.send().await.context("list contents")?;
        check_response(&response)?;
        Ok(response
            .json()
            .await
            .context("parse directory contents")?)
    }

    /// Create or update a file via GitHub API (commits directly)
    pub async fn create_or_update_file(
        &self,
        repo: &GitHubRepo,
        path: &str,
        content: &str,
        message: &str,
        branch: &str,
        sha: Option<&str>,
    ) -> Result<GitHubCommitResult> {
        let url = format!(
            "{}/repos/{}/contents/{}",
            GITHUB_API_BASE,
            repo.full_name(),
            path.trim_start_matches('/')
        );
        let mut body = serde_json::json!({
            "message": message,
            "content": base64_encode(content),
            "branch": branch,
        });
        if let Some(sha) = sha {
            body["sha"] = serde_json::Value::String(sha.to_string());
        }
        let response = self
            .client
            .put(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .context("create/update file")?;
        check_response(&response)?;
        let value: Value = response.json().await.context("parse commit result")?;
        Ok(GitHubCommitResult {
            sha: value["content"]["sha"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            html_url: value["content"]["html_url"]
                .as_str()
                .unwrap_or("")
                .to_string(),
        })
    }

    /// Create a pull request
    pub async fn create_pr(
        &self,
        repo: &GitHubRepo,
        title: &str,
        body: &str,
        head: &str,
        base: &str,
    ) -> Result<GitHubPullRequest> {
        let url = format!("{}/repos/{}/pulls", GITHUB_API_BASE, repo.full_name());
        let payload = serde_json::json!({
            "title": title,
            "body": body,
            "head": head,
            "base": base,
        });
        let response = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&payload)
            .send()
            .await
            .context("create PR")?;
        check_response(&response)?;
        Ok(response.json().await.context("parse PR")?)
    }

    /// List open pull requests
    pub async fn list_prs(
        &self,
        repo: &GitHubRepo,
        state: Option<&str>,
    ) -> Result<Vec<GitHubPullRequest>> {
        let url = format!("{}/repos/{}/pulls", GITHUB_API_BASE, repo.full_name());
        let mut request = self.client.get(&url).headers(self.headers());
        if let Some(state) = state {
            request = request.query(&[("state", state)]);
        }
        let response = request.send().await.context("list PRs")?;
        check_response(&response)?;
        Ok(response.json().await.context("parse PRs")?)
    }

    /// Create an issue
    pub async fn create_issue(
        &self,
        repo: &GitHubRepo,
        title: &str,
        body: &str,
        labels: &[String],
    ) -> Result<GitHubIssue> {
        let url = format!("{}/repos/{}/issues", GITHUB_API_BASE, repo.full_name());
        let payload = serde_json::json!({
            "title": title,
            "body": body,
            "labels": labels,
        });
        let response = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&payload)
            .send()
            .await
            .context("create issue")?;
        check_response(&response)?;
        let value: Value = response.json().await.context("parse issue")?;
        Ok(GitHubIssue {
            number: value["number"].as_u64().unwrap_or(0),
            title: value["title"].as_str().unwrap_or("").to_string(),
            body: value["body"].as_str().map(String::from),
            state: value["state"].as_str().unwrap_or("open").to_string(),
            html_url: value["html_url"].as_str().unwrap_or("").to_string(),
            created_at: value["created_at"].as_str().unwrap_or("").to_string(),
            updated_at: value["updated_at"].as_str().unwrap_or("").to_string(),
            labels: value["labels"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|l| l["name"].as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        })
    }

    /// List issues
    pub async fn list_issues(
        &self,
        repo: &GitHubRepo,
        state: Option<&str>,
    ) -> Result<Vec<GitHubIssue>> {
        let url = format!("{}/repos/{}/issues", GITHUB_API_BASE, repo.full_name());
        let mut request = self.client.get(&url).headers(self.headers());
        if let Some(state) = state {
            request = request.query(&[("state", state)]);
        }
        let response = request.send().await.context("list issues")?;
        check_response(&response)?;
        let values: Vec<Value> = response.json().await.context("parse issues")?;
        Ok(values
            .into_iter()
            .filter(|v| v.get("pull_request").is_none()) // exclude PRs
            .map(|v| GitHubIssue {
                number: v["number"].as_u64().unwrap_or(0),
                title: v["title"].as_str().unwrap_or("").to_string(),
                body: v["body"].as_str().map(String::from),
                state: v["state"].as_str().unwrap_or("open").to_string(),
                html_url: v["html_url"].as_str().unwrap_or("").to_string(),
                created_at: v["created_at"].as_str().unwrap_or("").to_string(),
                updated_at: v["updated_at"].as_str().unwrap_or("").to_string(),
                labels: v["labels"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|l| l["name"].as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
            })
            .collect())
    }

    /// Get the authenticated user
    pub async fn get_user(&self) -> Result<String> {
        let url = format!("{}/user", GITHUB_API_BASE);
        let response = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await
            .context("get user")?;
        check_response(&response)?;
        let value: Value = response.json().await.context("parse user")?;
        Ok(value["login"].as_str().unwrap_or("unknown").to_string())
    }
}

fn check_response(response: &reqwest::Response) -> Result<()> {
    if response.status().is_success() {
        return Ok(());
    }
    let status = response.status();
    Err(anyhow!(
        "GitHub API returned HTTP {}. Check your token and repository name.",
        status
    ))
}

fn base64_encode(input: &str) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(input.as_bytes())
}

fn base64_decode_ignore_whitespace(input: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::Engine as _;
    let cleaned: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    base64::engine::general_purpose::STANDARD.decode(&cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_repo() {
        let repo = GitHubRepo::parse("Vladrus39/DeepSeek-Mobile").unwrap();
        assert_eq!(repo.owner, "Vladrus39");
        assert_eq!(repo.name, "DeepSeek-Mobile");
    }

    #[test]
    fn rejects_invalid_repo() {
        assert!(GitHubRepo::parse("invalid").is_err());
        assert!(GitHubRepo::parse("").is_err());
        assert!(GitHubRepo::parse("/repo").is_err());
        assert!(GitHubRepo::parse("owner/").is_err());
    }

    #[test]
    fn base64_roundtrip() {
        let input = "Hello, GitHub!";
        let encoded = base64_encode(input);
        let decoded = base64_decode_ignore_whitespace(&encoded).unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), input);
    }
}
