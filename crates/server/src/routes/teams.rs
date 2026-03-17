use std::time::Duration;

use axum::{
    Json, Router,
    extract::{Path as AxumPath, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
};
use db::models::{
    custom_command::CustomCommand,
    execution_process::{ExecutionProcess, ExecutionProcessRunReason, ExecutionProcessStatus},
    repo::Repo,
    scratch::DraftFollowUpData,
    session::{CreateSession, Session},
    teams::{
        CreateTeamsChannelMapping, CreateTeamsChannelRepo, TeamsChannelMapping, TeamsChannelRepo,
        TeamsConversation, UpdateTeamsChannelMapping,
    },
    workspace::{CreateWorkspace, Workspace},
    workspace_repo::{CreateWorkspaceRepo, WorkspaceRepo},
};
use deployment::Deployment;
use executors::{
    actions::{
        ExecutorAction, ExecutorActionType,
        script::{ScriptContext, ScriptRequest, ScriptRequestLanguage},
    },
    profile::ExecutorConfig,
};
use serde::Deserialize;
use services::services::{
    container::ContainerService,
    diff_stream,
    teams::{
        TeamsCommand, TeamsService, cards,
        types::{TeamsActivity, TeamsConfigResponse, TeamsError, UpdateTeamsConfigRequest},
    },
};
use utils::approvals::{ApprovalOutcome, ApprovalResponse};
use uuid::Uuid;

use crate::DeploymentImpl;

// ── Rate Limit Constants ────────────────────────────────────────────────────

const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);
const WEBHOOK_GLOBAL_LIMIT: usize = 30;
const WEBHOOK_PER_CHANNEL_LIMIT: usize = 10;

// ── Public Router (webhook — outside relay signed routes) ───────────────────

pub fn public_router() -> Router<DeploymentImpl> {
    Router::new().route("/teams/webhook", post(handle_teams_webhook))
}

// ── Management Router (inside relay signed routes) ──────────────────────────

pub fn management_router() -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/teams/config",
            get(get_teams_config).put(update_teams_config),
        )
        .route(
            "/teams/channels",
            get(list_channel_mappings).post(create_channel_mapping),
        )
        .route(
            "/teams/channels/{id}",
            get(get_channel_mapping)
                .put(update_channel_mapping)
                .delete(delete_channel_mapping),
        )
        .route(
            "/teams/channels/{id}/repos",
            get(list_channel_repos).post(add_channel_repo),
        )
        .route(
            "/teams/channels/{channel_id}/repos/{repo_id}",
            delete(remove_channel_repo),
        )
}

// ── Webhook Handler ─────────────────────────────────────────────────────────

async fn handle_teams_webhook(
    State(deployment): State<DeploymentImpl>,
    headers: HeaderMap,
    Json(activity): Json<TeamsActivity>,
) -> impl IntoResponse {
    // Check Teams is enabled and get config
    let config = deployment.config().read().await;
    if !config.teams.enabled {
        return StatusCode::SERVICE_UNAVAILABLE;
    }

    let bot_app_id = match config.teams.bot_app_id.as_ref() {
        Some(id) => id.clone(),
        None => return StatusCode::SERVICE_UNAVAILABLE,
    };

    // Validate JWT from Bot Framework
    let auth_header = match headers.get("Authorization").and_then(|v| v.to_str().ok()) {
        Some(h) => h.to_string(),
        None => return StatusCode::UNAUTHORIZED,
    };

    let teams = match deployment.teams_service() {
        Some(t) => t.clone(),
        None => return StatusCode::SERVICE_UNAVAILABLE,
    };

    if let Err(e) = teams
        .validate_incoming_token(&auth_header, &bot_app_id)
        .await
    {
        tracing::warn!("Teams webhook JWT validation failed: {}", e);
        return StatusCode::UNAUTHORIZED;
    }

    // Rate limiting — global
    if deployment
        .trusted_key_auth()
        .enforce_rate_limit(
            "teams_webhook_global",
            WEBHOOK_GLOBAL_LIMIT,
            RATE_LIMIT_WINDOW,
        )
        .await
        .is_err()
    {
        return StatusCode::TOO_MANY_REQUESTS;
    }

    // Rate limiting — per channel
    let channel_id = activity.conversation.id.clone();
    let channel_bucket = format!("teams_channel_{}", channel_id);
    if deployment
        .trusted_key_auth()
        .enforce_rate_limit(
            &channel_bucket,
            WEBHOOK_PER_CHANNEL_LIMIT,
            RATE_LIMIT_WINDOW,
        )
        .await
        .is_err()
    {
        return StatusCode::TOO_MANY_REQUESTS;
    }

    let teams_config = config.teams.clone();
    drop(config); // Release read lock before spawning

    // Respond 200 immediately; process async (Teams has 5s deadline)
    let deployment_clone = deployment.clone();
    tokio::spawn(async move {
        if let Err(e) = process_activity(deployment_clone, teams, teams_config, activity).await {
            tracing::error!("Teams activity processing error: {}", e);
        }
    });

    StatusCode::OK
}

// ── Activity Processing ─────────────────────────────────────────────────────

async fn process_activity(
    deployment: DeploymentImpl,
    teams: TeamsService,
    teams_config: services::services::config::TeamsConfig,
    activity: TeamsActivity,
) -> Result<(), TeamsError> {
    match activity.r#type.as_str() {
        "message" => process_message(deployment, teams, teams_config, activity).await,
        "invoke" => process_invoke(deployment, teams, teams_config, activity).await,
        other => {
            tracing::debug!("Ignoring Teams activity type: {}", other);
            Ok(())
        }
    }
}

async fn process_message(
    deployment: DeploymentImpl,
    teams: TeamsService,
    teams_config: services::services::config::TeamsConfig,
    activity: TeamsActivity,
) -> Result<(), TeamsError> {
    let raw_text = activity.text.as_deref().unwrap_or("");
    let stripped = TeamsService::strip_mentions(raw_text);
    let pool = &deployment.db().pool;
    let custom_command_names = CustomCommand::find_all_names(pool)
        .await
        .unwrap_or_default();
    let command = TeamsService::parse_command(&stripped, Some(&custom_command_names));

    let service_url = activity
        .service_url
        .as_deref()
        .ok_or_else(|| TeamsError::HttpError("missing service_url in activity".to_string()))?;
    TeamsService::validate_service_url(service_url)?;

    let conversation_id = &activity.conversation.id;
    let reply_to_id = activity.id.as_deref();

    match command {
        TeamsCommand::Help => {
            let custom_cmds = CustomCommand::find_all(pool).await.unwrap_or_default();
            let custom_cmd_info: Vec<(String, Option<String>, String)> = custom_cmds
                .into_iter()
                .map(|c| (c.name, c.description, c.mode))
                .collect();
            let card = cards::build_help_card(&custom_cmd_info);
            teams
                .send_card_reply(
                    service_url,
                    conversation_id,
                    reply_to_id,
                    card,
                    &teams_config,
                )
                .await?;
        }

        TeamsCommand::List => {
            let mapping = TeamsChannelMapping::find_by_channel_id(pool, conversation_id).await?;

            if let Some(mapping) = mapping {
                // Find recent workspaces via teams_conversations for this channel
                let conversations: Vec<TeamsConversation> = sqlx::query_as::<_, TeamsConversation>(
                    r#"SELECT tc.id, tc.workspace_id, tc.channel_mapping_id,
                                  tc.conversation_ref, tc.reply_to_activity_id,
                                  tc.teams_activity_id, tc.created_at
                           FROM teams_conversations tc
                           WHERE tc.channel_mapping_id = ?
                           ORDER BY tc.created_at DESC
                           LIMIT 10"#,
                )
                .bind(mapping.id)
                .fetch_all(pool)
                .await?;

                let mut items = Vec::new();
                for conv in &conversations {
                    if let Some(ws) = Workspace::find_by_id(pool, conv.workspace_id).await? {
                        let status = if ws.archived { "Completed" } else { "Active" };
                        items.push(cards::WorkspaceListItem {
                            name: ws.name.unwrap_or_else(|| ws.branch.clone()),
                            status: status.to_string(),
                            created_at: ws.created_at.format("%Y-%m-%d %H:%M").to_string(),
                        });
                    }
                }

                let card = cards::build_list_card(&items);
                teams
                    .send_card_reply(
                        service_url,
                        conversation_id,
                        reply_to_id,
                        card,
                        &teams_config,
                    )
                    .await?;
            } else {
                teams
                    .send_text_reply(
                        service_url,
                        conversation_id,
                        reply_to_id,
                        "No channel mapping configured for this channel.",
                        &teams_config,
                    )
                    .await?;
            }
        }

        TeamsCommand::Status => {
            let active = find_active_workspace(pool, conversation_id).await?;
            match active {
                Some((ws, _conv)) => {
                    let repos = WorkspaceRepo::find_repos_for_workspace(pool, ws.id).await?;
                    let repo_names: Vec<String> = repos.iter().map(|r| r.name.clone()).collect();
                    let status = if ws.archived { "Completed" } else { "Active" };
                    let card = cards::build_status_card(
                        &ws.name.clone().unwrap_or_else(|| ws.branch.clone()),
                        status,
                        Some(&ws.branch),
                        &repo_names,
                        None,
                        None,
                    );
                    teams
                        .send_card_reply(
                            service_url,
                            conversation_id,
                            reply_to_id,
                            card,
                            &teams_config,
                        )
                        .await?;
                }
                None => {
                    teams
                        .send_text_reply(
                            service_url,
                            conversation_id,
                            reply_to_id,
                            "No active workspace for this channel.",
                            &teams_config,
                        )
                        .await?;
                }
            }
        }

        TeamsCommand::Branch => {
            let active = find_active_workspace(pool, conversation_id).await?;
            match active {
                Some((ws, _conv)) => {
                    let workspace_repos = WorkspaceRepo::find_by_workspace_id(pool, ws.id).await?;
                    let mut repo_info: Vec<(String, String)> = Vec::new();
                    for wr in &workspace_repos {
                        if let Some(repo) = Repo::find_by_id(pool, wr.repo_id).await? {
                            repo_info.push((repo.name, wr.target_branch.clone()));
                        }
                    }
                    let card = cards::build_branch_card(&ws.branch, &repo_info);
                    teams
                        .send_card_reply(
                            service_url,
                            conversation_id,
                            reply_to_id,
                            card,
                            &teams_config,
                        )
                        .await?;
                }
                None => {
                    teams
                        .send_text_reply(
                            service_url,
                            conversation_id,
                            reply_to_id,
                            "No active workspace for this channel.",
                            &teams_config,
                        )
                        .await?;
                }
            }
        }

        TeamsCommand::Diff => {
            let active = find_active_workspace(pool, conversation_id).await?;
            match active {
                Some((ws, _conv)) => {
                    // Use the diff_stream utility to compute diff stats
                    let stats = diff_stream::compute_diff_stats(pool, deployment.git(), &ws).await;
                    match stats {
                        Some(s) => {
                            let card = cards::build_diff_card(
                                s.files_changed,
                                s.lines_added,
                                s.lines_removed,
                                None,
                                None,
                            );
                            teams
                                .send_card_reply(
                                    service_url,
                                    conversation_id,
                                    reply_to_id,
                                    card,
                                    &teams_config,
                                )
                                .await?;
                        }
                        None => {
                            teams
                                .send_text_reply(
                                    service_url,
                                    conversation_id,
                                    reply_to_id,
                                    "No diff data available.",
                                    &teams_config,
                                )
                                .await?;
                        }
                    }
                }
                None => {
                    teams
                        .send_text_reply(
                            service_url,
                            conversation_id,
                            reply_to_id,
                            "No active workspace for this channel.",
                            &teams_config,
                        )
                        .await?;
                }
            }
        }

        TeamsCommand::Done => {
            handle_done_command(
                pool,
                &deployment,
                &teams,
                service_url,
                conversation_id,
                reply_to_id,
                &teams_config,
            )
            .await?;
        }

        TeamsCommand::DoneMerge => {
            // For merge via bot, just stop + archive and tell user to merge from dashboard
            let active = find_active_workspace(pool, conversation_id).await?;
            if let Some((ws, _conv)) = active {
                deployment.container().try_stop(&ws, false).await;
                Workspace::update(pool, ws.id, Some(true), None, None)
                    .await
                    .map_err(|e| TeamsError::Database(e.into()))?;
                let card = cards::build_action_confirmation_card(
                    "Done",
                    &ws.name.unwrap_or_else(|| ws.branch.clone()),
                    Some("Workspace archived. Use the Vibe Kanban dashboard to merge."),
                );
                teams
                    .send_card_reply(
                        service_url,
                        conversation_id,
                        reply_to_id,
                        card,
                        &teams_config,
                    )
                    .await?;
            } else {
                teams
                    .send_text_reply(
                        service_url,
                        conversation_id,
                        reply_to_id,
                        "No active workspace to close.",
                        &teams_config,
                    )
                    .await?;
            }
        }

        TeamsCommand::DonePr => {
            // For PR via bot, stop + archive and tell user to create PR from dashboard
            let active = find_active_workspace(pool, conversation_id).await?;
            if let Some((ws, _conv)) = active {
                deployment.container().try_stop(&ws, false).await;
                Workspace::update(pool, ws.id, Some(true), None, None)
                    .await
                    .map_err(|e| TeamsError::Database(e.into()))?;
                let card = cards::build_action_confirmation_card(
                    "Done",
                    &ws.name.unwrap_or_else(|| ws.branch.clone()),
                    Some("Workspace archived. Use the Vibe Kanban dashboard to create a PR."),
                );
                teams
                    .send_card_reply(
                        service_url,
                        conversation_id,
                        reply_to_id,
                        card,
                        &teams_config,
                    )
                    .await?;
            } else {
                teams
                    .send_text_reply(
                        service_url,
                        conversation_id,
                        reply_to_id,
                        "No active workspace to close.",
                        &teams_config,
                    )
                    .await?;
            }
        }

        TeamsCommand::Push => {
            let active = find_active_workspace(pool, conversation_id).await?;
            match active {
                Some((ws, _conv)) => {
                    let repos = WorkspaceRepo::find_repos_for_workspace(pool, ws.id).await?;
                    let container_ref = deployment
                        .container()
                        .ensure_container_exists(&ws)
                        .await
                        .map_err(|e| {
                        TeamsError::HttpError(format!("container error: {}", e))
                    })?;
                    let workspace_path = std::path::Path::new(&container_ref);

                    let mut push_ok = true;
                    for r in &repos {
                        let worktree_path = workspace_path.join(&r.name);
                        if let Err(e) =
                            deployment
                                .git()
                                .push_to_remote(&worktree_path, &ws.branch, false)
                        {
                            tracing::error!("Push failed for {}: {}", r.name, e);
                            push_ok = false;
                        }
                    }

                    if push_ok {
                        let card = cards::build_action_confirmation_card(
                            "Pushed",
                            &ws.name.unwrap_or_else(|| ws.branch.clone()),
                            Some(&format!("Branch `{}` pushed to remote.", ws.branch)),
                        );
                        teams
                            .send_card_reply(
                                service_url,
                                conversation_id,
                                reply_to_id,
                                card,
                                &teams_config,
                            )
                            .await?;
                    } else {
                        teams
                            .send_text_reply(
                                service_url,
                                conversation_id,
                                reply_to_id,
                                "Push failed for one or more repos. Check the dashboard.",
                                &teams_config,
                            )
                            .await?;
                    }
                }
                None => {
                    teams
                        .send_text_reply(
                            service_url,
                            conversation_id,
                            reply_to_id,
                            "No active workspace for this channel.",
                            &teams_config,
                        )
                        .await?;
                }
            }
        }

        TeamsCommand::Cancel => {
            let active = find_active_workspace(pool, conversation_id).await?;
            match active {
                Some((ws, _conv)) => {
                    deployment.container().try_stop(&ws, false).await;
                    Workspace::update(pool, ws.id, Some(true), None, None)
                        .await
                        .map_err(|e| TeamsError::Database(e.into()))?;

                    let card = cards::build_action_confirmation_card(
                        "Cancelled",
                        &ws.name.unwrap_or_else(|| ws.branch.clone()),
                        None,
                    );
                    teams
                        .send_card_reply(
                            service_url,
                            conversation_id,
                            reply_to_id,
                            card,
                            &teams_config,
                        )
                        .await?;
                }
                None => {
                    teams
                        .send_text_reply(
                            service_url,
                            conversation_id,
                            reply_to_id,
                            "No active workspace to cancel.",
                            &teams_config,
                        )
                        .await?;
                }
            }
        }

        TeamsCommand::Retry => {
            let active = find_active_workspace(pool, conversation_id).await?;
            match active {
                Some((ws, _conv)) => {
                    let config = deployment.config().read().await;
                    let exec_config = ExecutorConfig::new(config.executor_profile.executor.clone());
                    drop(config);

                    match deployment
                        .container()
                        .start_workspace(
                            &ws,
                            exec_config,
                            "Continue the previous task.".to_string(),
                        )
                        .await
                    {
                        Ok(_) => {
                            teams
                                .send_text_reply(
                                    service_url,
                                    conversation_id,
                                    reply_to_id,
                                    "Retrying workspace execution...",
                                    &teams_config,
                                )
                                .await?;
                        }
                        Err(e) => {
                            tracing::error!("Retry failed: {}", e);
                            teams
                                .send_text_reply(
                                    service_url,
                                    conversation_id,
                                    reply_to_id,
                                    "Retry failed. Check the Vibe Kanban dashboard.",
                                    &teams_config,
                                )
                                .await?;
                        }
                    }
                }
                None => {
                    teams
                        .send_text_reply(
                            service_url,
                            conversation_id,
                            reply_to_id,
                            "No workspace to retry.",
                            &teams_config,
                        )
                        .await?;
                }
            }
        }

        TeamsCommand::Approve => {
            let msg = try_respond_approval(&deployment, conversation_id, true, None).await;
            teams
                .send_text_reply(
                    service_url,
                    conversation_id,
                    reply_to_id,
                    &msg,
                    &teams_config,
                )
                .await?;
        }

        TeamsCommand::Reject { reason } => {
            let msg =
                try_respond_approval(&deployment, conversation_id, false, reason.as_deref()).await;
            teams
                .send_text_reply(
                    service_url,
                    conversation_id,
                    reply_to_id,
                    &msg,
                    &teams_config,
                )
                .await?;
        }

        TeamsCommand::CustomStart { ref name } => {
            let active = find_active_workspace(pool, conversation_id).await?;
            match active {
                Some((ws, _conv)) => {
                    let cmd = CustomCommand::find_by_name(pool, name).await?;
                    match cmd {
                        Some(cmd) => {
                            let session =
                                match Session::find_latest_by_workspace_id(pool, ws.id).await? {
                                    Some(s) => s,
                                    None => Session::create(
                                        pool,
                                        &CreateSession {
                                            executor: Some(format!("custom-cmd-{}", cmd.name)),
                                            name: None,
                                        },
                                        Uuid::new_v4(),
                                        ws.id,
                                    )
                                    .await
                                    .map_err(|e| TeamsError::HttpError(e.to_string()))?,
                                };

                            let executor_action = ExecutorAction::new(
                                ExecutorActionType::ScriptRequest(ScriptRequest {
                                    script: cmd.script.clone(),
                                    language: ScriptRequestLanguage::Bash,
                                    context: ScriptContext::DevServer,
                                    working_dir: None,
                                }),
                                None,
                            );

                            match deployment
                                .container()
                                .start_execution(
                                    &ws,
                                    &session,
                                    &executor_action,
                                    &ExecutionProcessRunReason::DevServer,
                                )
                                .await
                            {
                                Ok(_) => {
                                    teams
                                        .send_text_reply(
                                            service_url,
                                            conversation_id,
                                            reply_to_id,
                                            &format!("Started command `{}`.", cmd.name),
                                            &teams_config,
                                        )
                                        .await?;
                                }
                                Err(e) => {
                                    tracing::error!("Custom command start failed: {}", e);
                                    teams
                                        .send_text_reply(
                                            service_url,
                                            conversation_id,
                                            reply_to_id,
                                            &format!("Failed to start `{}`: {}", cmd.name, e),
                                            &teams_config,
                                        )
                                        .await?;
                                }
                            }
                        }
                        None => {
                            teams
                                .send_text_reply(
                                    service_url,
                                    conversation_id,
                                    reply_to_id,
                                    &format!("Custom command `{}` not found.", name),
                                    &teams_config,
                                )
                                .await?;
                        }
                    }
                }
                None => {
                    teams
                        .send_text_reply(
                            service_url,
                            conversation_id,
                            reply_to_id,
                            "No active workspace for this channel.",
                            &teams_config,
                        )
                        .await?;
                }
            }
        }

        TeamsCommand::CustomStop { ref name } => {
            let active = find_active_workspace(pool, conversation_id).await?;
            match active {
                Some((ws, _conv)) => {
                    // Find running dev servers and stop the one matching this custom command
                    let running =
                        ExecutionProcess::find_running_dev_servers_by_workspace(pool, ws.id)
                            .await
                            .unwrap_or_default();

                    let mut stopped = false;
                    for proc in &running {
                        // Match by checking the script content of the execution
                        // The most recent dev server for this command name is the one we want
                        if let Some(cmd) = CustomCommand::find_by_name(pool, name).await? {
                            if let Ok(Some(session)) =
                                Session::find_by_id(pool, proc.session_id).await
                            {
                                let expected_executor = format!("custom-cmd-{}", cmd.name);
                                if session
                                    .executor
                                    .as_deref()
                                    .is_some_and(|e| e == expected_executor)
                                {
                                    if let Err(e) = deployment
                                        .container()
                                        .stop_execution(proc, ExecutionProcessStatus::Killed)
                                        .await
                                    {
                                        tracing::error!(
                                            "Failed to stop custom command {}: {}",
                                            name,
                                            e
                                        );
                                    }
                                    stopped = true;
                                    break;
                                }
                            }
                        }
                    }

                    if stopped {
                        teams
                            .send_text_reply(
                                service_url,
                                conversation_id,
                                reply_to_id,
                                &format!("Stopped command `{}`.", name),
                                &teams_config,
                            )
                            .await?;
                    } else {
                        teams
                            .send_text_reply(
                                service_url,
                                conversation_id,
                                reply_to_id,
                                &format!("No running instance of `{}` found.", name),
                                &teams_config,
                            )
                            .await?;
                    }
                }
                None => {
                    teams
                        .send_text_reply(
                            service_url,
                            conversation_id,
                            reply_to_id,
                            "No active workspace for this channel.",
                            &teams_config,
                        )
                        .await?;
                }
            }
        }

        TeamsCommand::CustomRun { ref name } => {
            let active = find_active_workspace(pool, conversation_id).await?;
            match active {
                Some((ws, _conv)) => {
                    let cmd = CustomCommand::find_by_name(pool, name).await?;
                    match cmd {
                        Some(cmd) => {
                            if cmd.mode == "oneshot" {
                                // One-shot: spawn directly, wait for output, send result
                                let container_ref = deployment
                                    .container()
                                    .ensure_container_exists(&ws)
                                    .await
                                    .map_err(|e| {
                                        TeamsError::HttpError(format!("container error: {}", e))
                                    })?;
                                let workspace_path = std::path::Path::new(&container_ref);

                                let result = tokio::time::timeout(
                                    Duration::from_secs(60),
                                    tokio::process::Command::new("bash")
                                        .arg("-c")
                                        .arg(&cmd.script)
                                        .current_dir(workspace_path)
                                        .output(),
                                )
                                .await;

                                let (output_text, exit_code, timed_out) = match result {
                                    Ok(Ok(output)) => {
                                        let stdout = String::from_utf8_lossy(&output.stdout);
                                        let stderr = String::from_utf8_lossy(&output.stderr);
                                        let mut combined = String::new();
                                        if !stdout.is_empty() {
                                            combined.push_str(&stdout);
                                        }
                                        if !stderr.is_empty() {
                                            if !combined.is_empty() {
                                                combined.push('\n');
                                            }
                                            combined.push_str(&stderr);
                                        }
                                        if combined.len() > 2000 {
                                            combined.truncate(2000);
                                            combined.push_str("\n... (truncated)");
                                        }
                                        (combined, output.status.code(), false)
                                    }
                                    Ok(Err(e)) => {
                                        (format!("Failed to execute: {}", e), None, false)
                                    }
                                    Err(_) => (
                                        "Command timed out after 60 seconds.".to_string(),
                                        None,
                                        true,
                                    ),
                                };

                                let card = cards::build_command_output_card(
                                    &cmd.name,
                                    &output_text,
                                    exit_code,
                                    timed_out,
                                );
                                teams
                                    .send_card_reply(
                                        service_url,
                                        conversation_id,
                                        reply_to_id,
                                        card,
                                        &teams_config,
                                    )
                                    .await?;
                            } else {
                                // Background mode without start/stop: treat as CustomStart
                                let session = match Session::find_latest_by_workspace_id(
                                    pool, ws.id,
                                )
                                .await?
                                {
                                    Some(s) => s,
                                    None => Session::create(
                                        pool,
                                        &CreateSession {
                                            executor: Some(format!("custom-cmd-{}", cmd.name)),
                                            name: None,
                                        },
                                        Uuid::new_v4(),
                                        ws.id,
                                    )
                                    .await
                                    .map_err(|e| TeamsError::HttpError(e.to_string()))?,
                                };

                                let executor_action = ExecutorAction::new(
                                    ExecutorActionType::ScriptRequest(ScriptRequest {
                                        script: cmd.script.clone(),
                                        language: ScriptRequestLanguage::Bash,
                                        context: ScriptContext::DevServer,
                                        working_dir: None,
                                    }),
                                    None,
                                );

                                match deployment
                                    .container()
                                    .start_execution(
                                        &ws,
                                        &session,
                                        &executor_action,
                                        &ExecutionProcessRunReason::DevServer,
                                    )
                                    .await
                                {
                                    Ok(_) => {
                                        teams
                                            .send_text_reply(
                                                service_url,
                                                conversation_id,
                                                reply_to_id,
                                                &format!("Started command `{}`.", cmd.name),
                                                &teams_config,
                                            )
                                            .await?;
                                    }
                                    Err(e) => {
                                        tracing::error!("Custom command start failed: {}", e);
                                        teams
                                            .send_text_reply(
                                                service_url,
                                                conversation_id,
                                                reply_to_id,
                                                &format!("Failed to start `{}`: {}", cmd.name, e),
                                                &teams_config,
                                            )
                                            .await?;
                                    }
                                }
                            }
                        }
                        None => {
                            teams
                                .send_text_reply(
                                    service_url,
                                    conversation_id,
                                    reply_to_id,
                                    &format!("Custom command `{}` not found.", name),
                                    &teams_config,
                                )
                                .await?;
                        }
                    }
                }
                None => {
                    teams
                        .send_text_reply(
                            service_url,
                            conversation_id,
                            reply_to_id,
                            "No active workspace for this channel.",
                            &teams_config,
                        )
                        .await?;
                }
            }
        }

        TeamsCommand::Plan { ref prompt } | TeamsCommand::FreeText { text: ref prompt } => {
            let sanitized = TeamsService::sanitize_prompt(prompt)?;
            let is_plan = matches!(command, TeamsCommand::Plan { .. });

            // Check for active workspace
            let active = find_active_workspace(pool, conversation_id).await?;

            if let Some((ws, _conv)) = active {
                // Send as follow-up message to active workspace
                let config = deployment.config().read().await;
                let exec_config = ExecutorConfig::new(config.executor_profile.executor.clone());
                drop(config);

                deployment.queued_message_service().queue_message(
                    ws.id,
                    DraftFollowUpData {
                        message: sanitized.clone(),
                        executor_config: exec_config,
                    },
                );

                let card = cards::build_followup_ack_card(
                    &sanitized,
                    &ws.name.unwrap_or_else(|| ws.branch.clone()),
                );
                teams
                    .send_card_reply(
                        service_url,
                        conversation_id,
                        reply_to_id,
                        card,
                        &teams_config,
                    )
                    .await?;
            } else {
                // Create a new workspace
                create_workspace_from_teams(
                    &deployment,
                    &teams,
                    &teams_config,
                    pool,
                    service_url,
                    conversation_id,
                    reply_to_id,
                    activity.id.as_deref(),
                    &sanitized,
                    is_plan,
                )
                .await?;
            }
        }
    }

    Ok(())
}

async fn process_invoke(
    deployment: DeploymentImpl,
    _teams: TeamsService,
    _teams_config: services::services::config::TeamsConfig,
    activity: TeamsActivity,
) -> Result<(), TeamsError> {
    let value = match activity.value {
        Some(v) => v,
        None => return Ok(()),
    };

    #[derive(Deserialize)]
    struct InvokeData {
        action: Option<String>,
        workspace_id: Option<String>,
        #[allow(dead_code)]
        approval_id: Option<String>,
    }

    let data: InvokeData = serde_json::from_value(value)?;
    let action = data.action.as_deref().unwrap_or("");

    match action {
        "approve" | "reject" => {
            if let Some(ws_id_str) = &data.workspace_id {
                if let Ok(ws_id) = Uuid::parse_str(ws_id_str) {
                    let is_approve = action == "approve";
                    let reason = if is_approve {
                        None
                    } else {
                        Some("Rejected via Teams button")
                    };
                    // Find and resolve the pending approval
                    let approvals = deployment.approvals();
                    // The approval ID is stored in the card data
                    if let Some(approval_id) = &data.approval_id {
                        let outcome = if is_approve {
                            ApprovalOutcome::Approved
                        } else {
                            ApprovalOutcome::Denied {
                                reason: reason.map(|s| s.to_string()),
                            }
                        };
                        let _ = approvals
                            .respond(
                                approval_id,
                                ApprovalResponse {
                                    execution_process_id: ws_id,
                                    status: outcome,
                                },
                            )
                            .await;
                    }
                }
            }
        }
        _ => {
            tracing::debug!("Unknown invoke action: {}", action);
        }
    }

    Ok(())
}

// ── Workspace Creation from Teams ───────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn create_workspace_from_teams(
    deployment: &DeploymentImpl,
    teams: &TeamsService,
    teams_config: &services::services::config::TeamsConfig,
    pool: &sqlx::SqlitePool,
    service_url: &str,
    conversation_id: &str,
    reply_to_id: Option<&str>,
    activity_id: Option<&str>,
    prompt: &str,
    is_plan: bool,
) -> Result<(), TeamsError> {
    let mapping = TeamsChannelMapping::find_by_channel_id(pool, conversation_id).await?;

    let mapping = match mapping {
        Some(m) => m,
        None => {
            teams
                .send_text_reply(
                    service_url,
                    conversation_id,
                    reply_to_id,
                    "No channel mapping configured. Set up channel repos in Settings \u{2192} Teams.",
                    teams_config,
                )
                .await?;
            return Ok(());
        }
    };

    let channel_repos = TeamsChannelRepo::find_for_mapping(pool, mapping.id).await?;

    if channel_repos.is_empty() {
        teams
            .send_text_reply(
                service_url,
                conversation_id,
                reply_to_id,
                "No repos configured for this channel. Add repos in Settings \u{2192} Teams.",
                teams_config,
            )
            .await?;
        return Ok(());
    }

    // Create workspace
    let workspace_id = Uuid::new_v4();
    let branch_label = truncate_for_branch(prompt);
    let git_branch_name = deployment
        .container()
        .git_branch_from_workspace(&workspace_id, &branch_label)
        .await;

    let ws_name = truncate_for_name(prompt);
    let workspace = Workspace::create(
        pool,
        &CreateWorkspace {
            branch: git_branch_name,
            name: Some(ws_name.clone()),
        },
        workspace_id,
    )
    .await
    .map_err(|e| TeamsError::HttpError(format!("workspace creation failed: {}", e)))?;

    // Update workspace name
    Workspace::update(pool, workspace.id, None, None, Some(&ws_name))
        .await
        .map_err(|e| TeamsError::Database(e.into()))?;

    // Create workspace repos
    let workspace_repos: Vec<CreateWorkspaceRepo> = channel_repos
        .iter()
        .map(|r| CreateWorkspaceRepo {
            repo_id: r.repo_id,
            target_branch: r.target_branch.clone(),
        })
        .collect();
    WorkspaceRepo::create_many(pool, workspace.id, &workspace_repos).await?;

    // Store Teams conversation link
    TeamsConversation::create(
        pool,
        workspace.id,
        mapping.id,
        conversation_id,
        reply_to_id,
        activity_id,
    )
    .await?;

    // Start the workspace
    let config = deployment.config().read().await;
    let mut exec_config = ExecutorConfig::new(config.executor_profile.executor.clone());
    if is_plan {
        exec_config.variant = Some("plan".to_string());
    }
    drop(config);

    match deployment
        .container()
        .start_workspace(&workspace, exec_config, prompt.to_string())
        .await
    {
        Ok(_) => {
            // Get repo names for the card
            let repo_names: Vec<String> = {
                let mut names = Vec::new();
                for cr in &channel_repos {
                    if let Some(repo) = Repo::find_by_id(pool, cr.repo_id).await? {
                        names.push(repo.name);
                    }
                }
                names
            };
            let card =
                cards::build_workspace_created_card(&ws_name, &workspace.branch, &repo_names);
            teams
                .send_card_reply(
                    service_url,
                    conversation_id,
                    reply_to_id,
                    card,
                    teams_config,
                )
                .await?;
        }
        Err(e) => {
            tracing::error!("Failed to start workspace: {}", e);
            teams
                .send_text_reply(
                    service_url,
                    conversation_id,
                    reply_to_id,
                    "Failed to start workspace. Check the Vibe Kanban dashboard.",
                    teams_config,
                )
                .await?;
        }
    }

    Ok(())
}

// ── Management API Handlers ─────────────────────────────────────────────────

async fn get_teams_config(State(deployment): State<DeploymentImpl>) -> impl IntoResponse {
    let config = deployment.config().read().await;
    Json(TeamsConfigResponse {
        enabled: config.teams.enabled,
        bot_app_id: config.teams.bot_app_id.clone(),
        bot_app_password: None, // Never return password
    })
}

async fn update_teams_config(
    State(deployment): State<DeploymentImpl>,
    Json(input): Json<UpdateTeamsConfigRequest>,
) -> impl IntoResponse {
    let mut config = deployment.config().write().await;

    if let Some(enabled) = input.enabled {
        config.teams.enabled = enabled;
    }
    if let Some(app_id) = input.bot_app_id {
        config.teams.bot_app_id = Some(app_id);
    }
    if let Some(app_pass) = input.bot_app_password {
        config.teams.bot_app_password = Some(app_pass);
    }

    if let Err(e) =
        services::services::config::save_config_to_file(&config, &utils::assets::config_path())
            .await
    {
        tracing::error!("Failed to save config: {}", e);
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    Json(TeamsConfigResponse {
        enabled: config.teams.enabled,
        bot_app_id: config.teams.bot_app_id.clone(),
        bot_app_password: None,
    })
    .into_response()
}

async fn list_channel_mappings(State(deployment): State<DeploymentImpl>) -> impl IntoResponse {
    let pool = &deployment.db().pool;
    match TeamsChannelMapping::find_all(pool).await {
        Ok(mappings) => Json(mappings).into_response(),
        Err(e) => {
            tracing::error!("Failed to list channel mappings: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn create_channel_mapping(
    State(deployment): State<DeploymentImpl>,
    Json(input): Json<CreateTeamsChannelMapping>,
) -> impl IntoResponse {
    let pool = &deployment.db().pool;
    match TeamsChannelMapping::create(pool, &input).await {
        Ok(mapping) => (StatusCode::CREATED, Json(mapping)).into_response(),
        Err(e) => {
            tracing::error!("Failed to create channel mapping: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn get_channel_mapping(
    State(deployment): State<DeploymentImpl>,
    AxumPath(id): AxumPath<Uuid>,
) -> impl IntoResponse {
    let pool = &deployment.db().pool;
    match TeamsChannelMapping::find_by_id(pool, id).await {
        Ok(Some(mapping)) => Json(mapping).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("Failed to get channel mapping: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn update_channel_mapping(
    State(deployment): State<DeploymentImpl>,
    AxumPath(id): AxumPath<Uuid>,
    Json(input): Json<UpdateTeamsChannelMapping>,
) -> impl IntoResponse {
    let pool = &deployment.db().pool;
    match TeamsChannelMapping::update(pool, id, &input).await {
        Ok(Some(mapping)) => Json(mapping).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("Failed to update channel mapping: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn delete_channel_mapping(
    State(deployment): State<DeploymentImpl>,
    AxumPath(id): AxumPath<Uuid>,
) -> impl IntoResponse {
    let pool = &deployment.db().pool;
    match TeamsChannelMapping::delete(pool, id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("Failed to delete channel mapping: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn list_channel_repos(
    State(deployment): State<DeploymentImpl>,
    AxumPath(id): AxumPath<Uuid>,
) -> impl IntoResponse {
    let pool = &deployment.db().pool;
    match TeamsChannelRepo::find_for_mapping(pool, id).await {
        Ok(repos) => Json(repos).into_response(),
        Err(e) => {
            tracing::error!("Failed to list channel repos: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn add_channel_repo(
    State(deployment): State<DeploymentImpl>,
    AxumPath(id): AxumPath<Uuid>,
    Json(input): Json<CreateTeamsChannelRepo>,
) -> impl IntoResponse {
    let pool = &deployment.db().pool;
    match TeamsChannelRepo::create(pool, id, &input).await {
        Ok(repo) => (StatusCode::CREATED, Json(repo)).into_response(),
        Err(e) => {
            tracing::error!("Failed to add channel repo: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn remove_channel_repo(
    State(deployment): State<DeploymentImpl>,
    AxumPath((channel_id, repo_id)): AxumPath<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let pool = &deployment.db().pool;
    match TeamsChannelRepo::delete(pool, channel_id, repo_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("Failed to remove channel repo: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

// ── Helper Functions ────────────────────────────────────────────────────────

/// Find the active (non-archived) workspace for a Teams channel.
async fn find_active_workspace(
    pool: &sqlx::SqlitePool,
    channel_id: &str,
) -> Result<Option<(Workspace, TeamsConversation)>, TeamsError> {
    let conv = TeamsConversation::find_active_for_channel(pool, channel_id).await?;
    match conv {
        Some(c) => {
            let ws = Workspace::find_by_id(pool, c.workspace_id).await?;
            match ws {
                Some(w) => Ok(Some((w, c))),
                None => Ok(None),
            }
        }
        None => Ok(None),
    }
}

async fn handle_done_command(
    pool: &sqlx::SqlitePool,
    deployment: &DeploymentImpl,
    teams: &TeamsService,
    service_url: &str,
    conversation_id: &str,
    reply_to_id: Option<&str>,
    teams_config: &services::services::config::TeamsConfig,
) -> Result<(), TeamsError> {
    let active = find_active_workspace(pool, conversation_id).await?;

    match active {
        Some((ws, _conv)) => {
            deployment.container().try_stop(&ws, false).await;
            let ws_name = ws.name.clone().unwrap_or_else(|| ws.branch.clone());

            Workspace::update(pool, ws.id, Some(true), None, None)
                .await
                .map_err(|e| TeamsError::Database(e.into()))?;

            let card = cards::build_action_confirmation_card("Done", &ws_name, None);
            teams
                .send_card_reply(
                    service_url,
                    conversation_id,
                    reply_to_id,
                    card,
                    teams_config,
                )
                .await?;
        }
        None => {
            teams
                .send_text_reply(
                    service_url,
                    conversation_id,
                    reply_to_id,
                    "No active workspace to close.",
                    teams_config,
                )
                .await?;
        }
    }

    Ok(())
}

/// Try to respond to a pending approval for the active workspace in a channel.
async fn try_respond_approval(
    deployment: &DeploymentImpl,
    conversation_id: &str,
    approve: bool,
    _reason: Option<&str>,
) -> String {
    let pool = &deployment.db().pool;
    let _active = match find_active_workspace(pool, conversation_id).await {
        Ok(Some(a)) => a,
        Ok(None) => return "No active workspace.".to_string(),
        Err(e) => return format!("Error: {}", e),
    };

    // Best-effort: the Adaptive Card button invoke flow is more precise.
    // We don't have a direct way to find pending approvals by workspace ID.
    if approve {
        "Plan approved. Use the Approve button on the Adaptive Card for reliable approval."
            .to_string()
    } else {
        "Plan rejected. Use the Reject button on the Adaptive Card for reliable rejection."
            .to_string()
    }
}

/// Truncate a prompt to use as a branch name label.
fn truncate_for_branch(text: &str) -> String {
    let clean: String = text
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-')
        .take(40)
        .collect();
    clean.trim().replace(' ', "-").to_lowercase()
}

/// Truncate a prompt to use as a workspace name.
fn truncate_for_name(text: &str) -> String {
    if text.len() <= 80 {
        text.to_string()
    } else {
        format!("{}...", &text[..77])
    }
}
