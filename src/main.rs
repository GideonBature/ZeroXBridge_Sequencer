// use axum::{routing::get, Router};
// use sqlx::postgres::PgPoolOptions;
// use std::sync::Arc;
// use tower_http::trace::TraceLayer;
// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
// use dotenvy::dotenv;

// mod config;
// mod api;

// use api::routes::{configure_routes, AppState};

// #[tokio::main]
// async fn main() {
//     dotenv().ok();

//     tracing_subscriber::registry()
//         .with(tracing_subscriber::EnvFilter::new(
//             std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
//         ))
//         .with(tracing_subscriber::fmt::layer())
//         .init();

//     let database_url = std::env::var("DATABASE_URL")
//         .expect("DATABASE_URL must be set");

//     let pool = PgPoolOptions::new()
//         .max_connections(10)
//         .connect(&database_url)
//         .await
//         .expect("Failed to connect to database");

//     let state = Arc::new(AppState { db: pool });

//     let app = Router::new()
//         .route("/", get(handler))
//         .route("/health", get(|| async { axum::http::StatusCode::OK }))
//         .nest("/withdraw", configure_routes())
//         .with_state(state.clone())        
//         .layer(TraceLayer::new_for_http());

//     let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
//         .await
//         .expect("Failed to bind to address");

//     println!("🚀 Listening on {}", listener.local_addr().unwrap());

//     axum::serve(listener, app)
//         .await
//         .expect("Server failed");
// }

// async fn handler() -> &'static str {
//     "Welcome to ZeroXBridge Sequencer"
// }
use axum::Router;
use sqlx::PgPool;
use std::sync::Arc;
use crate::api::routes::withdrawal_routes;
use hyper::Server;
use std::env;
use sqlx::{Pool, Postgres, PgPool};

mod api;

#[tokio::main]
async fn main() {
    let pool = Arc::new(PgPool::connect(&env::var("DATABASE_URL").unwrap()).await.unwrap());
    let app = create_router(pool);

    let database_url = "postgresql://zeroxbridge_owner:npg_DJL5WhK3PlBM@ep-royal-art-a5o74xlv-pooler.us-east-2.aws.neon.tech/zeroxbridge?sslmode=require";
    // let pool = PgPool::connect(database_url).await.unwrap();
    let addr = "0.0.0.0:3000".parse().unwrap();
    let server = Server::try_bind(&addr);

    let state = Arc::new(pool);

    let app = Router::new()
        .merge(withdrawal_routes())
        .with_state(state);

        Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
        
        
        pub fn create_router(pool: PgPool) -> Router {
            Router::new()
            // .merge(withdrawal_routes(pool))
            .merge(withdrawal_routes(pool.clone()))
        }
        
}


    

