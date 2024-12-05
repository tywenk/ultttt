use std::sync::Arc;

use anyhow::anyhow;
use anyhow::Result;
use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use axum::extract::Path;
use axum::extract::WebSocketUpgrade;
use axum::http::StatusCode;
use axum::Json;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::crud::crud_create_match;
use crate::crud::crud_get_match;
use crate::crud::crud_get_matches;
use crate::crud::crud_update_match;
use crate::error::AppError;
use crate::schema::Coords;
use crate::schema::GetMatchSchema;
use crate::{schema::Pagination, AppState};

pub async fn get_matches_handler(
    opts: Option<Query<Pagination>>,
    State(data): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let Query(opts) = opts.unwrap_or_default();

    let limit = opts.limit.unwrap_or(10);
    let offset = opts.offset.unwrap_or(0);

    let matches = crud_get_matches(&data.db, limit, offset).await?;
    let matches: Result<Vec<GetMatchSchema>> = matches.iter().map(|m| m.try_into()).collect();

    match matches {
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
    Ok(Json(crud_get_match(&data.db, match_id).await?))
}

pub async fn create_match_handler(
    State(data): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let m = crud_create_match(&data.db).await?;
    Ok(Json(GetMatchSchema::try_from(&m)?))
}

pub async fn commit_match_from_snapshot_handler(
    Path(match_id): Path<Uuid>,
    State(data): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let coords = data
        .snapshot
        .find_max_indices()
        .ok_or_else(|| anyhow!("No moves found"))?;

    let m = crud_get_match(&data.db, match_id).await?;
    let m = GetMatchSchema::try_from(&m)?;

    let board = m.board.get_updated(coords)?;

    let m = crud_update_match(&data.db, match_id, board.status, board).await?;

    data.snapshot.reset();

    Ok(Json(GetMatchSchema::try_from(&m)?))
}

pub async fn get_snapshot_handler(
    State(data): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(data.snapshot.load()))
}

pub async fn update_snapshot_handler(
    Query(params): Query<Coords>,
    State(data): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    data.snapshot.increment(params.section, params.cell);
    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
struct IncrementRequest {
    section: usize,
    cell: usize,
}

#[derive(Serialize)]
struct SnapshotResponse {
    snap: [[usize; 9]; 9],
}

pub async fn handle_websocket(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket_connection(socket, state))
}

async fn handle_socket_connection(mut socket: WebSocket, state: Arc<AppState>) {
    // Send initial snapshot state
    let initial_snapshot = SnapshotResponse {
        snap: state.snapshot.load(),
    };

    if let Ok(initial_message) = serde_json::to_string(&initial_snapshot) {
        let _ = socket.send(Message::Text(initial_message)).await;
    }

    while let Some(Ok(message)) = socket.recv().await {
        match message {
            Message::Text(text) => {
                // Try to parse the increment request
                if let Ok(request) = serde_json::from_str::<IncrementRequest>(&text) {
                    // Validate indices
                    if request.section < 9 && request.cell < 9 {
                        // Increment the value
                        state.snapshot.increment(request.section, request.cell);

                        // Send back the updated snapshot
                        let response = SnapshotResponse {
                            snap: state.snapshot.load(),
                        };

                        if let Ok(response_text) = serde_json::to_string(&response) {
                            let _ = socket.send(Message::Text(response_text)).await;
                        }
                    }
                }
            }
            Message::Close(_) => break,
            _ => {} // Ignore other message types
        }
    }
}
