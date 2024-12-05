mod crud;
mod error;
mod handler;
mod model;
mod route;
mod schema;

use axum::{
    http::{header::CONTENT_TYPE, Method},
    routing::{any, get},
    Router,
};
use handler::{
    commit_match_from_snapshot_handler, create_match_handler, get_match_by_id_handler,
    get_matches_handler, get_snapshot_handler, handle_websocket, update_snapshot_handler,
};
use schema::Snapshot;
use sqlx::postgres;
use tokio::net::TcpListener;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::{DefaultOnResponse, TraceLayer},
};
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use std::{sync::Arc, time::Duration};

pub struct AppState {
    db: postgres::PgPool,
    snapshot: Snapshot,
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

    // build our application with some routes
    let app = Router::new()
        .route("/ws", any(handle_websocket))
        .route(
            "/api/matches",
            get(get_matches_handler).post(create_match_handler),
        )
        .route(
            "/api/matches/:match_id",
            get(get_match_by_id_handler).post(commit_match_from_snapshot_handler),
        )
        .route(
            "/api/snapshot",
            get(get_snapshot_handler).put(update_snapshot_handler),
        )
        .layer(cors)
        .layer(trace_layer)
        .with_state(Arc::new(AppState {
            db: pool.clone(),
            snapshot: Snapshot::new(),
        }));

    // run it with hyper
    let listener = TcpListener::bind("127.0.0.1:8000").await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
