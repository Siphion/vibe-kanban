use axum::{Router, response::Json as ResponseJson, routing::get};
use serde::{Deserialize, Serialize};
use utils::{path::expand_tilde, response::ApiResponse};

use crate::{DeploymentImpl, error::ApiError};

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/claude-usage", get(get_claude_usage))
        .route("/claude-usage/live", get(get_claude_usage_live))
}

// ---------------------------------------------------------------------------
// Original endpoint — returns the raw stats-cache.json
// ---------------------------------------------------------------------------

async fn get_claude_usage() -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let stats_path = expand_tilde("~/.claude/stats-cache.json");
    let content = tokio::fs::read_to_string(&stats_path).await?;
    let data: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(ResponseJson(ApiResponse::success(data)))
}

// ---------------------------------------------------------------------------
// Live endpoint — reads OAuth token from Keychain and calls Anthropic usage API
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct KeychainCredentials {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: Option<OAuthTokens>,
}

#[derive(Deserialize)]
struct OAuthTokens {
    #[serde(rename = "accessToken")]
    access_token: String,
}

// Deserialized from Anthropic API (snake_case)
#[derive(Debug, Deserialize)]
struct ApiUsageWindow {
    utilization: f64,
    resets_at: String,
}

#[derive(Debug, Deserialize)]
struct ApiExtraUsage {
    is_enabled: bool,
    monthly_limit: f64,
    used_credits: f64,
    utilization: f64,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsageResponse {
    five_hour: Option<ApiUsageWindow>,
    seven_day: Option<ApiUsageWindow>,
    seven_day_opus: Option<ApiUsageWindow>,
    seven_day_sonnet: Option<ApiUsageWindow>,
    extra_usage: Option<ApiExtraUsage>,
}

// Serialized to frontend (camelCase)
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UsageWindow {
    utilization: f64,
    resets_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExtraUsage {
    is_enabled: bool,
    monthly_limit: f64,
    used_credits: f64,
    utilization: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LiveUsageResponse {
    five_hour: Option<UsageWindow>,
    seven_day: Option<UsageWindow>,
    seven_day_opus: Option<UsageWindow>,
    seven_day_sonnet: Option<UsageWindow>,
    extra_usage: Option<ExtraUsage>,
}

impl From<ApiUsageWindow> for UsageWindow {
    fn from(w: ApiUsageWindow) -> Self {
        Self { utilization: w.utilization, resets_at: w.resets_at }
    }
}

impl From<ApiExtraUsage> for ExtraUsage {
    fn from(e: ApiExtraUsage) -> Self {
        Self {
            is_enabled: e.is_enabled,
            monthly_limit: e.monthly_limit,
            used_credits: e.used_credits,
            utilization: e.utilization,
        }
    }
}

/// Read the OAuth access token.
/// Tries ~/.claude/.credentials.json first (works in background processes),
/// then falls back to macOS Keychain (may prompt for access).
async fn read_oauth_token() -> Result<String, ApiError> {
    // 1. Try credentials file (written by keychain dump or Claude Code)
    let creds_path = expand_tilde("~/.claude/.credentials.json");
    if let Ok(content) = tokio::fs::read_to_string(&creds_path).await {
        if let Ok(creds) = serde_json::from_str::<KeychainCredentials>(content.trim()) {
            if let Some(token) = creds.claude_ai_oauth.map(|o| o.access_token) {
                return Ok(token);
            }
        }
    }

    // 2. Fall back to macOS Keychain
    let output = tokio::process::Command::new("security")
        .args(["find-generic-password", "-s", "Claude Code-credentials", "-w"])
        .output()
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Claude Code credentials not found",
        )
        .into());
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    let creds: KeychainCredentials = serde_json::from_str(raw.trim())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    creds
        .claude_ai_oauth
        .map(|o| o.access_token)
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No OAuth access token found",
            )
            .into()
        })
}

async fn get_claude_usage_live(
) -> Result<ResponseJson<ApiResponse<LiveUsageResponse>>, ApiError> {
    let token = read_oauth_token().await?;

    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.anthropic.com/api/oauth/usage")
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("User-Agent", "claude-code/2.1.77")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Anthropic API returned {}: {}", status, body),
        )
        .into());
    }

    let usage: AnthropicUsageResponse = resp
        .json()
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

    Ok(ResponseJson(ApiResponse::success(LiveUsageResponse {
        five_hour: usage.five_hour.map(Into::into),
        seven_day: usage.seven_day.map(Into::into),
        seven_day_opus: usage.seven_day_opus.map(Into::into),
        seven_day_sonnet: usage.seven_day_sonnet.map(Into::into),
        extra_usage: usage.extra_usage.map(Into::into),
    })))
}
