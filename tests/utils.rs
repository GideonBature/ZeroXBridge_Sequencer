use sqlx::postgres::PgPoolOptions;
use std::{path::Path, sync::Arc};
use zeroxbridge_sequencer::api::routes::AppState;
use zeroxbridge_sequencer::config;

pub async fn create_test_app() -> Arc<AppState> {
    let configuration = config::load_config(Some(&Path::new("./config-tests.toml"))).unwrap();
    let database_url = configuration.database.get_db_url();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    let state = Arc::new(AppState {
        db: pool.clone(),
        config: configuration.clone(),
    });

    state
}
