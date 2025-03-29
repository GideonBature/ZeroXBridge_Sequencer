use axum::{extract::State, Router};
use sqlx::PgPool;
use std::sync::Arc;
use dotenvy::dotenv;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use hyper::Server;

mod config;
mod api;

use api::routes::withdrawal_routes;
use config::get_db_pool;

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let pool = get_db_pool()
        .await
        .expect("Failed to connect to database");

    // Run database migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    let state = Arc::new(pool);

    let app = create_router(state.clone());

    let addr = "0.0.0.0:3000".parse().unwrap();

    println!("🚀 Listening on {}", addr);

    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn create_router(pool: Arc<PgPool>) -> Router {
    Router::new()
        .merge(withdrawal_routes())
        .with_state(pool)
        .layer(TraceLayer::new_for_http())
}