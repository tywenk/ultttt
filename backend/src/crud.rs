use anyhow::{anyhow, Ok};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::{
    model::MatchModel,
    schema::{Board, Status},
};

pub async fn crud_get_matches(
    db: &Pool<Postgres>,
    limit: usize,
    offset: usize,
) -> Result<Vec<MatchModel>, anyhow::Error> {
    let matches: Vec<MatchModel> =
        sqlx::query_as(r#"SELECT * FROM matches ORDER by id LIMIT $1 OFFSET $2"#)
            .bind(limit as i32)
            .bind(offset as i32)
            .fetch_all(db)
            .await
            .map_err(|e| anyhow!("Unable to query model from db: {}", e))?;

    Ok(matches)
}

pub async fn crud_get_match(db: &Pool<Postgres>, id: Uuid) -> Result<MatchModel, anyhow::Error> {
    let match_model: MatchModel = sqlx::query_as(
        r#"
        SELECT id, state, board, created_at, updated_at
        FROM matches
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_one(db)
    .await
    .map_err(|e| anyhow!("Unable to query model from db: {}", e))?;

    Ok(match_model)
}

pub async fn crud_create_match(db: &Pool<Postgres>) -> Result<MatchModel, anyhow::Error> {
    let state = Status::Pending;
    let board = Board::new();
    let m: MatchModel =
        sqlx::query_as(r#"INSERT INTO matches (id, state, board) VALUES ($1, $2, $3) RETURNING *"#)
            .bind(uuid::Uuid::new_v4())
            .bind(state)
            .bind(board)
            .fetch_one(db)
            .await
            .map_err(|e| anyhow!("Unable to query model from db: {}", e))?;

    Ok(m)
}

pub async fn crud_update_match(
    db: &Pool<Postgres>,
    id: Uuid,
    state: Status,
    board: Board,
) -> Result<MatchModel, anyhow::Error> {
    let m: MatchModel =
        sqlx::query_as(r#"UPDATE matches SET (state, board) = ($2, $3) WHERE id = $1 RETURNING *"#)
            .bind(id)
            .bind(state)
            .bind(board)
            .fetch_one(db)
            .await
            .map_err(|e| anyhow!("Unable to query model from db: {}", e))?;

    Ok(m)
}
