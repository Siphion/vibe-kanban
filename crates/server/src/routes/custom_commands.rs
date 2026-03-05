use axum::{
    Json, Router,
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::get,
};
use db::models::custom_command::{CreateCustomCommand, CustomCommand, UpdateCustomCommand};
use deployment::Deployment;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

async fn list_commands(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<CustomCommand>>>, ApiError> {
    let commands = CustomCommand::find_all(&deployment.db().pool).await?;
    Ok(ResponseJson(ApiResponse::success(commands)))
}

async fn create_command(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateCustomCommand>,
) -> Result<ResponseJson<ApiResponse<CustomCommand>>, ApiError> {
    let command = CustomCommand::create(&deployment.db().pool, &payload).await?;
    Ok(ResponseJson(ApiResponse::success(command)))
}

async fn update_command(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateCustomCommand>,
) -> Result<ResponseJson<ApiResponse<CustomCommand>>, ApiError> {
    let command = CustomCommand::update(&deployment.db().pool, id, &payload).await?;
    Ok(ResponseJson(ApiResponse::success(command)))
}

async fn delete_command(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let rows_affected = CustomCommand::delete(&deployment.db().pool, id).await?;
    if rows_affected == 0 {
        Err(ApiError::Database(sqlx::Error::RowNotFound))
    } else {
        Ok(ResponseJson(ApiResponse::success(())))
    }
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/commands", get(list_commands).post(create_command))
        .route(
            "/commands/{id}",
            axum::routing::put(update_command).delete(delete_command),
        )
}
