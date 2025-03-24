use axum::{routing::get, Router};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use dotenvy::dotenv;
use axum::http::StatusCode;
use crate::api::routes::configure_routes;



mod config;
mod api;

use api::routes::AppState;

#[tokio::main]
async fn main() {
    
    dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    let state = Arc::new(AppState { db: pool });

    let app = Router::new()
        .route("/", get(handler))
        .route("/health", get(|| async { StatusCode::OK }))
        .nest("/withdraw", api::routes::configure_routes())
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("Failed to bind to address");

    println!("🚀 Listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}

pub fn configure_routes() -> Router {
    Router::new()
}


async fn handler() -> &'static str {
    "Welcome to ZeroXBridge Sequencer"
}

