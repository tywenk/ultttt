use std::sync::atomic::Ordering;
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
use futures::SinkExt;
use futures::StreamExt;
use tokio::time::Duration;
use tokio::time::Instant;
use uuid::Uuid;

use crate::crud::crud_create_match;
use crate::crud::crud_get_match;
use crate::crud::crud_get_matches;
use crate::crud::crud_update_match;
use crate::error::AppError;
use crate::schema::Coords;
use crate::schema::GetMatchSchema;
use crate::schema::IncrementRequest;
use crate::schema::SnapshotResponse;
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

pub async fn handle_websocket(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket_connection(socket, state))
}

async fn handle_socket_connection(mut socket: WebSocket, state: Arc<AppState>) {
    // Subscribe to broadcast channel
    // This allows the connection to receive updates, which in this case are snapshots
    let mut rx = state.tx.subscribe();

    let current_connections = state.connection_count.fetch_add(1, Ordering::SeqCst);
    tracing::info!(
        "New connection. Total connections: {}",
        current_connections + 1
    );

    // Send initial snapshot state
    let initial_snapshot = SnapshotResponse {
        snap: state.snapshot.load(),
    };

    if let Ok(initial_message) = serde_json::to_string(&initial_snapshot) {
        let _ = socket.send(Message::Text(initial_message)).await;
    }

    // Split the socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Spawn a task to handle broadcast messages
    // Handles sending messages from this server to the client
    let mut send_task = {
        tokio::spawn(async move {
            while let Ok(snap) = rx.recv().await {
                if let Ok(msg) = serde_json::to_string(&snap) {
                    if sender.send(Message::Text(msg)).await.is_err() {
                        break;
                    }
                }
            }
        })
    };

    // Spawn a task to handle incoming messages
    // Handles receiving messages sent from the client to this server
    let mut receive_task = {
        let state = state.clone();
        tokio::spawn(async move {
            let mut last_broadcast = Instant::now();
            let mut needs_broadcast = false;

            while let Some(Ok(message)) = receiver.next().await {
                if let Message::Text(text) = message {
                    if let Ok(request) = serde_json::from_str::<IncrementRequest>(&text) {
                        if request.section < 9 && request.cell < 9 {
                            state.snapshot.increment(request.section, request.cell);
                            needs_broadcast = true;

                            // If enough time has passed since last broadcast then
                            // we send the most updated state. This is a primitive form
                            // of rate limiting.
                            if needs_broadcast
                                && last_broadcast.elapsed() > Duration::from_millis(100)
                            {
                                let new_snap = state.snapshot.load();
                                let _ = state.tx.send(SnapshotResponse { snap: new_snap });
                                needs_broadcast = false;
                                last_broadcast = Instant::now();
                            }
                        }
                    }
                }
            }

            // Don't forget final broadcast if needed
            if needs_broadcast {
                let new_snap = state.snapshot.load();
                let _ = state.tx.send(SnapshotResponse { snap: new_snap });
            }
        })
    };

    // Wait for either task to finish and then cleanup
    tokio::select! {
        _ = &mut send_task => receive_task.abort(),
        _ = &mut receive_task => send_task.abort(),
    };

    // Log connection close and decrement connection count
    let remaining = state.connection_count.fetch_sub(1, Ordering::SeqCst) - 1;
    tracing::info!("Connection closed. Remaining connections: {}", remaining);
}
