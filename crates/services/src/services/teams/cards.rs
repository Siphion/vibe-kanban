use serde_json::{Value, json};
use uuid::Uuid;

/// Adaptive Card schema version used for all cards.
const SCHEMA: &str = "http://adaptivecards.io/schemas/adaptive-card.json";
const VERSION: &str = "1.2";

fn wrap_card(body: Vec<Value>, actions: Option<Vec<Value>>) -> Value {
    let mut card = json!({
        "$schema": SCHEMA,
        "type": "AdaptiveCard",
        "version": VERSION,
        "body": body,
    });
    if let Some(actions) = actions {
        card["actions"] = json!(actions);
    }
    card
}

fn header(text: &str) -> Value {
    json!({
        "type": "TextBlock",
        "text": text,
        "weight": "Bolder",
        "size": "Medium",
        "wrap": true
    })
}

fn fact(title: &str, value: &str) -> Value {
    json!({ "title": title, "value": value })
}

fn fact_set(facts: Vec<Value>) -> Value {
    json!({ "type": "FactSet", "facts": facts })
}

fn text_block(text: &str) -> Value {
    json!({ "type": "TextBlock", "text": text, "wrap": true })
}

fn text_block_subtle(text: &str) -> Value {
    json!({ "type": "TextBlock", "text": text, "wrap": true, "isSubtle": true, "size": "Small" })
}

// ── Public Card Builders ────────────────────────────────────────────────────

pub fn build_help_card() -> Value {
    let body = vec![
        header("Vibe Kanban Bot — Commands"),
        text_block("Mention the bot followed by a command:"),
        fact_set(vec![
            fact("`<text>`", "Create new task or send follow-up"),
            fact("`plan <text>`", "Create task in plan-first mode"),
            fact("`approve`", "Approve pending plan"),
            fact("`reject <reason>`", "Reject plan with feedback"),
            fact("`status`", "Show active workspace status"),
            fact("`diff`", "Show diff summary"),
            fact("`branch`", "Show branch info"),
            fact("`push`", "Push branch to remote"),
            fact("`done`", "Complete workspace"),
            fact("`donemerge`", "Complete and merge"),
            fact("`donepr`", "Complete and create PR"),
            fact("`cancel`", "Cancel active workspace"),
            fact("`retry`", "Retry failed workspace"),
            fact("`list`", "List recent workspaces"),
            fact("`help`", "Show this help"),
        ]),
    ];
    wrap_card(body, None)
}

pub fn build_status_card(
    workspace_name: &str,
    status: &str,
    branch: Option<&str>,
    repos: &[String],
    duration: Option<&str>,
    executor: Option<&str>,
) -> Value {
    let mut facts = vec![fact("Status", status)];
    if let Some(branch) = branch {
        facts.push(fact("Branch", branch));
    }
    if !repos.is_empty() {
        facts.push(fact("Repos", &repos.join(", ")));
    }
    if let Some(d) = duration {
        facts.push(fact("Duration", d));
    }
    if let Some(e) = executor {
        facts.push(fact("Executor", e));
    }

    let body = vec![
        header(&format!("Workspace: {}", workspace_name)),
        fact_set(facts),
    ];
    wrap_card(body, None)
}

pub fn build_diff_card(
    files_changed: usize,
    insertions: usize,
    deletions: usize,
    vk_diff_url: Option<&str>,
    short_diff: Option<&str>,
) -> Value {
    let mut body = vec![
        header("Diff Summary"),
        fact_set(vec![
            fact("Files changed", &files_changed.to_string()),
            fact("Insertions", &format!("+{}", insertions)),
            fact("Deletions", &format!("-{}", deletions)),
        ]),
    ];

    if let Some(diff) = short_diff {
        body.push(json!({
            "type": "TextBlock",
            "text": format!("```\n{}\n```", diff),
            "wrap": true,
            "fontType": "Monospace",
            "size": "Small"
        }));
    }

    let actions = vk_diff_url.map(|url| {
        vec![json!({
            "type": "Action.OpenUrl",
            "title": "View Full Diff",
            "url": url
        })]
    });

    wrap_card(body, actions)
}

pub fn build_branch_card(branch: &str, repos: &[(String, String)]) -> Value {
    let mut facts = vec![fact("Branch", branch)];
    for (repo_name, target) in repos {
        facts.push(fact(repo_name, &format!("→ {}", target)));
    }

    let body = vec![header("Branch Info"), fact_set(facts)];
    wrap_card(body, None)
}

pub struct WorkspaceListItem {
    pub name: String,
    pub status: String,
    pub created_at: String,
}

pub fn build_list_card(workspaces: &[WorkspaceListItem]) -> Value {
    if workspaces.is_empty() {
        let body = vec![
            header("Recent Workspaces"),
            text_block("No workspaces found for this channel."),
        ];
        return wrap_card(body, None);
    }

    let mut body = vec![header("Recent Workspaces")];
    for ws in workspaces {
        body.push(json!({
            "type": "ColumnSet",
            "columns": [
                {
                    "type": "Column",
                    "width": "stretch",
                    "items": [{ "type": "TextBlock", "text": &ws.name, "weight": "Bolder", "wrap": true }]
                },
                {
                    "type": "Column",
                    "width": "auto",
                    "items": [{ "type": "TextBlock", "text": &ws.status, "isSubtle": true }]
                },
                {
                    "type": "Column",
                    "width": "auto",
                    "items": [{ "type": "TextBlock", "text": &ws.created_at, "isSubtle": true, "size": "Small" }]
                }
            ]
        }));
    }

    wrap_card(body, None)
}

pub fn build_plan_approval_card(plan_text: &str, workspace_id: Uuid, approval_id: &str) -> Value {
    let body = vec![
        header("Plan Approval Required"),
        json!({
            "type": "TextBlock",
            "text": plan_text,
            "wrap": true,
            "fontType": "Monospace",
            "size": "Small",
            "maxLines": 30
        }),
    ];

    let actions = Some(vec![
        json!({
            "type": "Action.Submit",
            "title": "Approve",
            "style": "positive",
            "data": {
                "action": "approve",
                "workspace_id": workspace_id.to_string(),
                "approval_id": approval_id
            }
        }),
        json!({
            "type": "Action.Submit",
            "title": "Reject",
            "style": "destructive",
            "data": {
                "action": "reject",
                "workspace_id": workspace_id.to_string(),
                "approval_id": approval_id
            }
        }),
    ]);

    wrap_card(body, actions)
}

pub fn build_workspace_created_card(workspace_name: &str, branch: &str, repos: &[String]) -> Value {
    let mut facts = vec![fact("Branch", branch)];
    if !repos.is_empty() {
        facts.push(fact("Repos", &repos.join(", ")));
    }

    let body = vec![
        header(&format!("Task Created: {}", workspace_name)),
        fact_set(facts),
        text_block_subtle("The agent is working on this task now."),
    ];
    wrap_card(body, None)
}

pub fn build_workspace_completed_card(
    workspace_name: &str,
    status: &str,
    summary: Option<&str>,
    diff_stats: Option<&str>,
    vk_url: Option<&str>,
) -> Value {
    let mut body = vec![
        header(&format!("Task Completed: {}", workspace_name)),
        fact_set(vec![fact("Status", status)]),
    ];

    if let Some(s) = summary {
        body.push(text_block(s));
    }
    if let Some(d) = diff_stats {
        body.push(text_block_subtle(d));
    }

    let actions = vk_url.map(|url| {
        vec![json!({
            "type": "Action.OpenUrl",
            "title": "Open in Vibe Kanban",
            "url": url
        })]
    });

    wrap_card(body, actions)
}

pub fn build_action_confirmation_card(
    action: &str,
    workspace_name: &str,
    details: Option<&str>,
) -> Value {
    let mut body = vec![header(&format!("{}: {}", action, workspace_name))];
    if let Some(d) = details {
        body.push(text_block(d));
    }
    wrap_card(body, None)
}

pub fn build_followup_ack_card(message_preview: &str, workspace_name: &str) -> Value {
    let preview = if message_preview.len() > 100 {
        format!("{}...", &message_preview[..100])
    } else {
        message_preview.to_string()
    };
    let body = vec![
        header(&format!("Follow-up sent to: {}", workspace_name)),
        text_block_subtle(&preview),
    ];
    wrap_card(body, None)
}

pub fn build_confirm_merge_card(workspace_name: &str, target_branch: &str) -> Value {
    let body = vec![
        header("Confirm Merge"),
        text_block(&format!(
            "Merge workspace **{}** into `{}`?",
            workspace_name, target_branch
        )),
    ];

    let actions = Some(vec![
        json!({
            "type": "Action.Submit",
            "title": "Confirm Merge",
            "style": "positive",
            "data": { "action": "confirm_merge" }
        }),
        json!({
            "type": "Action.Submit",
            "title": "Cancel",
            "style": "destructive",
            "data": { "action": "cancel_merge" }
        }),
    ]);

    wrap_card(body, actions)
}
