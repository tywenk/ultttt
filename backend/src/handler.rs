use std::sync::Arc;

use anyhow::anyhow;
use anyhow::bail;
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
use crate::crud::crud_get_latest_match;
use crate::crud::crud_get_match;
use crate::crud::crud_get_matches;
use crate::crud::crud_update_match;
use crate::error::AppError;
use crate::schema::Coords;
use crate::schema::IncrementRequest;
use crate::schema::MatchSchema;
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
    let matches: Result<Vec<MatchSchema>> = matches.iter().map(|m| m.try_into()).collect();

    match matches {
        Ok(res) => {
            let json_response = serde_json::json!({
                "count": res.len(),
                "data": res
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
    Ok(Json(MatchSchema::try_from(&m)?))
}

pub async fn get_latest_match_handler(
    State(data): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let m = crud_get_latest_match(&data.db).await?;
    Ok(Json(MatchSchema::try_from(&m)?))
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
    let m = MatchSchema::try_from(&m)?;

    let board = m.board.get_updated(coords)?;

    let m = crud_update_match(&data.db, match_id, board.status, board).await?;

    data.snapshot.reset();

    Ok(Json(MatchSchema::try_from(&m)?))
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

async fn handle_socket_connection(socket: WebSocket, state: Arc<AppState>) {
    // Add the connection to a team.
    let team_connection = state.teams.assign_team();

    // Subscribe to broadcast channel
    // This allows the connection to receive updates, which in this case are snapshots
    let mut snap_rx = state.snap_tx.subscribe();

    // Subscribe to timer broadcast channel
    // This allows the connection to receive match updates
    let mut timer_rx = state.timer_tx.subscribe();

    let (team_x_len, team_o_len) = state.teams.team_lens();
    tracing::info!(
        "New connection (Team {:?}). Total connections: {}",
        team_connection.team,
        team_x_len + team_o_len
    );

    // Split the socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Broadcast the most recent snapshot to this client
    // This lets them know what team they are on
    let initial_snap = state.snapshot.load();
    if let Ok(initial_response) = serde_json::to_string(&SnapshotResponse {
        snap: initial_snap,
        current_team: state.teams.current_team.load(),
        your_team: Some(team_connection.team),
    }) {
        if sender.send(Message::Text(initial_response)).await.is_err() {
            tracing::error!("Unable to send initial snapshot to client");
        }
    }

    // Spawn a task to handle broadcast snapshot and match messages
    // Handles sending messages from this server to the client
    let mut send_task = {
        tokio::spawn(async move {
            // Continually loop to handle snap_rx and timer_rx messages, whichever comes first
            loop {
                tokio::select! {
                    // Handle sending snapshot updates
                    snap = snap_rx.recv() => {
                        if let Ok(snap) = snap {
                            if let Ok(msg) = serde_json::to_string(&snap) {
                                if sender.send(Message::Text(msg)).await.is_err() {
                                    break;
                                }
                            }
                        } else {
                            break;
                        }
                    },
                    // Handle sending new match struct end of each turn
                    timer = timer_rx.recv() => {
                        if let Ok(timer) = timer {
                            if let Ok(msg) = serde_json::to_string(&timer) {
                                if sender.send(Message::Text(msg)).await.is_err() {
                                    break;
                                }
                            }
                        } else {
                            break;
                        }
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
                    // If the message is a valid increment request then we increment and broadcast updates to clients
                    // We also check that the current connection is on the team that is allowed to make the move
                    let current_team = state.teams.current_team.load();

                    if let Ok(request) = serde_json::from_str::<IncrementRequest>(&text) {
                        if request.section < 9
                            && request.cell < 9
                            && current_team == team_connection.team
                        {
                            state.snapshot.increment(request.section, request.cell);
                            needs_broadcast = true;

                            // If enough time has passed since last broadcast then
                            // we send the most updated state. This is a primitive form
                            // of rate limiting and stops a client from spamming everyone.
                            if needs_broadcast
                                && last_broadcast.elapsed() > Duration::from_millis(100)
                            {
                                let new_snap = state.snapshot.load();
                                let _ = state.snap_tx.send(SnapshotResponse {
                                    snap: new_snap,
                                    current_team: state.teams.current_team.load(),
                                    your_team: None,
                                });
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
                let _ = state.snap_tx.send(SnapshotResponse {
                    snap: new_snap,
                    current_team: state.teams.current_team.load(),
                    your_team: None,
                });
            }
        })
    };

    // Wait for either task to finish and then cleanup
    tokio::select! {
        _ = &mut send_task => receive_task.abort(),
        _ = &mut receive_task => send_task.abort(),
    };

    // Cleanup connection on disconnect
    state.teams.remove_connection(&team_connection);
    let (team_x_len, team_o_len) = state.teams.team_lens();

    // Log connection close and decrement connection count
    tracing::info!(
        "Connection closed. Remaining connections: {}",
        team_x_len + team_o_len
    );
}

async fn send_match_updates(state: Arc<AppState>) -> Result<()> {
    // Get current match data
    let match_schema = state.match_schema.load();

    if let Some(coords) = state.snapshot.find_max_indices() {
        let board = match_schema.board.get_updated(coords)?;
        let updated_match =
            crud_update_match(&state.db, match_schema.id, board.status, board).await?;
        let updated_match_schema = MatchSchema::try_from(&updated_match)?;

        // Send to all connected clients
        if state.timer_tx.send(updated_match_schema).is_ok() {
            // Reset snapshot state
            state.teams.set(updated_match_schema.board.current_team);
            state.snapshot.reset();
        } else {
            bail!("Unable to send match updates to clients");
        }
    }

    Ok(())
}

pub async fn run_match_updates(state: Arc<AppState>) {
    tracing::info!("Starting run match updates");

    let mut interval = tokio::time::interval(Duration::from_secs(10));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if state.snapshot.is_empty() {
                    tracing::warn!("Snapshot is empty, not starting match updates");
                    continue;
                }

                let (team_x_len, team_o_len) = state.teams.team_lens();
                if team_x_len == 0 || team_o_len == 0 {
                    tracing::warn!("Not enough players to start match updates");
                    continue;
                }

                tracing::info!("Conditions met, starting match updates");

                if let Err(e) = send_match_updates(state.clone()).await {
                    tracing::error!("Error sending match updates: {:?}", e);
                    continue;
                }
            }
        }
    }
}
