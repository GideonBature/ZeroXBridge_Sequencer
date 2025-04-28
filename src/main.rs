use axum::{routing::get, Router};
// use ethers::prelude::*;
use std::sync::Arc;
use std::{net::SocketAddr, path::Path};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
// use zeroxbridge_sequencer::oracle_service::oracle_service::sync_tvl;

mod config;
mod db;

use config::load_config;
use db::client::DBClient;
use zeroxbridge_sequencer::api::routes::create_router;

#[tokio::main]
async fn main() {
    let config = load_config(Some(Path::new("config.toml"))).expect("Failed to load config");

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db = DBClient::new(&config)
        .await
        .expect("Failed to connect to DB");

    db.run_migrations().await.expect("Failed to run migrations");

    let shared_db = Arc::new(db);
    let app = Router::new()
        .route("/", get(handler))
        .merge(create_router(shared_db.pool.clone()));

    let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
    println!("ZeroXBridge Sequencer listening on {}", addr);

    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}

async fn handler() -> &'static str {
    "Welcome to ZeroXBridge Sequencer"
}
