use std::sync::Arc;

use anyhow::anyhow;
use anyhow::Result;
use axum::extract::Path;
use axum::Json;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use uuid::Uuid;

use crate::error::AppError;
use crate::schema::GetMatchSchema;
use crate::schema::Status;
use crate::{model::MatchModel, schema::Pagination, AppState};

pub async fn get_matches_handler(
    opts: Option<Query<Pagination>>,
    State(data): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let Query(opts) = opts.unwrap_or_default();

    let limit = opts.limit.unwrap_or(10);
    let offset = opts.offset.unwrap_or(0);

    tracing::debug!("got params: {:?}", opts);
    tracing::debug!("parsed params: {} {}", limit, offset);

    let matches: Vec<MatchModel> =
        sqlx::query_as(r#"SELECT * FROM matches ORDER by id LIMIT $1 OFFSET $2"#)
            .bind(limit as i32)
            .bind(offset as i32)
            .fetch_all(&data.db)
            .await
            .map_err(|e| anyhow!("Unable to query model from db: {}", e))?;

    let matches_responses: Result<Vec<GetMatchSchema>> =
        matches.iter().map(|m| m.try_into()).collect();

    match matches_responses {
        Ok(res) => {
            let json_response = serde_json::json!({
                "count": res.len(),
                "notes": res
            });

            Ok(Json(json_response))
        }
        Err(e) => Err(AppError::from(anyhow!("Unable to parse matches: {}", e))),
    }
}

pub async fn get_match_by_id_handler(
    Path(match_id): Path<Uuid>,
    State(data): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!("got match_id: {}", match_id);
    let match_model: MatchModel = sqlx::query_as(
        r#"
        SELECT id, state, snapshot, created_at, updated_at
        FROM matches
        WHERE id = $1
        "#,
    )
    .bind(match_id)
    .fetch_one(&data.db)
    .await?;

    Ok(Json(match_model))
}

pub async fn create_match_handler(
    State(data): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let state = Status::Pending;
    let snapshot = [
        [0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0],
    ];
    let m: MatchModel =
        sqlx::query_as(r#"INSERT INTO matches (id, state, snapshot) VALUES ($1, $2, $3)"#)
            .bind(uuid::Uuid::new_v4())
            .bind(state)
            .bind(snapshot)
            .fetch_one(&data.db)
            .await
            .map_err(|e| anyhow!("Unable to query model from db: {}", e))?;

    Ok(Json(m))
}
