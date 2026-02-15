use axum::{Router, response::Json as ResponseJson, routing::get};
use utils::{path::expand_tilde, response::ApiResponse};

use crate::{DeploymentImpl, error::ApiError};

pub fn router() -> Router<DeploymentImpl> {
    Router::new().route("/claude-usage", get(get_claude_usage))
}

async fn get_claude_usage() -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let stats_path = expand_tilde("~/.claude/stats-cache.json");
    let content = tokio::fs::read_to_string(&stats_path).await?;
    let data: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(ResponseJson(ApiResponse::success(data)))
}
