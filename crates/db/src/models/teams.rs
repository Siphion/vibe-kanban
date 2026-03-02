use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

// ── Channel Mapping ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TeamsChannelMapping {
    pub id: Uuid,
    pub teams_channel_id: String,
    pub teams_channel_name: Option<String>,
    pub teams_service_url: String,
    pub conversation_ref: String,
    pub project_id: Option<Uuid>,
    pub label: Option<String>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateTeamsChannelMapping {
    pub teams_channel_id: String,
    pub teams_channel_name: Option<String>,
    pub teams_service_url: String,
    pub project_id: Option<Uuid>,
    pub label: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateTeamsChannelMapping {
    pub teams_channel_name: Option<String>,
    pub teams_service_url: Option<String>,
    pub project_id: Option<Uuid>,
    pub label: Option<String>,
}

impl TeamsChannelMapping {
    pub async fn find_all(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TeamsChannelMapping,
            r#"SELECT id as "id!: Uuid",
                      teams_channel_id,
                      teams_channel_name,
                      teams_service_url,
                      conversation_ref,
                      project_id as "project_id: Uuid",
                      label,
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM teams_channel_mappings
               ORDER BY created_at DESC"#
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TeamsChannelMapping,
            r#"SELECT id as "id!: Uuid",
                      teams_channel_id,
                      teams_channel_name,
                      teams_service_url,
                      conversation_ref,
                      project_id as "project_id: Uuid",
                      label,
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM teams_channel_mappings
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_channel_id(
        pool: &SqlitePool,
        channel_id: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TeamsChannelMapping,
            r#"SELECT id as "id!: Uuid",
                      teams_channel_id,
                      teams_channel_name,
                      teams_service_url,
                      conversation_ref,
                      project_id as "project_id: Uuid",
                      label,
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM teams_channel_mappings
               WHERE teams_channel_id = $1"#,
            channel_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        input: &CreateTeamsChannelMapping,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            TeamsChannelMapping,
            r#"INSERT INTO teams_channel_mappings (id, teams_channel_id, teams_channel_name, teams_service_url, project_id, label)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING id as "id!: Uuid",
                         teams_channel_id,
                         teams_channel_name,
                         teams_service_url,
                         conversation_ref,
                         project_id as "project_id: Uuid",
                         label,
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            input.teams_channel_id,
            input.teams_channel_name,
            input.teams_service_url,
            input.project_id,
            input.label
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        input: &UpdateTeamsChannelMapping,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TeamsChannelMapping,
            r#"UPDATE teams_channel_mappings SET
                teams_channel_name = COALESCE($2, teams_channel_name),
                teams_service_url = COALESCE($3, teams_service_url),
                project_id = COALESCE($4, project_id),
                label = COALESCE($5, label),
                updated_at = datetime('now','subsec')
               WHERE id = $1
               RETURNING id as "id!: Uuid",
                         teams_channel_id,
                         teams_channel_name,
                         teams_service_url,
                         conversation_ref,
                         project_id as "project_id: Uuid",
                         label,
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            input.teams_channel_name,
            input.teams_service_url,
            input.project_id,
            input.label
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn update_conversation_ref(
        pool: &SqlitePool,
        id: Uuid,
        conversation_ref: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE teams_channel_mappings SET conversation_ref = $2, updated_at = datetime('now','subsec') WHERE id = $1",
            id,
            conversation_ref
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM teams_channel_mappings WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

// ── Channel Repos ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TeamsChannelRepo {
    pub id: Uuid,
    pub channel_mapping_id: Uuid,
    pub repo_id: Uuid,
    pub target_branch: String,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateTeamsChannelRepo {
    pub repo_id: Uuid,
    pub target_branch: String,
}

impl TeamsChannelRepo {
    pub async fn find_for_mapping(
        pool: &SqlitePool,
        channel_mapping_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TeamsChannelRepo,
            r#"SELECT id as "id!: Uuid",
                      channel_mapping_id as "channel_mapping_id!: Uuid",
                      repo_id as "repo_id!: Uuid",
                      target_branch
               FROM teams_channel_repos
               WHERE channel_mapping_id = $1"#,
            channel_mapping_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        channel_mapping_id: Uuid,
        input: &CreateTeamsChannelRepo,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            TeamsChannelRepo,
            r#"INSERT INTO teams_channel_repos (id, channel_mapping_id, repo_id, target_branch)
               VALUES ($1, $2, $3, $4)
               RETURNING id as "id!: Uuid",
                         channel_mapping_id as "channel_mapping_id!: Uuid",
                         repo_id as "repo_id!: Uuid",
                         target_branch"#,
            id,
            channel_mapping_id,
            input.repo_id,
            input.target_branch
        )
        .fetch_one(pool)
        .await
    }

    pub async fn delete(
        pool: &SqlitePool,
        channel_mapping_id: Uuid,
        repo_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM teams_channel_repos WHERE channel_mapping_id = $1 AND repo_id = $2",
            channel_mapping_id,
            repo_id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

// ── Conversations ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TeamsConversation {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub channel_mapping_id: Uuid,
    pub conversation_ref: String,
    pub reply_to_activity_id: Option<String>,
    pub teams_activity_id: Option<String>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

impl TeamsConversation {
    pub async fn find_by_workspace_id(
        pool: &SqlitePool,
        workspace_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TeamsConversation,
            r#"SELECT id as "id!: Uuid",
                      workspace_id as "workspace_id!: Uuid",
                      channel_mapping_id as "channel_mapping_id!: Uuid",
                      conversation_ref,
                      reply_to_activity_id,
                      teams_activity_id,
                      created_at as "created_at!: DateTime<Utc>"
               FROM teams_conversations
               WHERE workspace_id = $1"#,
            workspace_id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find the active workspace conversation for a channel.
    /// A workspace is "active" if it is not archived.
    pub async fn find_active_for_channel(
        pool: &SqlitePool,
        channel_id: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TeamsConversation,
            r#"SELECT tc.id as "id!: Uuid",
                      tc.workspace_id as "workspace_id!: Uuid",
                      tc.channel_mapping_id as "channel_mapping_id!: Uuid",
                      tc.conversation_ref,
                      tc.reply_to_activity_id,
                      tc.teams_activity_id,
                      tc.created_at as "created_at!: DateTime<Utc>"
               FROM teams_conversations tc
               JOIN teams_channel_mappings tcm ON tc.channel_mapping_id = tcm.id
               JOIN workspaces w ON tc.workspace_id = w.id
               WHERE tcm.teams_channel_id = $1
                 AND (w.archived IS NULL OR w.archived = 0)
               ORDER BY tc.created_at DESC
               LIMIT 1"#,
            channel_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        workspace_id: Uuid,
        channel_mapping_id: Uuid,
        conversation_ref: &str,
        reply_to_activity_id: Option<&str>,
        teams_activity_id: Option<&str>,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            TeamsConversation,
            r#"INSERT INTO teams_conversations
               (id, workspace_id, channel_mapping_id, conversation_ref, reply_to_activity_id, teams_activity_id)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING id as "id!: Uuid",
                         workspace_id as "workspace_id!: Uuid",
                         channel_mapping_id as "channel_mapping_id!: Uuid",
                         conversation_ref,
                         reply_to_activity_id,
                         teams_activity_id,
                         created_at as "created_at!: DateTime<Utc>""#,
            id,
            workspace_id,
            channel_mapping_id,
            conversation_ref,
            reply_to_activity_id,
            teams_activity_id
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update_reply_activity_id(
        pool: &SqlitePool,
        id: Uuid,
        activity_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE teams_conversations SET reply_to_activity_id = $2 WHERE id = $1",
            id,
            activity_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}
