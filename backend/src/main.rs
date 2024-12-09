mod crud;
mod error;
mod handler;
mod model;
mod schema;

use axum::{
    http::{header::CONTENT_TYPE, Method},
    routing::{any, get, post},
    Router,
};
use crossbeam::atomic::AtomicCell;
use crud::{crud_create_match, crud_get_latest_match};
use handler::{
    commit_match_from_snapshot_handler, create_match_handler, get_latest_match_handler,
    get_match_by_id_handler, get_matches_handler, get_snapshot_handler, handle_websocket,
    reset_match_board_handler, run_match_updates, update_snapshot_handler,
};
use schema::{MatchSchema, Snapshot, SnapshotResponse, Teams, TeamsResponse};
use sqlx::postgres;
use tokio::{net::TcpListener, sync::broadcast};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::{DefaultOnResponse, TraceLayer},
};
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use std::sync::atomic::AtomicBool;
use std::{sync::Arc, time::Duration}; // Add this import

pub struct AppState {
    db: postgres::PgPool,
    snapshot: Snapshot,
    match_schema: AtomicCell<MatchSchema>,
    snap_tx: broadcast::Sender<SnapshotResponse>,
    teams_tx: broadcast::Sender<TeamsResponse>,
    match_tx: broadcast::Sender<MatchSchema>,
    teams: Teams,
    is_paused: AtomicBool,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_connection_str = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // set up connection pool
    let pool = postgres::PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&db_connection_str)
        .await
        .expect("can't connect to database");

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any)
        .allow_headers([CONTENT_TYPE]);

    let trace_layer =
        TraceLayer::new_for_http().on_response(DefaultOnResponse::new().level(Level::INFO));

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let (snap_tx, _snap_rx) = broadcast::channel(100);
    let (teams_tx, _teams_rx) = broadcast::channel(100);
    let (match_tx, _match_rx) = broadcast::channel(4096);

    // Get the latest match state or create a new one
    let match_schema = match crud_get_latest_match(&pool).await {
        Ok(model) => MatchSchema::try_from(&model)
            .map_err(|e| anyhow::anyhow!("Failed to convert model to schema: {}", e))
            .unwrap(),
        Err(_) => {
            let new_match = crud_create_match(&pool)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create new match: {}", e))
                .unwrap();

            MatchSchema::try_from(&new_match)
                .map_err(|e| anyhow::anyhow!("Failed to convert new match to schema: {}", e))
                .unwrap()
        }
    };

    let state = Arc::new(AppState {
        db: pool.clone(),
        snapshot: Snapshot::new(),
        match_schema: AtomicCell::new(match_schema),
        snap_tx: snap_tx.clone(),
        teams_tx: teams_tx.clone(),
        match_tx: match_tx.clone(),
        teams: Teams::new(),
        is_paused: AtomicBool::new(true), // Changed from AtomicCell to AtomicBool
    });

    // Spawn the global 10 second timer.
    let update_state = state.clone();
    tokio::spawn(async move {
        run_match_updates(update_state).await;
    });

    // Build our application with some routes
    let app = Router::new()
        .route("/ws", any(handle_websocket))
        .route("/api/matches/latest", get(get_latest_match_handler))
        .route(
            "/api/matches",
            get(get_matches_handler).post(create_match_handler),
        )
        .route(
            "/api/matches/:match_id",
            get(get_match_by_id_handler).post(commit_match_from_snapshot_handler),
        )
        .route(
            "/api/matches/:match_id/reset",
            post(reset_match_board_handler),
        )
        .route(
            "/api/snapshot",
            get(get_snapshot_handler).put(update_snapshot_handler),
        )
        .layer(cors)
        .layer(trace_layer)
        .with_state(state);

    // run it with hyper
    let listener = TcpListener::bind("127.0.0.1:8000").await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
