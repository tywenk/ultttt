use std::sync::atomic::Ordering;
use std::sync::Arc; // Add this import

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
use crate::crud::crud_get_latest_match;
use crate::crud::crud_get_match;
use crate::crud::crud_get_matches;
use crate::crud::crud_update_match;
use crate::error::AppError;
use crate::schema::Board;
use crate::schema::Coords;
use crate::schema::IncrementRequest;
use crate::schema::MatchSchema;
use crate::schema::SnapshotResponse;
use crate::schema::Status;
use crate::schema::TeamsResponse;
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

pub async fn reset_match_board_handler(
    Path(match_id): Path<Uuid>,
    State(data): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let board = Board::new();
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

    // Subscribe to match broadcast channel
    // This allows the connection to receive match updates
    let mut match_rx = state.match_tx.subscribe();

    // Subscribe to team broadcast channel
    // This allows the connection to receive team size updates
    let mut teams_rx = state.teams_tx.subscribe();

    let (team_x_len, team_o_len) = state.teams.team_lens();
    tracing::info!(
        "New connection (Team {:?}). Total connections: {}",
        team_connection.team,
        team_x_len + team_o_len
    );

    // Update the paused state on new connection
    let enough_players = team_x_len > 0 && team_o_len > 0;
    state.is_paused.store(!enough_players, Ordering::SeqCst);

    // Split the socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Broadcast the most recent snapshot to this client
    // This lets them know what team they are on
    let initial_snap = state.snapshot.load();
    if let Ok(initial_response) = serde_json::to_string(&SnapshotResponse {
        snap: initial_snap,
        your_team: Some(team_connection.team),
    }) {
        if sender.send(Message::Text(initial_response)).await.is_err() {
            tracing::error!("Unable to send initial snapshot to client");
        }
    }

    // Also blast clients with new team sizes
    let _ = state.teams_tx.send(TeamsResponse {
        x_team_size: team_x_len,
        o_team_size: team_o_len,
    });

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
                    match_schema = match_rx.recv() => {
                        if let Ok(match_schema) = match_schema {
                            if let Ok(msg) = serde_json::to_string(&match_schema) {
                                if sender.send(Message::Text(msg)).await.is_err() {
                                    break;
                                }
                            }
                        } else {
                            break;
                        }
                    }
                    // Handle sending team size updates
                    teams = teams_rx.recv() => {
                        if let Ok(teams) = teams {
                            if let Ok(msg) = serde_json::to_string(&teams) {
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
                    tracing::info!(
                        "Incrementing for team: {:?} {:?}",
                        team_connection.team,
                        &text
                    );
                    // If the message is a valid increment request then we increment and broadcast updates to clients
                    // We also check that the current connection is on the team that is allowed to make the move
                    if let Ok(request) = serde_json::from_str::<IncrementRequest>(&text) {
                        match state.snapshot.validate_move(
                            state.clone(),
                            request.section,
                            request.cell,
                            team_connection.team,
                        ) {
                            Ok(_) => {
                                tracing::debug!(
                                    "Incrementing cell: {:?} for team: {:?}",
                                    request,
                                    team_connection.team
                                );
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
                                        your_team: None,
                                    });
                                    needs_broadcast = false;
                                    last_broadcast = Instant::now();
                                }
                            }
                            Err(e) => {
                                tracing::error!("Invalid move: {:?}", e);
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

    // Update pause state based on team sizes
    let enough_players = team_x_len > 0 && team_o_len > 0;
    state.is_paused.store(!enough_players, Ordering::SeqCst);

    // Send out updated team sizes
    let _ = state.teams_tx.send(TeamsResponse {
        x_team_size: team_x_len,
        o_team_size: team_o_len,
    });

    // Log connection close and decrement connection count
    tracing::info!(
        "Connection closed. Remaining connections: {}",
        team_x_len + team_o_len
    );
}

async fn send_match_updates(state: Arc<AppState>) -> Result<Status> {
    // Get current match data
    let match_schema = state.match_schema.load();

    tracing::info!(
        "Sending match updates. Snapshot: {:?}",
        state.snapshot.load()
    );

    // Only update match if snapshot is not empty
    if let Some(coords) = state.snapshot.find_max_indices() {
        let board = match_schema.board.get_updated(coords)?;
        let updated_match =
            crud_update_match(&state.db, match_schema.id, board.status, board).await?;
        let updated_match_schema = MatchSchema::try_from(&updated_match)?;

        // Send to all connected clients and update state
        state
            .match_tx
            .send(updated_match_schema)
            .map_err(|_| anyhow::anyhow!("Failed to send match updates to clients"))?;

        // Reset snapshot state
        state.snapshot.reset();
        state.match_schema.store(updated_match_schema);

        state
            .snap_tx
            .send(SnapshotResponse {
                snap: state.snapshot.load(),
                your_team: None,
            })
            .map_err(|_| anyhow::anyhow!("Failed to send snapshot response"))?;

        return Ok(updated_match_schema.board.status);
    }

    Ok(Status::Pending)
}

async fn create_and_send_new_match(state: Arc<AppState>) -> Result<()> {
    let new_match = crud_create_match(&state.db).await?;
    let new_match_schema = MatchSchema::try_from(&new_match)?;
    state.match_schema.store(new_match_schema);
    state
        .match_tx
        .send(new_match_schema)
        .map_err(|_| anyhow::anyhow!("Failed to send match updates to clients"))?;

    Ok(())
}

pub async fn run_match_updates(state: Arc<AppState>) -> Result<()> {
    tracing::info!("Starting run match updates");

    loop {
        // Check conditions first
        let (team_x_len, team_o_len) = state.teams.team_lens();
        tracing::debug!("Team sizes: X: {}, O: {}", team_x_len, team_o_len);
        if team_x_len == 0 || team_o_len == 0 {
            state.is_paused.store(true, Ordering::SeqCst);
            tracing::warn!("Not enough players, game paused");
            // Sleep for a shorter duration when waiting for players
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        }

        if state.is_paused.load(Ordering::SeqCst) {
            tracing::info!("Game is paused.");
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        }

        if state.snapshot.is_empty() {
            tracing::warn!("Snapshot is empty, not starting match updates");
            tokio::time::sleep(Duration::from_secs(4)).await;
            continue;
        }

        // Only sleep the full interval when actually running updates
        tracing::debug!("Conditions met, starting match updates");
        match send_match_updates(state.clone()).await {
            Ok(status) => {
                if status.is_complete() {
                    tracing::info!("Match is complete, pausing game for 20 seconds");
                    state.is_paused.store(true, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_secs(20)).await;
                    create_and_send_new_match(state.clone()).await?;
                    state.is_paused.store(false, Ordering::SeqCst);
                }
            }
            Err(e) => {
                tracing::error!("Error sending match updates: {:?}", e);
            }
        }
        tokio::time::sleep(Duration::from_secs(12)).await;
    }
}
