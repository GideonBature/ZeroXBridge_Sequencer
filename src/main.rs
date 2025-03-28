use axum::{routing::get, Extension, Router};
use std::{net::SocketAddr, path::Path, sync::Arc};

mod config;
mod db;

use config::load_config;
use db::client::DBClient;

#[tokio::main]
async fn main() {
    let config = load_config(Some(Path::new("config.toml"))).expect("Failed to load config");

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
