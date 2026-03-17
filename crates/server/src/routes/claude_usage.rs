use std::path::PathBuf;

use axum::{Router, response::Json as ResponseJson, routing::get};
use chrono::{DateTime, Duration, Utc};
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
// Live endpoint — reads active sessions and computes rolling-window usage
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LiveUsageResponse {
    /// Messages in the current 5-hour rolling window (all projects)
    window_messages: u64,
    /// Timestamp of the first message in the current window
    window_start: Option<String>,
    /// Estimated reset time (window_start + 5h), or None if no messages
    window_reset: Option<String>,
    /// Per-session breakdown
    sessions: Vec<SessionUsage>,
    /// Messages sent today (from JSONL, not stats-cache)
    today_messages: u64,
    /// Messages sent in the last 7 days
    week_messages: u64,
    /// Output tokens in the current window
    window_output_tokens: u64,
    /// Output tokens today
    today_output_tokens: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionUsage {
    session_id: String,
    project: String,
    started_at: String,
    messages: u64,
    output_tokens: u64,
}

#[derive(Deserialize)]
struct SessionFile {
    #[serde(rename = "sessionId")]
    session_id: String,
    #[allow(dead_code)]
    cwd: Option<String>,
    #[serde(rename = "startedAt")]
    started_at: Option<u64>,
}

async fn get_claude_usage_live(
) -> Result<ResponseJson<ApiResponse<LiveUsageResponse>>, ApiError> {
    let now = Utc::now();
    let window_duration = Duration::hours(5);
    let window_boundary = now - window_duration;
    let today_start = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();
    let week_start = (now - Duration::days(7))
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();

    let claude_dir = expand_tilde("~/.claude");
    let projects_dir = PathBuf::from(&claude_dir).join("projects");
    let sessions_dir = PathBuf::from(&claude_dir).join("sessions");

    // 1. Discover active sessions
    let mut active_sessions: Vec<SessionFile> = Vec::new();
    if let Ok(mut entries) = tokio::fs::read_dir(&sessions_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    if let Ok(sf) = serde_json::from_str::<SessionFile>(&content) {
                        active_sessions.push(sf);
                    }
                }
            }
        }
    }

    // 2. Scan all project JSONL files for recent messages
    let mut window_messages: u64 = 0;
    let mut window_output_tokens: u64 = 0;
    let mut today_messages: u64 = 0;
    let mut today_output_tokens: u64 = 0;
    let mut week_messages: u64 = 0;
    let mut earliest_in_window: Option<DateTime<Utc>> = None;
    let mut session_usages: Vec<SessionUsage> = Vec::new();

    // Collect all JSONL files across all projects
    let mut jsonl_files: Vec<(PathBuf, String)> = Vec::new(); // (path, project_name)
    if let Ok(mut project_entries) = tokio::fs::read_dir(&projects_dir).await {
        while let Ok(Some(project_entry)) = project_entries.next_entry().await {
            let project_path = project_entry.path();
            if !project_path.is_dir() {
                continue;
            }
            let project_name = project_entry
                .file_name()
                .to_string_lossy()
                .replace('-', "/")
                .trim_start_matches('/')
                .to_string();

            if let Ok(mut files) = tokio::fs::read_dir(&project_path).await {
                while let Ok(Some(file_entry)) = files.next_entry().await {
                    let file_path = file_entry.path();
                    if file_path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                        jsonl_files.push((file_path, project_name.clone()));
                    }
                }
            }
        }
    }

    // Process JSONL files — only read those modified in the last 7 days
    for (jsonl_path, project_name) in &jsonl_files {
        let metadata = match tokio::fs::metadata(&jsonl_path).await {
            Ok(m) => m,
            Err(_) => continue,
        };
        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| {
                let dur = t.duration_since(std::time::UNIX_EPOCH).ok()?;
                DateTime::from_timestamp(dur.as_secs() as i64, dur.subsec_nanos())
            })
            .unwrap_or(now);

        if modified < week_start {
            continue;
        }

        let content = match tokio::fs::read_to_string(&jsonl_path).await {
            Ok(c) => c,
            Err(_) => continue,
        };

        let session_id = jsonl_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let mut sess_messages: u64 = 0;
        let mut sess_output_tokens: u64 = 0;
        let mut sess_started: Option<String> = None;
        let mut sess_in_window = false;

        for line in content.lines() {
            let entry: serde_json::Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let msg_type = entry.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if msg_type != "assistant" {
                continue;
            }

            let ts_str = match entry.get("timestamp").and_then(|v| v.as_str()) {
                Some(s) => s,
                None => continue,
            };
            let ts = match ts_str.parse::<DateTime<Utc>>() {
                Ok(t) => t,
                Err(_) => continue,
            };

            if sess_started.is_none() {
                sess_started = Some(ts_str.to_string());
            }

            let output_tokens = entry
                .get("message")
                .or(Some(&entry))
                .and_then(|m| m.get("usage"))
                .and_then(|u| u.get("output_tokens"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            // Week
            if ts >= week_start {
                week_messages += 1;
            }

            // Today
            if ts >= today_start {
                today_messages += 1;
                today_output_tokens += output_tokens;
            }

            // 5h window
            if ts >= window_boundary {
                window_messages += 1;
                window_output_tokens += output_tokens;
                sess_messages += 1;
                sess_output_tokens += output_tokens;
                sess_in_window = true;

                match &earliest_in_window {
                    None => earliest_in_window = Some(ts),
                    Some(e) if ts < *e => earliest_in_window = Some(ts),
                    _ => {}
                }
            }
        }

        if sess_in_window {
            // Check if this session is currently active
            let is_active = active_sessions
                .iter()
                .any(|s| s.session_id == session_id);

            let started_at = if is_active {
                active_sessions
                    .iter()
                    .find(|s| s.session_id == session_id)
                    .and_then(|s| s.started_at)
                    .and_then(|ms| DateTime::from_timestamp_millis(ms as i64))
                    .map(|dt| dt.to_rfc3339())
                    .or(sess_started)
                    .unwrap_or_default()
            } else {
                sess_started.unwrap_or_default()
            };

            session_usages.push(SessionUsage {
                session_id,
                project: project_name.clone(),
                started_at,
                messages: sess_messages,
                output_tokens: sess_output_tokens,
            });
        }
    }

    // Sort sessions by started_at descending
    session_usages.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    let window_start = earliest_in_window.map(|t| t.to_rfc3339());
    let window_reset = earliest_in_window.map(|t| (t + window_duration).to_rfc3339());

    Ok(ResponseJson(ApiResponse::success(LiveUsageResponse {
        window_messages,
        window_start,
        window_reset,
        sessions: session_usages,
        today_messages,
        week_messages,
        window_output_tokens,
        today_output_tokens,
    })))
}
