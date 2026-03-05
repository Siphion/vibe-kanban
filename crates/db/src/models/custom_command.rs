use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct CustomCommand {
    pub id: Uuid,
    pub name: String,
    pub script: String,
    pub description: Option<String>,
    pub mode: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateCustomCommand {
    pub name: String,
    pub script: String,
    pub description: Option<String>,
    pub mode: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateCustomCommand {
    pub name: Option<String>,
    pub script: Option<String>,
    pub description: Option<String>,
    pub mode: Option<String>,
}

impl CustomCommand {
    pub async fn find_all(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            CustomCommand,
            r#"SELECT id as "id!: Uuid", name, script, description, mode,
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM custom_commands
               ORDER BY name ASC"#
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            CustomCommand,
            r#"SELECT id as "id!: Uuid", name, script, description, mode,
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM custom_commands
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_name(pool: &SqlitePool, name: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            CustomCommand,
            r#"SELECT id as "id!: Uuid", name, script, description, mode,
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM custom_commands
               WHERE name = $1 COLLATE NOCASE"#,
            name
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_all_names(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
        let rows = sqlx::query_scalar!("SELECT name FROM custom_commands ORDER BY name ASC")
            .fetch_all(pool)
            .await?;
        Ok(rows)
    }

    pub async fn create(
        pool: &SqlitePool,
        data: &CreateCustomCommand,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let mode = data.mode.as_deref().unwrap_or("background");
        sqlx::query_as!(
            CustomCommand,
            r#"INSERT INTO custom_commands (id, name, script, description, mode)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING id as "id!: Uuid", name, script, description, mode,
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            data.name,
            data.script,
            data.description,
            mode
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: &UpdateCustomCommand,
    ) -> Result<Self, sqlx::Error> {
        let existing = Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let name = data.name.as_ref().unwrap_or(&existing.name);
        let script = data.script.as_ref().unwrap_or(&existing.script);
        let description = data.description.as_ref().or(existing.description.as_ref());
        let mode = data.mode.as_ref().unwrap_or(&existing.mode);

        sqlx::query_as!(
            CustomCommand,
            r#"UPDATE custom_commands
               SET name = $2, script = $3, description = $4, mode = $5, updated_at = datetime('now', 'subsec')
               WHERE id = $1
               RETURNING id as "id!: Uuid", name, script, description, mode,
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            name,
            script,
            description,
            mode
        )
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result: sqlx::sqlite::SqliteQueryResult =
            sqlx::query("DELETE FROM custom_commands WHERE id = $1")
                .bind(id)
                .execute(pool)
                .await?;
        Ok(result.rows_affected())
    }
}
