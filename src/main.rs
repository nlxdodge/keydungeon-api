use sqlx::{Error, SqlitePool};
use sqlx::pool::PoolOptions;
use std::env;
use std::net::SocketAddr;

use crate::models::app_state::AppState;
use crate::repositories::event_repository::EventRepository;
use crate::repositories::password_repository::PasswordRepository;

mod api;
mod handlers;
mod models;
mod repositories;
mod router;
mod helpers;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    dotenvy::dotenv().ok();

    let port = env::var("PORT").unwrap_or_else(|_| "4321".to_string());
    let pool = get_pool().await?;
    let state: AppState = AppState {
        db: pool.clone(),
        events: EventRepository { pool: pool.clone() },
        passwords: PasswordRepository { pool: pool.clone() },
    };
    let app = router::create_router(state);

    sqlx::query("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;").execute(&pool).await?;

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    // "Started KeyDungeon api on port {port}"
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

async fn get_pool() -> Result<SqlitePool, Error> {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost/test".to_string());

    // "Connecting to database: {}", database_url

    PoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
}
